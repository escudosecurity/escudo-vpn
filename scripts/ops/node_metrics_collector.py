#!/usr/bin/env python3

import base64
import csv
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request


CONFIG_PATH = pathlib.Path("/etc/escudo/api.toml")
STATE_PATH = pathlib.Path("/var/lib/escudo/node_metrics_collector_state.json")
SCRAPE_TIMEOUT_SECONDS = 3.0
SSH_TIMEOUT_SECONDS = 5


def load_database_url() -> str:
    content = CONFIG_PATH.read_text(encoding="utf-8")
    match = re.search(r'^\s*url\s*=\s*"([^"]+)"\s*$', content, re.MULTILINE)
    if not match:
        raise RuntimeError(f"database url not found in {CONFIG_PATH}")
    return match.group(1)


def run_psql_query(database_url: str, sql: str) -> list[dict[str, str]]:
    cmd = [
        "psql",
        database_url,
        "-X",
        "-A",
        "-F",
        "\t",
        "-P",
        "footer=off",
        "-c",
        sql,
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    if not lines:
        return []
    reader = csv.DictReader(lines, delimiter="\t")
    return list(reader)


def run_psql_file(database_url: str, sql_text: str) -> None:
    with tempfile.NamedTemporaryFile("w", delete=False, encoding="utf-8") as handle:
        handle.write(sql_text)
        temp_path = handle.name
    try:
        subprocess.run(
            ["psql", database_url, "-X", "-v", "ON_ERROR_STOP=1", "-f", temp_path],
            capture_output=True,
            text=True,
            check=True,
        )
    finally:
        os.unlink(temp_path)


def load_state() -> dict:
    if not STATE_PATH.exists():
        return {}
    try:
        return json.loads(STATE_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}


def save_state(state: dict) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    temp_path = STATE_PATH.with_suffix(".tmp")
    temp_path.write_text(json.dumps(state, indent=2, sort_keys=True), encoding="utf-8")
    temp_path.replace(STATE_PATH)


def scrape_metrics(public_ip: str) -> tuple[dict[str, float], float | None, str | None]:
    url = f"http://{public_ip}:8080/metrics"
    request = urllib.request.Request(url, headers={"User-Agent": "escudo-node-metrics-collector/1.0"})
    start = time.monotonic()
    try:
        with urllib.request.urlopen(request, timeout=SCRAPE_TIMEOUT_SECONDS) as response:
            body = response.read().decode("utf-8", errors="replace")
        latency_ms = (time.monotonic() - start) * 1000.0
        metrics = parse_prometheus_text(body)
        return metrics, latency_ms, None
    except urllib.error.URLError as exc:
        return {}, None, str(exc.reason)
    except TimeoutError:
        return {}, None, "timeout"
    except Exception as exc:  # noqa: BLE001
        return {}, None, str(exc)


def ssh_collect(public_ip: str) -> tuple[dict[str, str], float | None, str | None]:
    command = [
        "ssh",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        f"ConnectTimeout={SSH_TIMEOUT_SECONDS}",
        f"root@{public_ip}",
        (
            "set -e;"
            " printf 'gateway=%s\\n' \"$(systemctl is-active escudo-gateway 2>/dev/null || echo missing)\";"
            " printf 'metrics_b64=%s\\n' \"$(curl -m 3 -s http://127.0.0.1:8080/metrics | base64 -w0 || true)\";"
            " printf 'cpu=%s\\n' \"$(awk '/^cpu /{print $2\" \" $3\" \" $4\" \" $5\" \" $6\" \" $7\" \" $8\" \" $9; exit}' /proc/stat)\";"
            " printf 'mem=%s\\n' \"$(awk '/MemTotal:/{t=$2} /MemAvailable:/{a=$2} END{printf \"%s %s\", t, a}' /proc/meminfo)\";"
        ),
    ]
    start = time.monotonic()
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=SSH_TIMEOUT_SECONDS + 3, check=True)
        latency_ms = (time.monotonic() - start) * 1000.0
        parsed: dict[str, str] = {}
        for line in result.stdout.splitlines():
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            parsed[key.strip()] = value.strip()
        return parsed, latency_ms, None
    except subprocess.TimeoutExpired:
        return {}, None, "ssh timeout"
    except subprocess.CalledProcessError as exc:
        error = (exc.stderr or exc.stdout or str(exc)).strip().splitlines()[-1]
        return {}, None, error


def parse_prometheus_text(body: str) -> dict[str, float]:
    metrics: dict[str, float] = {}
    for raw_line in body.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) != 2:
            continue
        name, value = parts
        try:
            metrics[name] = float(value)
        except ValueError:
            continue
    return metrics


def sql_literal(value) -> str:
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "TRUE" if value else "FALSE"
    if isinstance(value, (int, float)):
        return str(value)
    return "'" + str(value).replace("'", "''") + "'"


def lifecycle_for_score(score: int) -> str:
    if score >= 85:
        return "healthy"
    if score >= 70:
        return "warm"
    if score >= 50:
        return "degraded"
    return "blocked"


def cpu_pct_from_state(previous_cpu: list[float] | None, current_cpu: list[float]) -> float:
    if not previous_cpu or len(previous_cpu) != len(current_cpu):
        return 0.0
    delta = [max(curr - prev, 0.0) for prev, curr in zip(previous_cpu, current_cpu)]
    total = sum(delta)
    if total <= 0:
        return 0.0
    idle = delta[3] + (delta[4] if len(delta) > 4 else 0.0)
    busy = max(total - idle, 0.0)
    return (busy / total) * 100.0


def collect() -> int:
    database_url = load_database_url()
    servers = run_psql_query(
        database_url,
        """
        SELECT
            s.id,
            s.name,
            s.public_ip,
            s.assigned_user_cap,
            s.active_session_soft_cap,
            s.active_session_hard_cap
        FROM servers s
        WHERE s.is_active = true
        ORDER BY s.created_at ASC
        """,
    )
    assigned_rows = run_psql_query(
        database_url,
        """
        SELECT server_id, COUNT(*)::BIGINT AS assigned_users
        FROM devices
        WHERE is_active = true
        GROUP BY server_id
        """,
    )
    assigned_users = {row["server_id"]: int(row["assigned_users"]) for row in assigned_rows}
    previous_state = load_state()
    next_state: dict[str, dict[str, float]] = {}
    collected_at = time.strftime("%Y-%m-%d %H:%M:%S+00", time.gmtime())

    insert_values: list[str] = []
    update_statements: list[str] = []

    for server in servers:
        server_id = server["id"]
        public_ip = server["public_ip"]
        ssh_data, latency_ms, error_message = ssh_collect(public_ip)
        metrics: dict[str, float] = {}
        cpu_pct = 0.0
        ram_pct = 0.0

        if error_message is None:
            if ssh_data.get("gateway") != "active":
                error_message = f"gateway service state is {ssh_data.get('gateway', 'unknown')}"
            else:
                metrics_b64 = ssh_data.get("metrics_b64", "")
                if metrics_b64:
                    metrics = parse_prometheus_text(base64.b64decode(metrics_b64).decode("utf-8", errors="replace"))
                if not metrics:
                    metrics, _, public_error = scrape_metrics(public_ip)
                    if not metrics:
                        error_message = public_error or "gateway metrics unavailable"

                cpu_values = [float(value) for value in ssh_data.get("cpu", "").split() if value]
                mem_values = [float(value) for value in ssh_data.get("mem", "").split() if value]
                prev = previous_state.get(server_id, {})
                cpu_pct = cpu_pct_from_state(prev.get("cpu"), cpu_values)
                if len(mem_values) == 2 and mem_values[0] > 0:
                    mem_total, mem_available = mem_values
                    ram_pct = ((mem_total - mem_available) / mem_total) * 100.0
        else:
            prev = previous_state.get(server_id, {})

        active_sessions = int(metrics.get("escudo_active_peers", 0))
        rx_total = float(metrics.get("escudo_rx_bytes_total", 0.0))
        tx_total = float(metrics.get("escudo_tx_bytes_total", 0.0))
        assigned = assigned_users.get(server_id, 0)
        soft_cap = int(server["active_session_soft_cap"])
        hard_cap = int(server["active_session_hard_cap"])
        assigned_cap = int(server["assigned_user_cap"])

        nic_in_mbps = 0.0
        nic_out_mbps = 0.0
        prev = previous_state.get(server_id)
        now_ts = time.time()
        if prev:
            delta_seconds = max(now_ts - float(prev.get("ts", now_ts)), 1.0)
            delta_rx = max(rx_total - float(prev.get("rx", rx_total)), 0.0)
            delta_tx = max(tx_total - float(prev.get("tx", tx_total)), 0.0)
            nic_in_mbps = (delta_rx * 8.0) / delta_seconds / 1_000_000.0
            nic_out_mbps = (delta_tx * 8.0) / delta_seconds / 1_000_000.0
        next_state[server_id] = {
            "ts": now_ts,
            "rx": rx_total,
            "tx": tx_total,
            "cpu": [float(value) for value in ssh_data.get("cpu", "").split() if value],
        }

        connect_success_pct = 100.0 if error_message is None else 0.0
        median_connect_ms = int(latency_ms or 0.0)
        reasons: list[str] = []
        score = 100

        if error_message is not None:
            score = 25
            reasons.append(f"gateway metrics scrape failed: {error_message}")
        else:
            if median_connect_ms > 3000:
                score -= 20
                reasons.append("metrics latency above 3000ms")
            elif median_connect_ms > 1000:
                score -= 10
                reasons.append("metrics latency above 1000ms")

            if cpu_pct >= 90:
                score -= 20
                reasons.append("cpu above 90%")
            elif cpu_pct >= 80:
                score -= 10
                reasons.append("cpu above 80%")

            if ram_pct >= 90:
                score -= 20
                reasons.append("ram above 90%")
            elif ram_pct >= 85:
                score -= 10
                reasons.append("ram above 85%")

            if hard_cap > 0 and active_sessions >= hard_cap:
                score -= 35
                reasons.append("active sessions at or above hard cap")
            elif soft_cap > 0 and active_sessions >= soft_cap:
                score -= 15
                reasons.append("active sessions at or above soft cap")

            if assigned_cap > 0 and assigned >= assigned_cap:
                score -= 20
                reasons.append("assigned users at or above cap")

        score = max(0, min(100, score))
        lifecycle_state = lifecycle_for_score(score)

        insert_values.append(
            "("
            + ", ".join(
                [
                    sql_literal(server_id),
                    sql_literal(round(cpu_pct, 2)),
                    sql_literal(round(ram_pct, 2)),
                    sql_literal(round(nic_in_mbps, 3)),
                    sql_literal(round(nic_out_mbps, 3)),
                    sql_literal(active_sessions),
                    sql_literal(assigned),
                    sql_literal(round(connect_success_pct, 2)),
                    sql_literal(median_connect_ms),
                    sql_literal(score),
                    sql_literal(lifecycle_state),
                    sql_literal(collected_at),
                ]
            )
            + ")"
        )

        update_statements.append(
            f"""
            UPDATE servers
            SET health_score = {score},
                lifecycle_state = {sql_literal(lifecycle_state)},
                health_reasons = {sql_literal(json.dumps(reasons))}::jsonb,
                last_health_at = {sql_literal(collected_at)}::timestamptz,
                updated_at = NOW()
            WHERE id = {sql_literal(server_id)};
            """
        )

    if insert_values:
        sql_text = f"""
        BEGIN;
        INSERT INTO node_metrics (
            server_id, cpu_pct, ram_pct, nic_in_mbps, nic_out_mbps,
            active_sessions, assigned_users, connect_success_pct, median_connect_ms,
            health_score, health_state, collected_at
        ) VALUES
        {",\n".join(insert_values)};
        {"".join(update_statements)}
        COMMIT;
        """
        run_psql_file(database_url, sql_text)
        save_state(next_state)

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(collect())
    except subprocess.CalledProcessError as exc:
        sys.stderr.write(exc.stderr or str(exc))
        raise
