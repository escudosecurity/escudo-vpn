# Google Ads Automation Agent — Escudo VPN

## Overview

A browser automation agent using Playwright MCP routed through a Brazilian IPRoyal residential proxy that logs into Google Ads Manager (MCC), navigates to the Escudo VPN account, and creates three campaigns targeting Brazilian users. Total daily budget: R$35.

## Architecture

```
Claude Code
  └─ Playwright MCP (--proxy-server socks5://geo.iproyal.com:32325)
       └─ Chromium browser (Brazilian residential IP)
            └─ ads.google.com (Google Ads MCC → Escudo VPN account)
```

### Proxy Configuration

- **Protocol:** SOCKS5
- **Host:** geo.iproyal.com
- **Port:** 32325
- **Username format:** `{IPROYAL_API_TOKEN}__country-br__session-{uuid}__lifetime-240m`
- **Password:** `{IPROYAL_API_TOKEN}`
- **Session:** Sticky 4-hour session (240 minutes) to ensure login persists through full campaign setup

Credentials are embedded directly in the SOCKS5 URL, which Chromium supports natively:

```
socks5://USERNAME:PASSWORD@geo.iproyal.com:32325
```

Where USERNAME = `{IPROYAL_API_TOKEN}__country-br__session-{uuid}__lifetime-240m` (URL-encoded)

### MCP Launch Command

```
npx @playwright/mcp@latest \
  --proxy-server "socks5://ENCODED_USER:ENCODED_PASS@geo.iproyal.com:32325" \
  --browser chrome \
  --caps vision
```

The `--caps vision` flag enables screenshot-based fallback for identifying UI elements when accessibility snapshots fail on Google Ads' SPA.

### Prerequisites

- Google Ads MCC account exists and is accessible
- Escudo VPN sub-account exists within the MCC
- Billing is configured and active on the account
- escudovpn.com is live and serving pages without `.html` extensions (clean URLs)
- IPROYAL_API_TOKEN is set in `/home/dev/pulsovpn/escudo-vpn/.env`

## Login Flow

1. Agent navigates to `ads.google.com`
2. Agent enters user-provided email (user provides in chat)
3. Agent enters user-provided password (user provides in chat)
4. Agent pauses and asks user to complete OTP/2FA — waits for user confirmation before proceeding
5. Agent verifies login success by checking for the MCC dashboard
6. Agent navigates MCC → selects Escudo VPN account

**Credential handling:** User provides email and password directly in the chat when prompted. No credentials are stored in files. OTP is handled manually by the user (the agent pauses and waits for the user to say "done").

## Campaign Structure

### Campaign 1: Ferramentas Grátis (Free Tools) — R$15/day

**Goal:** Website traffic via high-volume utility keywords
**Strategy:** Cheap clicks on generic tool searches → user lands on Escudo tool page → sees branding → joins waitlist
**Bidding:** Maximize clicks
**Location:** Brazil only
**Language:** Portuguese

| Ad Group | Keywords | Match Type | Landing Page |
|----------|----------|------------|-------------|
| Teste de Velocidade | teste de velocidade, medir velocidade internet, speed test, teste de internet | Phrase | escudovpn.com/teste-de-velocidade |
| Meu IP | meu ip, qual meu ip, qual é meu ip, descobrir meu ip, meu endereço ip | Phrase | escudovpn.com/meu-ip |
| Verificar Senha | verificar senha vazada, minha senha foi vazada, senha vazada, senha hackeada | Phrase | escudovpn.com/verificar-senha |
| Verificar Link | verificar link seguro, link perigoso, site seguro, golpe whatsapp, verificar url | Phrase | escudovpn.com/verificar-link |

**Ad Copy (per ad group — 2 responsive search ads each):**

*Teste de Velocidade:*
- Headlines: "Teste de Velocidade Grátis", "Medir Velocidade da Internet", "Download, Upload e Ping", "Resultado em Segundos", "Compare com Sua Cidade"
- Descriptions: "Teste a velocidade da sua internet grátis. Meça download, upload e ping em segundos.", "Descubra se sua conexão é rápida. Compare com a média da sua cidade. 100% grátis."

*Meu IP:*
- Headlines: "Qual é Meu IP?", "Descubra Seu IP Agora", "Veja o Que Sites Sabem de Você", "Verificar IP Grátis", "Teste de Privacidade"
- Descriptions: "Descubra seu IP, localização e provedor. Veja o que qualquer site pode ver sobre você.", "Seu IP revela sua localização. Verifique agora e proteja sua privacidade online."

*Verificar Senha:*
- Headlines: "Sua Senha Foi Vazada?", "Verificar Senha Grátis", "Senha Hackeada?", "Teste Sua Senha Agora", "Proteção de Dados"
- Descriptions: "Verifique se sua senha apareceu em vazamentos. Teste a força da sua senha. 100% privado.", "Descubra em quantos vazamentos sua senha apareceu. Verificação gratuita e segura."

*Verificar Link:*
- Headlines: "Link Seguro?", "Verificar Link Grátis", "Detector de Phishing", "Site Perigoso?", "Golpe no WhatsApp?"
- Descriptions: "Verifique se um link é seguro antes de clicar. Scanner contra phishing, malware e sites falsos.", "Recebeu link suspeito? Verifique grátis. Proteja-se contra golpes e phishing."

### Campaign 2: Segurança Digital — R$12/day

**Goal:** Website traffic via security concern keywords
**Strategy:** Users worried about security → educational content builds trust → waitlist signup
**Bidding:** Maximize clicks
**Location:** Brazil only
**Language:** Portuguese

| Ad Group | Keywords | Match Type | Landing Page |
|----------|----------|------------|-------------|
| WhatsApp Clonado | whatsapp clonado, como saber se whatsapp foi clonado, whatsapp hackeado, recuperar whatsapp | Phrase | escudovpn.com/blog/whatsapp-clonado |
| Vazamento de Dados | vazamento de dados, email vazado, dados pessoais vazados, meus dados vazaram | Phrase | escudovpn.com/vazamentos |
| Phishing | o que é phishing, golpe online, site falso, como identificar phishing, email falso | Phrase | escudovpn.com/blog/o-que-e-phishing |
| Bloqueador de Anúncios | bloqueador de anúncios, bloquear anúncios celular, adblock grátis, remover anúncios | Phrase | escudovpn.com/bloqueador-de-anuncios |

**Ad Copy (per ad group — 2 responsive search ads each):**

*WhatsApp Clonado:*
- Headlines: "WhatsApp Clonado?", "Como Saber se Foi Clonado", "Recuperar WhatsApp", "Proteja Seu WhatsApp", "Guia Completo Grátis"
- Descriptions: "Descubra se seu WhatsApp foi clonado e como recuperar. Guia completo de segurança.", "Seu WhatsApp pode estar clonado. Veja os sinais e como se proteger agora."

*Vazamento de Dados:*
- Headlines: "Seus Dados Vazaram?", "Verificar Email Vazado", "Vazamento de Dados", "Proteção Grátis", "Verifique Agora"
- Descriptions: "Verifique se seu email foi exposto em vazamentos de dados. Ferramenta gratuita.", "Descubra se seus dados pessoais foram vazados. Verificação instantânea e gratuita."

*Phishing:*
- Headlines: "Golpe Online?", "Identificar Phishing", "Site Falso?", "Proteção Anti-Phishing", "Guia de Segurança"
- Descriptions: "Aprenda a identificar emails e sites falsos. Guia prático com exemplos reais.", "Recebeu email suspeito? Saiba como identificar phishing e proteger seus dados."

*Bloqueador de Anúncios:*
- Headlines: "Bloqueador de Anúncios Grátis", "Remover Anúncios do Celular", "Adblock Grátis", "Bloquear Anúncios", "Sem Mais Propagandas"
- Descriptions: "Bloqueie anúncios, malware e rastreadores em todos os apps. Proteção DNS gratuita.", "Cansado de anúncios? Bloqueie 500.000+ domínios maliciosos automaticamente. Grátis."

### Campaign 3: VPN Intent — R$8/day

**Goal:** Website traffic from users actively seeking VPN
**Strategy:** Higher-intent, higher-CPC keywords → direct to waitlist/use-case pages
**Bidding:** Maximize clicks
**Location:** Brazil only
**Language:** Portuguese

| Ad Group | Keywords | Match Type | Landing Page |
|----------|----------|------------|-------------|
| VPN Brasil | vpn brasileira, melhor vpn brasil, vpn grátis brasil, vpn segura brasil | Phrase | escudovpn.com/lista-de-espera |
| Streaming | vpn netflix, desbloquear streaming, vpn disney plus, assistir conteúdo bloqueado, vpn streaming | Phrase | escudovpn.com/casos-de-uso/streaming |

**Ad Copy:**

*VPN Brasil:*
- Headlines: "VPN Brasileira", "Escudo VPN — Feita no Brasil", "A Partir de R$9,80/mês", "7 Dias Grátis", "IPs Residenciais Reais"
- Descriptions: "A primeira VPN brasileira com IPs residenciais reais. Streaming sem bloqueio, privacidade total.", "VPN feita no Brasil, para o Brasil. Servidor em SP, 5ms de latência. Teste grátis por 7 dias."

*Streaming:*
- Headlines: "VPN para Streaming", "Netflix Sem Bloqueio", "Assista de Qualquer Lugar", "Disney+ Desbloqueado", "Streaming Sem Limites"
- Descriptions: "Acesse catálogos internacionais de streaming com IPs residenciais reais. Sem buffering.", "Netflix, Disney+, HBO Max sem bloqueio geográfico. VPN brasileira com velocidade máxima."

## Negative Keywords (Account-Level)

- grátis download (freeloader intent)
- crack, pirata, torrent (piracy intent)
- como hackear (hacking intent)
- vpn china (irrelevant geo)
- emprego, vaga (job seeker intent)
- whatsapp web, whatsapp download (navigational, not security intent)
- teste de velocidade ookla, speedtest.net (brand navigational — unwinnable)
- configurar, como configurar (support intent, not discovery)
- curso, aula (educational intent, not tool usage)

**Campaign-level negatives for Campaign 3 (VPN Intent):**
- grátis (freeloader — we want paying customers in this campaign)

## Agent Execution Steps

1. **Configure proxy** — Generate sticky session credentials for BR
2. **Launch browser** — Playwright MCP with SOCKS5 proxy
3. **Login** — Navigate to ads.google.com, enter credentials, user handles OTP
4. **Select account** — Navigate MCC to Escudo VPN account
5. **Create Campaign 1** — Ferramentas Grátis (4 ad groups, 8 ads)
6. **Create Campaign 2** — Segurança Digital (4 ad groups, 8 ads)
7. **Create Campaign 3** — VPN Intent (2 ad groups, 4 ads)
8. **Set negative keywords** — Account-level negatives
9. **Review & activate** — Verify all settings, enable campaigns
10. **Screenshot confirmation** — Capture final state for user verification

## Error Handling

- **Login fails:** Agent retries once, then asks user to verify credentials
- **MCC account not found:** Agent takes screenshot and asks user to identify the correct account
- **Campaign creation partially fails:** Agent saves progress, reports which campaigns succeeded/failed, and offers to retry the failed ones
- **CAPTCHA or verification screen:** Agent pauses, takes screenshot, asks user to solve manually
- **Proxy disconnects:** Agent detects page load failure, reports to user, suggests reconnecting with a new session
- **Google Ads UI changed/unexpected modal:** Agent takes a vision screenshot and attempts to adapt; if stuck, pauses and asks user

## Ad Extensions (Campaign 3 — VPN Intent)

**Sitelink extensions:**
- "Teste de Velocidade" → /teste-de-velocidade
- "Meu IP" → /meu-ip
- "Verificar Senha" → /verificar-senha
- "Comparativo VPN" → /comparativo

**Callout extensions:**
- "100% Brasileira"
- "IPs Residenciais"
- "Bloqueio de Anúncios Grátis"
- "Servidor em São Paulo"

## Monthly Budget Cap

Set account-level monthly budget cap of R$1.100 as a safety net against accidental overspend.

## Constraints

- All campaigns Brazil-only targeting
- Portuguese language only
- R$35/day total budget (R$15 + R$12 + R$8)
- No image/display ads for now — search only (simpler, faster to set up)
- Phrase match keywords (balanced reach vs relevance)
- Maximize clicks bidding (no smart bidding until data accumulates)

## Success Criteria

- All 3 campaigns created and active in Google Ads
- 10 ad groups with correct keywords and landing pages
- 20 responsive search ads live
- Budget correctly allocated
- Brazil-only targeting confirmed
- Campaigns submitted for Google review / activated
