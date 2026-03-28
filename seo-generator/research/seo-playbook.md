# Programmatic SEO Playbook — Replicable Framework

> This playbook documents the complete SEO strategy used for Escudo VPN (escudovpn.com).
> It is designed to be replicated for any business with local/geographic data.

---

## Phase 0: Keyword Research & Market Mapping

### Tools
- **DataForSEO API** ($50 minimum deposit, pay-per-use)
  - `keywords_data/google_ads/search_volume/live` — $0.075 per request (up to 1,000 keywords)
  - `keywords_data/google_ads/keywords_for_keywords/live` — $0.075 per request (keyword suggestions)
  - `serp/google/organic/live/regular` — $0.002 per SERP (see who ranks)
  - `dataforseo_labs/google/related_keywords/live` — $0.01 per request

### Process
1. **Seed keywords** — Start with 5-10 obvious terms for your niche
2. **Expand** — Use `keywords_for_keywords` to get 200-3,000 related terms per seed
3. **Volume check** — Batch all discovered keywords through `search_volume` (1,000 per request = cheap)
4. **SERP analysis** — For top 20 keywords, pull actual Google results to see competitors
5. **Classify** — Sort into tiers by volume, competition, and intent

### Keyword Tiers
| Tier | Volume | Strategy | Content Type |
|------|--------|----------|--------------|
| 1 | 100k+ | Build dedicated tools/pages | Interactive tools, hub pages |
| 2 | 5k-100k | Blog posts + landing pages | Educational content |
| 3 | 1k-5k | Blog posts | How-to guides, comparisons |
| 4 | 100-1k | Feature pages | Product-specific landing pages |
| 5 | <100 | Programmatic pages (scale) | Auto-generated from data |

### Cost for full keyword research
- Typical project: 10-15 API calls = $0.50-$1.00
- SERP analysis for top 20 keywords: $0.04
- **Total: ~$1-2 per project**

---

## Phase 1: Programmatic Pages (Long-Tail Capture)

### Concept
Generate one page per geographic entity (city, state, region) using real data.
Each page targets "[topic] em [city]" queries.

### Requirements
- **Unique data per page** — Not just city name swapped. Real metrics that differ.
- **Internal linking** — Every page links to neighbors, parent (state), and hub (national)
- **Structured data** — JSON-LD Article + BreadcrumbList schemas
- **Sitemap** — Dedicated sitemap for programmatic pages

### URL Structure
```
/{topic}/                           → National index
/{topic}/{state}/                   → State page
/{topic}/{state}/{city-slug}        → City page
```

### Technical Stack
- Python 3 + Jinja2 templates
- PostgreSQL for data (or any structured data source)
- Static HTML output (no framework needed)
- nginx with `try_files $uri $uri/ $uri.html =404` for clean URLs

### Template Architecture
```
templates/
  base.html          ← Shared nav/footer/CSS (edit once, all pages update)
  municipality.html   ← City page template
  state.html          ← State overview
  national.html       ← National index
```

### Key Principle
One template change → regenerate all pages in seconds.
Never manually edit a generated page.

### SEO Elements (per page)
- **Title:** `{Topic} em {City}, {State} — {Metric} | {Brand}`
- **Meta description:** Dynamic with 2-3 key data points
- **H1:** One per page, includes city name
- **Canonical URL:** Self-referencing
- **JSON-LD:** Article + BreadcrumbList
- **Internal links:** Neighbors (geographic), parent state, national index

### Expected Results
- 5,000+ pages × 5-10 visits/month each = 25,000-50,000 visits/month
- Zero competition on long-tail city queries
- Domain authority boost from massive internal link graph

---

## Phase 2: Hook Pages (High-Volume Traffic Capture)

### Concept
Build interactive tool pages targeting the highest-volume keywords.
These pages serve as entry points that funnel users into the product.

### Speed Test Hook (for connectivity/VPN products)
**Target keyword:** "teste de velocidade" — 2,740,000 searches/month

**How it works:**
1. User searches "teste de velocidade"
2. Lands on your speed test page
3. Runs actual speed test (JS-based, LibreSpeed or similar)
4. Page shows: "Your speed: X Mbps. Average in {your city}: Y Mbps"
5. CTA: "Your connection is slower/faster than average. Protect it with {Product}"

**Who ranks today:**
| Rank | Domain | Monetization |
|------|--------|--------------|
| #1 | speedtest.net (Ookla) | Ads + VPN partnership (with one of the big VPN companies integrated) |
| #2 | fast.com (Netflix) | Brand awareness, free |
| #3 | minhaconexao.com.br | Ads (Google AdSense) |
| #4 | nperf.com | Ads + ISP partnerships |
| #5 | brasilbandalarga.com.br (EAQ/Anatel) | Government, free |
| #6 | vivo.com.br | ISP brand, customer retention |

**Our angle:** None of them have per-municipality average data.
We show "your result vs your city average vs state average vs national average."
That's unique content Google will reward.

**Revenue model:** Speed test = free hook → VPN conversion.
Ookla makes money from ads + VPN affiliate. We skip the middleman — we ARE the VPN.

### Breach Checker Hook (for security products)
**Target keywords:** "vazamento de dados" (8,100/mo), "cpf vazados" (2,900/mo), "dados vazados" (1,600/mo)

**Implementation:** Already built (`vazamentos.html` using HIBP API).
Needs SEO optimization: proper title, meta, structured data, internal links.

### Ad Blocker Hook (for privacy products)
**Target keywords:** "bloqueador de anuncios" (22,200/mo), "adblock" (90,500/mo)

**Implementation:** Landing page explaining DNS-level ad blocking.
CTA: "Block ads system-wide with {Product} — no browser extension needed"

---

## Phase 3: Content Marketing (Blog)

### Content Calendar Strategy
Blog posts targeting Tier 2-3 keywords. One post per week minimum.

### Priority Blog Topics (by search volume)

| Priority | Topic | Target Keyword | Vol/mo | Type |
|----------|-------|---------------|--------|------|
| 1 | O que e phishing e como se proteger | phishing o que é | 14,800 | Educational |
| 2 | LGPD: O que e e quais sao seus direitos | lgpd o que é | 12,100 | Educational |
| 3 | Vazamento de dados: como saber e o que fazer | vazamento de dados | 8,100 | Educational + Tool |
| 4 | DNS privado: o que e e como configurar | dns privado | 5,400 | Educational |
| 5 | WhatsApp clonado: como prevenir | whatsapp clonado | 3,600 | Educational |
| 6 | Seguranca digital: guia completo | seguranca digital | 3,600 | Hub/Pillar |
| 7 | Proteção de dados pessoais: guia pratico | proteção de dados | 2,900 | Educational |
| 8 | CPF vazado: como verificar e se proteger | cpf vazados | 2,900 | Educational + Tool |
| 9 | Melhores bloqueadores de anuncio | bloqueador de anuncios | 22,200 | Comparison + CTA |
| 10 | Escudo VPN vs NordVPN | nordvpn | 40,500 | Comparison |

### Blog Post Template Structure
1. **H1** — Target keyword in title
2. **Intro** — Problem statement + hook
3. **What is [topic]** — Definition, examples
4. **How it affects you** — Real data from our database (city-level stats if applicable)
5. **How to protect yourself** — Actionable steps
6. **How {Product} helps** — Soft CTA, not pushy
7. **FAQ** — 3-5 "People Also Ask" questions (from SERP data)
8. **Related articles** — Internal links to other blog posts
9. **CTA** — Download/signup

### Blog URL Structure
```
/blog/{slug}          → Blog post
/blog/                → Blog index
```

---

## Phase 4: Social Distribution

### Twitter/X Auto-Posting
- **API:** Free tier (1,500 posts/month)
- **Content types:**
  - Daily city spotlight: "Indice de vulnerabilidade digital em {City}: {score}/100"
  - Weekly state summary: "As 5 cidades mais vulneraveis de {State}"
  - Blog post promotion: Share new articles
  - Data-driven insights: "X% das escolas em {State} nao tem internet"

### Reddit
- **Subreddits:** r/brasil, r/InternetBrasil, r/privacidade, r/brdev
- **Strategy:** Manual first. Share genuinely useful content (speed data, breach checker)
- **Rule:** Never post direct product links. Share the tool/data, product sells itself.

### LinkedIn
- **Strategy:** B2B angle for enterprise VPN
- **Content:** LGPD compliance, corporate data protection

---

## Phase 5: Comparison & Competitor Pages

### Target Keywords
| Keyword | Vol/mo |
|---------|--------|
| nordvpn | 40,500 |
| surfshark | 14,800 |
| expressvpn | 9,900 |
| nordvpn preço | 880 |
| nordvpn é bom | 90 |

### Page Structure
```
/comparativo/nordvpn     → Escudo vs NordVPN
/comparativo/surfshark   → Escudo vs Surfshark
/comparativo/expressvpn  → Escudo vs ExpressVPN
/comparativo/            → Index: Compare all VPNs
```

### Comparison Table Elements
- Price (Escudo is cheaper, Brazilian)
- Servers (locations)
- Ad blocking (Escudo has built-in, NordVPN doesn't by default)
- Post-quantum encryption (Escudo unique advantage)
- LGPD compliance (Escudo is Brazilian = automatic advantage)
- Speed test results
- Kill switch
- Streaming unblock

---

## Phase 6: Technical SEO

### Sitemap Strategy
```
sitemap-index.xml        → Points to all sitemaps
  sitemap-main.xml       → Main site pages
  sitemap-seo.xml        → Programmatic pages (5,599 URLs)
  sitemap-blog.xml       → Blog posts
```

### robots.txt
```
User-agent: Googlebot
Allow: /

User-agent: GPTBot
Allow: /

User-agent: OAI-SearchBot
Allow: /

User-agent: *
Allow: /

Sitemap: https://yourdomain.com/sitemap-index.xml
```

### Nginx Configuration
```nginx
location /seguranca-digital/ {
    try_files $uri $uri/ $uri.html =404;
    expires 7d;
    add_header Cache-Control "public, immutable";
}
```

### Page Speed
- No JavaScript frameworks (plain HTML/CSS)
- Inline critical CSS in base template
- Preconnect Google Fonts
- No images on programmatic pages (data-driven, text + charts)
- Target: <1s load time, 95+ Lighthouse score

---

## Replication Guide

### To replicate this for another business:

1. **Identify your data advantage** — What unique data do you have per geographic area?
2. **Run keyword research** ($1-2 via DataForSEO) — Find your "teste de velocidade" (hook keyword)
3. **Build programmatic pages** — Fork the generator, swap templates + data source
4. **Build your hook page** — The high-volume interactive tool
5. **Write 10 blog posts** — Target Tier 2 keywords
6. **Set up social distribution** — Twitter auto-posting from same data

### Data sources that work for programmatic SEO:
- Government open data (census, infrastructure, crime)
- Telecom data (coverage, speed, providers)
- Economic data (jobs, income, cost of living)
- Health data (hospitals, coverage, indicators)
- Education data (schools, connectivity, test scores)
- Real estate data (prices, inventory, trends)

### What makes it work:
- **Genuinely unique data per page** (not just city name swapped)
- **Proprietary composite score** (our Vulnerability Index)
- **Internal link graph** (neighbors + hierarchy)
- **Hook page** for high-volume traffic entry
- **Blog content** for mid-volume educational traffic
- **Social distribution** for initial signals + backlinks

---

## Budget Summary

| Item | Cost | Frequency |
|------|------|-----------|
| DataForSEO keyword research | $1-2 | One-time per project |
| DataForSEO rank tracking (optional) | $5-10/mo | Monthly (50-100 keywords) |
| Twitter/X API | Free | Ongoing |
| Hosting | Already have | - |
| Content writing | AI-generated + human review | Ongoing |
| **Total monthly** | **$5-10** | After initial setup |

---

## Execution Order

1. ~~Keyword research~~ ✅ Done
2. ~~Programmatic page generator~~ ✅ Done
3. **Speed test hook page** ← NEXT
4. **SEO-optimize existing pages** (vazamentos.html, comparativo.html)
5. **Connect to real database** (whitelist server IP in pg_hba.conf)
6. **Generate all 5,571 pages** from real data
7. **Deploy + submit sitemap**
8. **Blog post generator** (10 priority articles)
9. **Twitter auto-posting**
10. **Comparison pages** (vs NordVPN, Surfshark, ExpressVPN)
