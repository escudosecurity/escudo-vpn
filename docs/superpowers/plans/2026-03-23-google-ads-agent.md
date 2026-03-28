# Google Ads Automation Agent — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This is a browser automation plan — tasks are sequential UI interactions, not code files.

**Goal:** Launch 3 Google Ads search campaigns for Escudo VPN through browser automation using Playwright MCP with a Brazilian residential proxy.

**Architecture:** Claude Code drives Playwright MCP browser through an IPRoyal SOCKS5 residential proxy (Brazil). The agent navigates Google Ads UI to create campaigns, ad groups, keywords, and ads. User handles OTP during login.

**Tech Stack:** Playwright MCP, IPRoyal SOCKS5 proxy, Google Ads web UI, Claude Code

**Spec:** `docs/superpowers/specs/2026-03-23-google-ads-agent-design.md`

---

## Error Handling (applies to all tasks)

- **Login fails:** Retry once, then ask user to verify credentials
- **MCC account not found:** Take screenshot, ask user to identify the correct account
- **Campaign creation partially fails:** Report which campaigns succeeded/failed, offer to retry
- **CAPTCHA or verification screen:** Pause, take screenshot, ask user to solve manually
- **Proxy disconnects (page load failure):** Report to user, suggest reconnecting with new session
- **Unexpected modal/popup:** Take a vision screenshot, attempt to dismiss or adapt. If stuck, pause and ask user
- **Google Ads UI flow note:** The campaign wizard typically lets you create only the first ad group + ad during initial setup. After saving the campaign, navigate to the campaign → Ad Groups → "+ New Ad Group" to add remaining ad groups. Each step below notes when this navigation is needed.

---

### Task 1: Configure and Launch Browser with Brazilian Proxy

**Context:** We need to start the Playwright MCP browser routed through a Brazilian residential IP so Google Ads sees a Brazilian user.

- [ ] **Step 1: Read the IPRoyal API token from environment**

Read the `IPROYAL_API_TOKEN` value from `/home/dev/pulsovpn/escudo-vpn/.env`

- [ ] **Step 2: Generate sticky session proxy credentials**

Build the SOCKS5 URL with a 4-hour sticky session:
```
Username: {TOKEN}__country-br__session-{random_uuid_no_dashes}__lifetime-240m
Password: {TOKEN}
URL: socks5://USERNAME:PASSWORD@geo.iproyal.com:32325
```
URL-encode the username (the `__` separators and alphanumeric chars are safe, but encode if needed).

- [ ] **Step 3: Update MCP config to use proxy**

Modify the Playwright MCP configuration to include the proxy. The MCP config is at:
`/home/dev/.claude/plugins/marketplaces/claude-plugins-official/external_plugins/playwright/.mcp.json`

Update args to:
```json
{
  "playwright": {
    "command": "npx",
    "args": [
      "@playwright/mcp@latest",
      "--proxy-server", "socks5://ENCODED_USER:ENCODED_PASS@geo.iproyal.com:32325",
      "--browser", "chrome",
      "--caps", "vision"
    ]
  }
}
```

- [ ] **Step 4: Restart/reload MCP and verify browser launches**

After updating the config, the Playwright MCP server needs to be restarted. Use the browser_navigate tool to go to `https://httpbin.org/ip` and verify the response shows a Brazilian IP address.

- [ ] **Step 5: Verify Brazilian IP**

Navigate to `https://httpbin.org/ip` and confirm the IP is Brazilian. If not, check proxy credentials and retry.

---

### Task 2: Log Into Google Ads

**Context:** Navigate to Google Ads and authenticate. User will provide credentials in chat and handle OTP manually.

- [ ] **Step 1: Navigate to Google Ads**

Use browser_navigate to go to `https://ads.google.com`

- [ ] **Step 2: Ask user for email**

Ask the user: "Please provide your Google Ads email address."

- [ ] **Step 3: Enter email on Google login page**

Use browser_type or browser_click to enter the email in the login form and click Next.

- [ ] **Step 4: Ask user for password**

Ask the user: "Please provide your Google Ads password."

- [ ] **Step 5: Enter password**

Use browser_type to enter the password and click Next.

- [ ] **Step 6: Pause for OTP**

Tell the user: "Please complete the OTP/2FA verification in the browser. Type 'done' when you're finished."

Wait for user confirmation. If OTP is not required, the page will already show the dashboard.

- [ ] **Step 7: Verify login success**

Take a screenshot. Confirm we're on the Google Ads dashboard (MCC view). If not, troubleshoot with the user.

- [ ] **Step 8: Select the Escudo VPN account**

In the MCC, find and click the Escudo VPN sub-account. If the account name is different, take a screenshot and ask the user which account to select.

---

### Task 3: Set Monthly Budget Cap

**Context:** Before creating campaigns, set an account-level monthly budget cap as a safety net.

- [ ] **Step 1: Navigate to billing settings**

Click the gear/wrench icon (Tools & Settings) → Billing → Settings. Look for "Account budget" or "Monthly spending limit."

- [ ] **Step 2: Set monthly budget cap**

Set the account monthly budget to R$1.100 (approximately R$35/day × 31 days, rounded up for safety).

- [ ] **Step 3: Save and confirm**

Save the budget cap. Take a screenshot to confirm.

---

### Task 4: Create Campaign 1 — Ferramentas Grátis (R$15/day)

**Context:** This campaign targets high-volume utility keywords (speed test, IP check, password check, link check) to drive cheap traffic to Escudo's free tool pages.

- [ ] **Step 1: Start new campaign**

Click "+ New Campaign" or the equivalent button. Select:
- Campaign objective: **Website traffic**
- Campaign type: **Search**
- Enter website: `escudovpn.com`

- [ ] **Step 2: Configure campaign settings**

- Campaign name: `Ferramentas Grátis`
- Networks: **Search Network only** (uncheck Display Network)
- Location: **Brazil** only
- Language: **Portuguese**
- Bidding: **Maximize clicks**
- Daily budget: **R$15**

- [ ] **Step 3: Create Ad Group 1 — Teste de Velocidade**

- Ad group name: `Teste de Velocidade`
- Keywords (phrase match — wrap each in quotes):
  - "teste de velocidade"
  - "medir velocidade internet"
  - "speed test"
  - "teste de internet"

- [ ] **Step 4: Create Ad 1 for Teste de Velocidade**

Responsive Search Ad:
- Final URL: `https://escudovpn.com/teste-de-velocidade`
- Headlines:
  1. Teste de Velocidade Grátis
  2. Medir Velocidade da Internet
  3. Download, Upload e Ping
  4. Resultado em Segundos
  5. Compare com Sua Cidade
- Descriptions:
  1. Teste a velocidade da sua internet grátis. Meça download, upload e ping em segundos.
  2. Descubra se sua conexão é rápida. Compare com a média da sua cidade. 100% grátis.

- [ ] **Step 5: Create Ad 2 for Teste de Velocidade**

Responsive Search Ad (variation):
- Final URL: `https://escudovpn.com/teste-de-velocidade`
- Headlines:
  1. Teste de Internet Grátis
  2. Velocidade Real da Sua Conexão
  3. Download, Upload e Ping
  4. Resultado Instantâneo
  5. Ferramenta 100% Gratuita
- Descriptions:
  1. Meça a velocidade real da sua internet. Resultado em segundos com download, upload e latência.
  2. Sua internet está lenta? Teste agora e descubra. Ferramenta grátis do Escudo VPN.

- [ ] **Step 6: Save Campaign 1 with first ad group**

Save the campaign. It will be created with Ad Group 1 (Teste de Velocidade).

**UI Navigation:** After saving, go back to Campaign "Ferramentas Grátis" → Ad Groups → "+ New Ad Group" to add the remaining 3 ad groups.

- [ ] **Step 7: Create Ad Group 2 — Meu IP**

- Ad group name: `Meu IP`
- Keywords:
  - "meu ip"
  - "qual meu ip"
  - "qual é meu ip"
  - "descobrir meu ip"
  - "meu endereço ip"
- **Ad 1** — Final URL: `https://escudovpn.com/meu-ip`
  - Headlines: Qual é Meu IP? | Descubra Seu IP Agora | Veja o Que Sites Sabem de Você | Verificar IP Grátis | Teste de Privacidade
  - Desc 1: Descubra seu IP, localização e provedor. Veja o que qualquer site pode ver sobre você.
  - Desc 2: Seu IP revela sua localização. Verifique agora e proteja sua privacidade online.
- **Ad 2** — Final URL: `https://escudovpn.com/meu-ip`
  - Headlines: Meu IP — Verificação Grátis | O Que a Internet Sabe de Você | Seu IP Está Exposto | Verifique Seu IP | Privacidade Online
  - Desc 1: Qualquer site pode ver seu IP, localização e provedor. Verifique o que está exposto agora.
  - Desc 2: Seu endereço IP revela mais do que você imagina. Teste grátis de privacidade online.

- [ ] **Step 8: Create Ad Group 3 — Verificar Senha**

- Ad group name: `Verificar Senha`
- Keywords:
  - "verificar senha vazada"
  - "minha senha foi vazada"
  - "senha vazada"
  - "senha hackeada"
- **Ad 1** — Final URL: `https://escudovpn.com/verificar-senha`
  - Headlines: Sua Senha Foi Vazada? | Verificar Senha Grátis | Senha Hackeada? | Teste Sua Senha Agora | Proteção de Dados
  - Desc 1: Verifique se sua senha apareceu em vazamentos. Teste a força da sua senha. 100% privado.
  - Desc 2: Descubra em quantos vazamentos sua senha apareceu. Verificação gratuita e segura.
- **Ad 2** — Final URL: `https://escudovpn.com/verificar-senha`
  - Headlines: Senha Vazada? Verifique Agora | Teste de Senha Grátis | Seus Dados Estão Seguros? | Verificador de Senhas | Proteção Contra Hackers
  - Desc 1: Milhões de senhas já foram vazadas. Verifique se a sua está entre elas. Ferramenta gratuita.
  - Desc 2: Teste a força da sua senha e descubra se ela apareceu em ataques. 100% privado e seguro.

- [ ] **Step 9: Create Ad Group 4 — Verificar Link**

- Ad group name: `Verificar Link`
- Keywords:
  - "verificar link seguro"
  - "link perigoso"
  - "site seguro"
  - "golpe whatsapp"
  - "verificar url"
- **Ad 1** — Final URL: `https://escudovpn.com/verificar-link`
  - Headlines: Link Seguro? | Verificar Link Grátis | Detector de Phishing | Site Perigoso? | Golpe no WhatsApp?
  - Desc 1: Verifique se um link é seguro antes de clicar. Scanner contra phishing, malware e sites falsos.
  - Desc 2: Recebeu link suspeito? Verifique grátis. Proteja-se contra golpes e phishing.
- **Ad 2** — Final URL: `https://escudovpn.com/verificar-link`
  - Headlines: Esse Link é Seguro? | Scanner de Links Grátis | Proteção Contra Golpes | Verificar URL | Anti-Phishing
  - Desc 1: Antes de clicar, verifique. Scanner gratuito detecta phishing, malware e sites falsos.
  - Desc 2: Golpe no WhatsApp? Link suspeito? Verifique a segurança de qualquer URL gratuitamente.

- [ ] **Step 10: Take screenshot of Campaign 1**

Take a screenshot showing all 4 ad groups with their ads and keywords. Verify completeness.

---

### Task 5: Create Campaign 2 — Segurança Digital (R$12/day)

**Context:** Targets users with security concerns — WhatsApp cloning, data breaches, phishing, ad blocking.

- [ ] **Step 1: Start new campaign**

Click "+ New Campaign":
- Objective: **Website traffic**
- Type: **Search**
- Website: `escudovpn.com`

- [ ] **Step 2: Configure campaign settings**

- Campaign name: `Segurança Digital`
- Networks: **Search Network only**
- Location: **Brazil**
- Language: **Portuguese**
- Bidding: **Maximize clicks**
- Daily budget: **R$12**

- [ ] **Step 3: Create Ad Group 1 — WhatsApp Clonado**

- Ad group name: `WhatsApp Clonado`
- Keywords:
  - "whatsapp clonado"
  - "como saber se whatsapp foi clonado"
  - "whatsapp hackeado"
  - "recuperar whatsapp"
- **Ad 1** — Final URL: `https://escudovpn.com/blog/whatsapp-clonado`
  - Headlines: WhatsApp Clonado? | Como Saber se Foi Clonado | Recuperar WhatsApp | Proteja Seu WhatsApp | Guia Completo Grátis
  - Desc 1: Descubra se seu WhatsApp foi clonado e como recuperar. Guia completo de segurança.
  - Desc 2: Seu WhatsApp pode estar clonado. Veja os sinais e como se proteger agora.
- **Ad 2** — Final URL: `https://escudovpn.com/blog/whatsapp-clonado`
  - Headlines: Seu WhatsApp Foi Hackeado? | Sinais de Clonagem | Como se Proteger | Guia de Segurança | Recuperação Rápida
  - Desc 1: Aprenda a identificar se seu WhatsApp foi clonado. Passo a passo para recuperar e proteger.
  - Desc 2: WhatsApp hackeado ou clonado? Veja os sinais de alerta e como agir imediatamente.

- [ ] **Step 4: Save Campaign 2 with first ad group, then add remaining ad groups**

Save the campaign. Navigate back to Campaign "Segurança Digital" → Ad Groups → "+ New Ad Group".

- [ ] **Step 5: Create Ad Group 2 — Vazamento de Dados**

- Ad group name: `Vazamento de Dados`
- Keywords:
  - "vazamento de dados"
  - "email vazado"
  - "dados pessoais vazados"
  - "meus dados vazaram"
- **Ad 1** — Final URL: `https://escudovpn.com/vazamentos`
  - Headlines: Seus Dados Vazaram? | Verificar Email Vazado | Vazamento de Dados | Proteção Grátis | Verifique Agora
  - Desc 1: Verifique se seu email foi exposto em vazamentos de dados. Ferramenta gratuita.
  - Desc 2: Descubra se seus dados pessoais foram vazados. Verificação instantânea e gratuita.
- **Ad 2** — Final URL: `https://escudovpn.com/vazamentos`
  - Headlines: Email Vazado? Descubra Agora | Verificador de Vazamentos | Seus Dados Estão Seguros? | Teste Grátis | Proteção de Dados
  - Desc 1: Bilhões de dados já foram vazados. Verifique se seu email está entre eles. Grátis.
  - Desc 2: Ferramenta gratuita para verificar vazamentos de dados. Resultado instantâneo e privado.

- [ ] **Step 6: Create Ad Group 3 — Phishing**

- Ad group name: `Phishing`
- Keywords:
  - "o que é phishing"
  - "golpe online"
  - "site falso"
  - "como identificar phishing"
  - "email falso"
- **Ad 1** — Final URL: `https://escudovpn.com/blog/o-que-e-phishing`
  - Headlines: Golpe Online? | Identificar Phishing | Site Falso? | Proteção Anti-Phishing | Guia de Segurança
  - Desc 1: Aprenda a identificar emails e sites falsos. Guia prático com exemplos reais.
  - Desc 2: Recebeu email suspeito? Saiba como identificar phishing e proteger seus dados.
- **Ad 2** — Final URL: `https://escudovpn.com/blog/o-que-e-phishing`
  - Headlines: Como Identificar Phishing | Email Falso? | Proteção Online | Guia Completo | Dicas de Segurança
  - Desc 1: Não caia em golpes online. Aprenda a identificar phishing com exemplos reais e práticos.
  - Desc 2: Sites e emails falsos estão cada vez mais sofisticados. Saiba como se proteger hoje.

- [ ] **Step 7: Create Ad Group 4 — Bloqueador de Anúncios**

- Ad group name: `Bloqueador de Anúncios`
- Keywords:
  - "bloqueador de anúncios"
  - "bloquear anúncios celular"
  - "adblock grátis"
  - "remover anúncios"
- **Ad 1** — Final URL: `https://escudovpn.com/bloqueador-de-anuncios`
  - Headlines: Bloqueador de Anúncios Grátis | Remover Anúncios do Celular | Adblock Grátis | Bloquear Anúncios | Sem Mais Propagandas
  - Desc 1: Bloqueie anúncios, malware e rastreadores em todos os apps. Proteção DNS gratuita.
  - Desc 2: Cansado de anúncios? Bloqueie 500.000+ domínios maliciosos automaticamente. Grátis.
- **Ad 2** — Final URL: `https://escudovpn.com/bloqueador-de-anuncios`
  - Headlines: Adblock para Celular Grátis | Bloqueio de Anúncios DNS | Remover Propagandas | Proteção Automática | Navegue Sem Anúncios
  - Desc 1: Bloqueio inteligente via DNS. Elimina anúncios, malware e rastreadores em todos os apps.
  - Desc 2: Mais de 500 mil domínios maliciosos bloqueados automaticamente. Proteção gratuita.

- [ ] **Step 8: Take screenshot of Campaign 2**

Take a screenshot showing all 4 ad groups. Verify completeness.

---

### Task 6: Create Campaign 3 — VPN Intent (R$8/day)

**Context:** Bottom-funnel campaign targeting users actively searching for VPN. Includes ad extensions.

- [ ] **Step 1: Start new campaign**

Click "+ New Campaign":
- Objective: **Website traffic**
- Type: **Search**
- Website: `escudovpn.com`

- [ ] **Step 2: Configure campaign settings**

- Campaign name: `VPN Intent`
- Networks: **Search Network only**
- Location: **Brazil**
- Language: **Portuguese**
- Bidding: **Maximize clicks**
- Daily budget: **R$8**

- [ ] **Step 3: Create Ad Group 1 — VPN Brasil**

- Ad group name: `VPN Brasil`
- Keywords:
  - "vpn brasileira"
  - "melhor vpn brasil"
  - "vpn grátis brasil"
  - "vpn segura brasil"
- **Ad 1** — Final URL: `https://escudovpn.com/lista-de-espera`
  - Headlines: VPN Brasileira | Escudo VPN — Feita no Brasil | A Partir de R$9,80/mês | 7 Dias Grátis | IPs Residenciais Reais
  - Desc 1: A primeira VPN brasileira com IPs residenciais reais. Streaming sem bloqueio, privacidade total.
  - Desc 2: VPN feita no Brasil, para o Brasil. Servidor em SP, 5ms de latência. Teste grátis por 7 dias.
- **Ad 2** — Final URL: `https://escudovpn.com/lista-de-espera`
  - Headlines: Melhor VPN do Brasil | Escudo VPN — IPs Reais | Privacidade Total | Servidor em São Paulo | Teste Grátis
  - Desc 1: VPN brasileira com servidores em São Paulo. Latência de 5ms e IPs residenciais reais.
  - Desc 2: Construída no Brasil, para brasileiros. Bloqueio de anúncios grátis incluso. Teste 7 dias.

- [ ] **Step 4: Save Campaign 3 with first ad group, then add Streaming ad group**

Save. Navigate to Campaign "VPN Intent" → Ad Groups → "+ New Ad Group".

- [ ] **Step 5: Create Ad Group 2 — Streaming**

- Ad group name: `Streaming`
- Keywords:
  - "vpn netflix"
  - "desbloquear streaming"
  - "vpn disney plus"
  - "assistir conteúdo bloqueado"
  - "vpn streaming"
- **Ad 1** — Final URL: `https://escudovpn.com/casos-de-uso/streaming`
  - Headlines: VPN para Streaming | Netflix Sem Bloqueio | Assista de Qualquer Lugar | Disney+ Desbloqueado | Streaming Sem Limites
  - Desc 1: Acesse catálogos internacionais de streaming com IPs residenciais reais. Sem buffering.
  - Desc 2: Netflix, Disney+, HBO Max sem bloqueio geográfico. VPN brasileira com velocidade máxima.
- **Ad 2** — Final URL: `https://escudovpn.com/casos-de-uso/streaming`
  - Headlines: Desbloqueie Netflix e Disney+ | VPN Streaming Brasil | Sem Buffering | Assista Tudo | IP Residencial
  - Desc 1: IPs residenciais reais que não são detectados. Assista Netflix, Disney+ e HBO sem bloqueio.
  - Desc 2: Streaming sem limites com a VPN brasileira mais rápida. Servidor em SP, velocidade máxima.

- [ ] **Step 6: Take screenshot of Campaign 3**

Take a screenshot showing both ad groups. Verify completeness.

---

### Task 7: Add Negative Keywords

**Context:** Prevent wasted spend on irrelevant searches.

- [ ] **Step 1: Navigate to negative keyword lists**

Click Tools & Settings (wrench icon) → Shared Library → Negative keyword lists.

- [ ] **Step 2: Create a negative keyword list**

Name: `Escudo Global Negatives`

Add these as phrase match negatives:
- "grátis download"
- "crack"
- "pirata"
- "torrent"
- "como hackear"
- "vpn china"
- "emprego"
- "vaga"
- "whatsapp web"
- "whatsapp download"
- "teste de velocidade ookla"
- "speedtest.net"
- "configurar"
- "como configurar"
- "curso"
- "aula"

- [ ] **Step 3: Apply the list to all 3 campaigns**

Apply the `Escudo Global Negatives` list to: Ferramentas Grátis, Segurança Digital, VPN Intent.

- [ ] **Step 4: Add campaign-level negative for VPN Intent**

Navigate to Campaign 3 (VPN Intent) → Keywords → Negative keywords.
Add: "grátis" as a phrase match negative (prevents freeloader clicks on the paid-product campaign).

---

### Task 8: Add Ad Extensions (Campaign 3)

**Context:** Sitelinks and callouts for the VPN Intent campaign to improve CTR.

- [ ] **Step 1: Navigate to ad extensions**

Go to Ads & Extensions → Assets (or Extensions) for Campaign 3 (VPN Intent).

- [ ] **Step 2: Add sitelink extensions**

Add 4 sitelinks:
1. Text: "Teste de Velocidade" → URL: `https://escudovpn.com/teste-de-velocidade`
2. Text: "Meu IP" → URL: `https://escudovpn.com/meu-ip`
3. Text: "Verificar Senha" → URL: `https://escudovpn.com/verificar-senha`
4. Text: "Comparativo VPN" → URL: `https://escudovpn.com/comparativo`

- [ ] **Step 3: Add callout extensions**

Add 4 callouts:
1. "100% Brasileira"
2. "IPs Residenciais"
3. "Bloqueio de Anúncios Grátis"
4. "Servidor em São Paulo"

- [ ] **Step 4: Save extensions**

Save and confirm extensions are attached to Campaign 3.

---

### Task 9: Final Review and Activation

**Context:** Verify everything is correct before activating.

- [ ] **Step 1: Review all campaigns**

Navigate to the Campaigns overview. Take a screenshot showing:
- 3 campaigns listed
- Budget allocations (R$15 + R$12 + R$8 = R$35/day)
- Status of each campaign

- [ ] **Step 2: Verify targeting on each campaign**

Spot-check one campaign's settings to confirm:
- Location: Brazil only
- Language: Portuguese
- Network: Search only
- Bidding: Maximize clicks

- [ ] **Step 3: Verify keywords and ads**

Navigate to Ad Groups view. Confirm:
- 10 ad groups total (4 + 4 + 2)
- Each has keywords and 2 responsive search ads

- [ ] **Step 4: Enable all campaigns**

If campaigns were created in paused state, enable them now. Change status to "Enabled" for all 3.

- [ ] **Step 5: Take final confirmation screenshot**

Take a screenshot of the campaigns overview showing all 3 campaigns enabled. Share with user.

- [ ] **Step 6: Report to user**

Summarize what was created:
- 3 campaigns, 10 ad groups, 20 responsive search ads
- R$35/day total budget, R$1.100/month cap
- Brazil only, Portuguese, Search network
- Negative keywords applied (16 account-level + 1 campaign-level)
- Ad extensions on Campaign 3 (4 sitelinks + 4 callouts)
- Note: Ads will be reviewed by Google (1-3 business days) before serving

---

### Task 10: Cleanup — Restore MCP Config

**Context:** After execution, restore the Playwright MCP config to its default (no proxy) so it doesn't interfere with future browser usage.

- [ ] **Step 1: Restore original MCP config**

Reset `/home/dev/.claude/plugins/marketplaces/claude-plugins-official/external_plugins/playwright/.mcp.json` to:
```json
{
  "playwright": {
    "command": "npx",
    "args": ["@playwright/mcp@latest"]
  }
}
```
