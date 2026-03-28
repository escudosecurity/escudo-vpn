#!/usr/bin/env bash
set -euo pipefail

OUT="${1:-/home/dev/pulsovpn/escudo-vpn/scripts/load/dnsperf-queries.txt}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/legit.txt" <<'EOF'
google.com
youtube.com
netflix.com
amazon.com
facebook.com
instagram.com
cloudflare.com
openai.com
apple.com
microsoft.com
linkedin.com
reddit.com
wikipedia.org
whatsapp.com
x.com
tiktok.com
spotify.com
discord.com
twitch.tv
github.com
gitlab.com
stackexchange.com
stackoverflow.com
paypal.com
ebay.com
booking.com
airbnb.com
zoom.us
office.com
live.com
outlook.com
bing.com
yahoo.com
cnn.com
bbc.com
bbc.co.uk
theguardian.com
nytimes.com
globo.com
globoplay.globo.com
g1.globo.com
uol.com.br
terra.com.br
gov.br
itau.com.br
bradesco.com.br
nubank.com.br
mercadolivre.com.br
mercadopago.com.br
americanas.com.br
magazineluiza.com.br
casasbahia.com.br
aliexpress.com
temu.com
shein.com
adobe.com
dropbox.com
slack.com
notion.so
figma.com
canva.com
digitalocean.com
vultr.com
hetzner.com
oracle.com
salesforce.com
mozilla.org
ubuntu.com
debian.org
docker.com
kubernetes.io
npmjs.com
pypi.org
rust-lang.org
crates.io
hulu.com
peacocktv.com
paramountplus.com
disneyplus.com
bamgrid.com
nflxvideo.net
nflximg.net
googlevideo.com
ytimg.com
akamaihd.net
fastly.net
cloudfront.net
skype.com
telegram.org
signal.org
proton.me
protonmail.com
surfshark.com
nordvpn.com
protonvpn.com
1.1.1.1
speed.cloudflare.com
ipinfo.io
dnsleaktest.com
browserleaks.com
archive.org
imdb.com
weather.com
espn.com
steamcommunity.com
steampowered.com
roblox.com
epicgames.com
riotgames.com
playstation.com
xbox.com
ea.com
ubisoft.com
gov.uk
service.gov.uk
co.uk
co.jp
co.in
com.br
olx.com.br
ifood.com.br
99app.com
uber.com
lyft.com
doordash.com
ubereats.com
linkedincdn.com
gstatic.com
googleapis.com
doubleclick.net
windowsupdate.com
download.windowsupdate.com
office365.com
sharepoint.com
onedrive.com
icloud.com
fast.com
primevideo.com
max.com
hbomax.com
deezer.com
soundcloud.com
deepl.com
translate.google.com
maps.google.com
photos.google.com
drive.google.com
news.google.com
finance.yahoo.com
duckduckgo.com
perplexity.ai
anthropic.com
claude.ai
stripe.com
wise.com
remessaonline.com.br
inter.com.br
caixa.gov.br
banco.bradesco
santander.com.br
banrisul.com.br
bancodobrasil.com.br
web.whatsapp.com
mail.google.com
calendar.google.com
meet.google.com
teams.microsoft.com
login.microsoftonline.com
amazonaws.com
azureedge.net
akamaized.net
quartzdns.com
digitaloceanspaces.com
linode.com
ovhcloud.com
france24.com
lemonde.fr
elmundo.es
corriere.it
spiegel.de
bild.de
nhk.or.jp
rakuten.co.jp
naver.com
daum.net
baidu.com
jd.com
taobao.com
tokopedia.com
traveloka.com
grab.com
gojek.com
canalplus.com
rte.ie
rtve.es
cbc.ca
abc.net.au
stuff.co.nz
folha.uol.com.br
estadao.com.br
oglobo.globo.com
metropoles.com
infomoney.com.br
valor.globo.com
techtudo.com.br
ge.globo.com
cartola.globo.com
globonews.globo.com
receita.fazenda.gov.br
meu.inss.gov.br
enel.com.br
vivo.com.br
tim.com.br
claro.com.br
oi.com.br
EOF

cat > "$TMP/ad-blocked.txt" <<'EOF'
googleads.g.doubleclick.net
adnxs.com
acdn.adnxs.com
ams1-ib.adnxs.com
ams3-ib.adnxs.com
anycast.adnxs.com
ib.anycast.adnxs.com
aax-eu-amazon-adsystem.com
doubleclick.net
adservice.google.com
ads.yahoo.com
adservice.google.com.br
pagead2.googlesyndication.com
partnerad.l.doubleclick.net
securepubads.g.doubleclick.net
adserver.snapads.com
ads-twitter.com
ads.linkedin.com
analytics.twitter.com
ads.facebook.com
adclick.g.doubleclick.net
ad.doubleclick.net
pixel.facebook.com
ads.youtube.com
ad-delivery.net
adservice.google.co.uk
EOF

cat > "$TMP/phish-blocked.txt" <<'EOF'
00-coopb144.com
000054343-info.weebly.com
0002985647.weebly.com
00065455.weebly.com
000outloook3665maail.weebly.com
verify-account-wixsite-login.example
secure-microsoft-login.weebly.com
paypal-confirm-account.wixsite.com
banco-seguro-update.blogpost.com
nubank-verificacao.000webhostapp.com
EOF

fetch_source() {
  local url="$1"
  local out="$2"
  curl -fsSL --max-time 30 "$url" -o "$out" || return 1
}

extract_domains() {
  sed 's/\r$//' "$1" \
    | sed 's/#.*$//' \
    | awk '
      NF==0 { next }
      $1=="0.0.0.0" || $1=="127.0.0.1" { print $2; next }
      $1 ~ /^[A-Za-z0-9._-]+\.[A-Za-z][A-Za-z.-]+$/ { print $1; next }
    ' \
    | tr '[:upper:]' '[:lower:]' \
    | sed 's/\.$//' \
    | grep -E '^[a-z0-9._-]+\.[a-z][a-z.-]+$' \
    | grep -vE '^(localhost|localdomain)$'
}

touch "$TMP/fetched-blocked.txt"
fetch_source "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/domains/multi.txt" "$TMP/hagezi.txt" || true
fetch_source "https://urlhaus.abuse.ch/downloads/hostfile/" "$TMP/urlhaus.txt" || true
fetch_source "https://malware-filter.gitlab.io/malware-filter/phishing-filter-domains.txt" "$TMP/phishing.txt" || true
fetch_source "https://raw.githubusercontent.com/PeterDaveHello/ip_blacklist/master/block_hostfile.txt" "$TMP/threat.txt" || true

for f in "$TMP"/hagezi.txt "$TMP"/urlhaus.txt "$TMP"/phishing.txt "$TMP"/threat.txt; do
  [[ -s "$f" ]] || continue
  extract_domains "$f" >> "$TMP/fetched-blocked.txt" || true
done

cat "$TMP/legit.txt" > "$TMP/all-legit.txt"
for prefix in www m api cdn static img media edge login auth app mobile news sport live video assets content; do
  awk -v p="$prefix" '{print p "." $0}' "$TMP/legit.txt" >> "$TMP/all-legit.txt"
done

{
  sort -u "$TMP/all-legit.txt"
  sort -u "$TMP/ad-blocked.txt"
  sort -u "$TMP/phish-blocked.txt"
  sort -u "$TMP/fetched-blocked.txt" | sed -n '1,1500p'
} \
  | awk 'length($0) > 0 {print $0 " A"}' \
  | awk '!seen[$0]++' \
  > "$OUT"

echo "wrote=$(wc -l < "$OUT") file=$OUT"
