#!/usr/bin/env bash
set -euo pipefail

API_BASE="${API_BASE:-https://api.escudovpn.com}"
DB_URL="${DB_URL:-postgresql://escudo:escudo_secret@localhost/escudo}"
AUDIT_ROOT="${AUDIT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$AUDIT_ROOT/backend-control-plane-$TS"
mkdir -p "$OUT_DIR"

DIRECT_EMAIL="${DIRECT_EMAIL:-audit-direct@escudovpn.test}"
DIRECT_PASSWORD="${DIRECT_PASSWORD:-AuditPass123!}"
PRO_EMAIL="${PRO_EMAIL:-test@escudovpn.test}"
PRO_PASSWORD="${PRO_PASSWORD:-Test123!}"
DEDICATED_EMAIL="${DEDICATED_EMAIL:-ritafiol@private.com}"
DEDICATED_PASSWORD="${DEDICATED_PASSWORD:-Private01}"

log() {
  printf '%s %s\n' "$(date -u +%FT%TZ)" "$*"
}

api_register_if_missing() {
  local email="$1"
  local password="$2"
  local code
  code="$(curl -sS --max-time 20 -o /tmp/escudo-register.json -w '%{http_code}' \
    -X POST "$API_BASE/api/v1/auth/register" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$email\",\"password\":\"$password\"}" || true)"
  if [[ "$code" != "200" && "$code" != "400" && "$code" != "409" ]]; then
    log "register_unexpected_http email=$email http=$code"
  fi
}

ensure_active_tier() {
  local email="$1"
  local tier="$2"
  local plan="$3"
  local audit_tag
  local user_id
  user_id="$(psql "$DB_URL" -Atqc "select id from users where email = '$email' limit 1;")"
  if [[ -z "$user_id" ]]; then
    log "missing_user email=$email"
    exit 1
  fi
  audit_tag="audit-${tier}-${user_id}-${TS}"

  psql "$DB_URL" -v ON_ERROR_STOP=1 <<SQL >/dev/null
UPDATE subscriptions
SET status = 'canceled', updated_at = NOW()
WHERE user_id = '$user_id' AND status = 'active';

INSERT INTO subscriptions (
  id, user_id, plan, status, period_start, period_end,
  bandwidth_limit_bytes, created_at, updated_at, tier,
  stripe_customer_id, stripe_subscription_id
) VALUES (
  gen_random_uuid(), '$user_id', '$plan', 'active', NOW(), NOW() + INTERVAL '30 days',
  0, NOW(), NOW(), '$tier',
  '$audit_tag', '$audit_tag'
);
SQL
}

login_token() {
  local email="$1"
  local password="$2"
  curl -fsS --max-time 20 -X POST "$API_BASE/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$email\",\"password\":\"$password\"}" | jq -r '.token'
}

audit_tier() {
  local slug="$1"
  local email="$2"
  local password="$3"
  local token list_file result_file summary_file

  token="$(login_token "$email" "$password")"
  list_file="$OUT_DIR/${slug}-servers.json"
  result_file="$OUT_DIR/${slug}-connect.tsv"
  summary_file="$OUT_DIR/${slug}-summary.json"

  curl -fsS --max-time 20 "$API_BASE/api/v1/servers" \
    -H "Authorization: Bearer $token" > "$list_file"

  {
    echo -e "RESULT\tCOUNTRY\tNAME\tLOCATION\tENDPOINT\tDNS\tASSIGNED_IP\tPUBLIC_IP\tDEVICE_ID"
    echo -e "INFO\t-\tserver_count\t-\t$(jq 'length' < "$list_file")\t-\t-\t-\t-"
    jq -r '.[] | [.id,.name,.location,(.country_code // "")] | @tsv' < "$list_file" |
      while IFS=$'\t' read -r server_id name location country; do
        local safe_name device body resp http device_id cfg endpoint dns addr public_ip err
        safe_name="$(printf '%s' "$name" | tr ' ' '-' | tr -cd '[:alnum:]-_')"
        device="audit-${slug}-${safe_name}-$(date +%s%N | tail -c 7)"
        body="$(jq -nc --arg sid "$server_id" --arg dn "$device" '{server_id:$sid,device_name:$dn}')"
        resp="$(mktemp)"
        http="$(curl -sS --max-time 20 -o "$resp" -w '%{http_code}' \
          -X POST "$API_BASE/api/v1/connect" \
          -H "Authorization: Bearer $token" \
          -H 'Content-Type: application/json' \
          -d "$body" || true)"
        if [[ "$http" == "200" ]]; then
          device_id="$(jq -r '.device_id // empty' < "$resp")"
          cfg="$(jq -r '.config // empty' < "$resp")"
          endpoint="$(printf '%s\n' "$cfg" | sed -n 's/^Endpoint = //p' | head -n1)"
          dns="$(printf '%s\n' "$cfg" | sed -n 's/^DNS = //p' | head -n1)"
          addr="$(printf '%s\n' "$cfg" | sed -n 's/^Address = //p' | head -n1)"
          public_ip="$(jq -r '.public_ip // empty' < "$resp")"
          echo -e "OK\t$country\t$name\t$location\t$endpoint\t$dns\t$addr\t$public_ip\t$device_id"
          if [[ -n "$device_id" ]]; then
            curl -sS --max-time 20 -o /dev/null -X DELETE \
              "$API_BASE/api/v1/disconnect/$device_id" \
              -H "Authorization: Bearer $token" || true
          fi
        else
          err="$(tr '\n' ' ' < "$resp" | sed 's/[[:space:]]\+/ /g' | cut -c1-220)"
          echo -e "FAIL\t$country\t$name\t$location\tHTTP-$http\t$err\t-\t-\t-"
        fi
        rm -f "$resp"
      done
  } > "$result_file"

  jq -n \
    --arg tier "$slug" \
    --arg generated_at "$(date -u +%FT%TZ)" \
    --argjson visible "$(jq 'length' < "$list_file")" \
    --argjson ok "$(awk -F '\t' '$1=="OK"{c++} END{print c+0}' "$result_file")" \
    --argjson fail "$(awk -F '\t' '$1=="FAIL"{c++} END{print c+0}' "$result_file")" \
    '{
      tier: $tier,
      generated_at: $generated_at,
      visible_servers: $visible,
      ok_connects: $ok,
      failed_connects: $fail
    }' > "$summary_file"
}

log "ensuring direct audit account"
api_register_if_missing "$DIRECT_EMAIL" "$DIRECT_PASSWORD"
ensure_active_tier "$DIRECT_EMAIL" "escudo" "escudo"
ensure_active_tier "$PRO_EMAIL" "pro" "pro"
ensure_active_tier "$DEDICATED_EMAIL" "dedicated" "dedicated"

log "auditing escudo/direct"
audit_tier "escudo" "$DIRECT_EMAIL" "$DIRECT_PASSWORD"
log "auditing pro"
audit_tier "pro" "$PRO_EMAIL" "$PRO_PASSWORD"
log "auditing dedicated"
audit_tier "dedicated" "$DEDICATED_EMAIL" "$DEDICATED_PASSWORD"

(cd "$OUT_DIR" && sha256sum ./* > SHA256SUMS)

log "audit_complete out_dir=$OUT_DIR"
printf '%s\n' "$OUT_DIR"
