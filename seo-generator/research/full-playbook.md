# Complete SEO & Growth Playbook — Replicable for Any Business

> Generated for Escudo VPN (escudovpn.com) — March 2026
> This playbook documents every step, tool, credential, and decision made.
> Fork it for any business with geographic/local data.

---

## 1. DataForSEO API (Keyword Research)

**Signup:** dataforseo.com (any email, no Gmail restriction)
**Auth:** Basic auth (base64 of email:password)
**Credentials used:**
```
Login: contato@escudovpn.com
API Auth Header: Authorization: Basic Y29udGF0b0Blc2N1ZG92cG4uY29tOmM5ODFkMTQ0MTVjN2ZkMjk=
```
**Minimum deposit:** $50 (pay-as-you-go, no subscription)
**Total spent on this project:** ~$0.77

### API Endpoints Used

#### Keyword Volume ($0.075 per request, up to 1,000 keywords)
```bash
curl -X POST "https://api.dataforseo.com/v3/keywords_data/google_ads/search_volume/live" \
  -H "Authorization: Basic YOUR_BASE64_AUTH" \
  -H "Content-Type: application/json" \
  -d '[{
    "keywords": ["keyword1", "keyword2", "...up to 1000"],
    "language_code": "pt",
    "location_code": 2076
  }]'
```
Location codes: 2076 = Brazil, 2840 = USA, 2826 = UK

#### Keyword Suggestions ($0.075 per request)
```bash
curl -X POST "https://api.dataforseo.com/v3/keywords_data/google_ads/keywords_for_keywords/live" \
  -H "Authorization: Basic YOUR_BASE64_AUTH" \
  -H "Content-Type: application/json" \
  -d '[{
    "keywords": ["seed keyword 1", "seed keyword 2"],
    "language_code": "pt",
    "location_code": 2076,
    "sort_by": "search_volume"
  }]'
```

#### SERP Results ($0.002 per query — very cheap)
```bash
curl -X POST "https://api.dataforseo.com/v3/serp/google/organic/live/regular" \
  -H "Authorization: Basic YOUR_BASE64_AUTH" \
  -H "Content-Type: application/json" \
  -d '[{
    "keyword": "your target keyword",
    "language_code": "pt",
    "location_code": 2076,
    "device": "desktop",
    "depth": 10
  }]'
```

#### Related Keywords ($0.01 per request)
```bash
curl -X POST "https://api.dataforseo.com/v3/dataforseo_labs/google/related_keywords/live" \
  -H "Authorization: Basic YOUR_BASE64_AUTH" \
  -H "Content-Type: application/json" \
  -d '[{
    "keyword": "your keyword",
    "language_code": "pt",
    "location_code": 2076,
    "limit": 50
  }]'
```

#### Check Balance
```bash
curl "https://api.dataforseo.com/v3/appendix/user_data" \
  -H "Authorization: Basic YOUR_BASE64_AUTH"
```

### Research Process (do this for any new project)

1. **Start with 5-10 seed keywords** — the obvious terms for your niche
2. **Run keywords_for_keywords** on each seed — gets 200-3,000 related terms
3. **Batch all discovered keywords** through search_volume (1,000 per request)
4. **Sort by volume** — classify into tiers
5. **Run SERP analysis** on top 20 keywords — see who ranks, find gaps
6. **Total cost:** ~$1-2 per project

---

## 2. Keyword Research Results (Escudo VPN)

### Tier 1: Build dedicated tools/pages (100k+ monthly searches)
| Keyword | Vol/mo | What we built |
|---|---|---|
| teste de velocidade | 2,740,000 | /teste-de-velocidade |
| adblock / bloqueador de anuncios | 112,700 | /bloqueador-de-anuncios |
| lgpd | 135,000 | /blog/lgpd-o-que-e |

### Tier 2: High-value content (5k-100k)
| Keyword | Vol/mo | What we built |
|---|---|---|
| nordvpn | 40,500 | /comparativo/nordvpn |
| phishing | 40,500 | /blog/o-que-e-phishing |
| surfshark | 14,800 | /comparativo/surfshark |
| phishing o que e | 14,800 | /blog/o-que-e-phishing |
| lgpd o que e | 12,100 | /blog/lgpd-o-que-e |
| expressvpn | 9,900 | /comparativo/expressvpn |
| vazamento de dados | 8,100 | /blog/vazamento-de-dados |
| dns privado | 5,400 | /blog/dns-privado |

### Tier 3: Blog content (1k-5k)
| Keyword | Vol/mo | What we built |
|---|---|---|
| whatsapp clonado | 3,600 | /blog/whatsapp-clonado |
| seguranca digital | 3,600 | /blog/seguranca-digital-guia + 5,571 city pages |
| cpf vazados | 2,900 | /blog/vazamento-de-dados |
| proteção de dados | 2,900 | /blog/lgpd-o-que-e |
| dados vazados | 1,600 | /blog/vazamento-de-dados |
| clonagem de whatsapp | 1,600 | /blog/whatsapp-clonado |

### Tier 4: Programmatic long-tail (< 10 each, 5,571 pages)
| Pattern | Pages | Total potential |
|---|---|---|
| seguranca digital em {cidade} | 5,571 | ~50,000/mo combined |

---

## 3. Programmatic SEO Generator

### Tech Stack
- Python 3 + Jinja2 templates
- PostgreSQL (data source)
- Static HTML output
- nginx with clean URLs

### Directory Structure
```
seo-generator/
  generate.py          — Main generator
  blog_generator.py    — Blog post generator
  config.py            — DB config, paths
  queries.py           — SQL queries
  models.py            — Data classes
  scores.py            — Vulnerability Index
  slugify_br.py        — URL slug generation
  sample_data.py       — Sample data for testing
  templates/
    base.html          — Shared layout
    municipality.html  — City page
    state.html         — State page
    national.html      — National index
    blog_post.html     — Blog post
    blog_index.html    — Blog index
  research/
    keyword-research.md
    seo-playbook.md
    full-playbook.md   — This file
```

### Database Connection
```python
DB_CONFIG = {
    "host": "144.76.2.72",
    "port": 5432,
    "dbname": "enlace",
    "user": "enlace",
    "password": "enlace",
}
```

### Commands
```bash
# Generate all pages from real DB
python3 generate.py --clean

# Generate one state only (testing)
python3 generate.py --state pr --clean

# Generate with sample data (no DB needed)
python3 generate.py --sample-data --clean

# Generate blog posts
python3 blog_generator.py

# List blog posts
python3 blog_generator.py --list
```

### Output
```
site/seguranca-digital/           — 5,599 pages (5,571 cities + 27 states + 1 national)
site/blog/                        — 6 blog posts + index
site/sitemap-seo.xml              — 5,599 SEO URLs
site/sitemap-blog.xml             — 7 blog URLs
site/sitemap-main.xml             — 11 main pages
site/sitemap-index.xml            — Points to all sitemaps
site/robots.txt                   — All bots allowed
```

### Vulnerability Index Formula (0-100, higher = more vulnerable)
| Component | Weight | Logic |
|---|---|---|
| Speed | 25% | 0 Mbps=100, 100+ Mbps=0 |
| Mobile tech age | 20% | % on 2G/3G vs 4G/5G |
| Safety/crime | 20% | risk_score from DB |
| Infrastructure | 15% | School connectivity + sanitation |
| Provider concentration | 10% | Fewer providers = higher |
| Tower density | 10% | Fewer towers/capita = higher |

---

## 4. Tools Built

### Speed Test (/teste-de-velocidade)
- **Target:** 2.7M searches/month
- **How it works:** Downloads real binary files (1MB/5MB/10MB/25MB), measures throughput
- **Upload:** POST to nginx /speedtest/upload endpoint (pure nginx, no backend)
- **Geolocation:** GPS (browser) → IP fallback (ipapi.co)
- **Comparison:** Real state averages from Ookla/Anatel database
- **Files:** /var/www/escudovpn/speedtest/1mb.bin, 5mb.bin, 10mb.bin, 25mb.bin

### Site Scanner (/scanner)
- **What it does:** Fetches any URL, finds all third-party domains, checks against real 316,849-domain blocklist
- **API:** /scan.php?url=globo.com
- **Blocklist:** Same 4 feeds Escudo Shield uses (HaGezi, URLhaus, PhishingFilter, ThreatList)
- **Categories:** Ads, tracking, analytics, malware
- **Unique advantage:** No other VPN offers a public scanner against their real blocklist

### Breach Checker (/vazamentos)
- **Already existed** — uses Have I Been Pwned API
- **Target:** 8,100 searches/month for "vazamento de dados"

### Comparison Pages (/comparativo/)
- Escudo vs NordVPN, Surfshark, ExpressVPN
- Overview "Melhor VPN Brasil" page
- **Target:** 65,200 searches/month combined

### Ad Blocker Page (/bloqueador-de-anuncios)
- **Target:** 112,700 searches/month
- Explains DNS-level blocking vs browser extensions

---

## 5. nginx Configuration

```nginx
# Clean URLs for all static pages
location / {
    try_files $uri $uri/ $uri.html =404;
}

# Speed test upload — pure nginx, scales to millions
location = /speedtest/upload {
    client_max_body_size 10M;
    client_body_buffer_size 10M;
    add_header Access-Control-Allow-Origin '*' always;
    add_header Access-Control-Allow-Methods 'POST, OPTIONS' always;
    if ($request_method = OPTIONS) { return 204; }
    return 200 '{"status":"ok"}';
}

# Speed test downloads with no-cache
location /speedtest/ {
    add_header Access-Control-Allow-Origin '*' always;
    add_header Cache-Control 'no-store' always;
    expires -1;
}

# SEO pages caching
location /seguranca-digital/ {
    try_files $uri $uri/ $uri.html =404;
    expires 7d;
    add_header Cache-Control "public, immutable";
}
```

---

## 6. Deploy Process

```bash
# Generate all pages
cd /home/dev/pulsovpn/escudo-vpn/seo-generator
python3 generate.py --clean
python3 blog_generator.py

# Deploy to production
sudo cp -r site/seguranca-digital /var/www/escudovpn/
sudo cp -r site/blog /var/www/escudovpn/
sudo cp site/sitemap-*.xml /var/www/escudovpn/
sudo cp site/robots.txt /var/www/escudovpn/

# Update nginx if needed
sudo nginx -t && sudo systemctl reload nginx
```

---

## 7. SEO Checklist

### Per page
- [x] Title with target keyword + city/brand
- [x] Meta description with 2-3 data points
- [x] H1 with city/topic name
- [x] Canonical URL (self-referencing)
- [x] JSON-LD Article + BreadcrumbList
- [x] FAQ schema on blog posts
- [x] Internal links (neighbors, state, national, tools)
- [x] Breadcrumb navigation
- [x] Mobile responsive
- [x] Fast load (<1s, no JS framework)

### Site-wide
- [x] sitemap-index.xml with all sitemaps
- [x] robots.txt allowing all bots + Googlebot + GPTBot
- [x] Clean URLs (no .html extension)
- [ ] Google Search Console verified + sitemap submitted
- [ ] Umami analytics installed
- [ ] Cloudflare CDN (optional but recommended)

---

## 8. Content Calendar

### Blog posting: 3x per week recommended

**Already published (6 posts):**
1. O que e phishing (14,800/mo)
2. LGPD o que e (12,100/mo)
3. Vazamento de dados (8,100/mo)
4. DNS privado (5,400/mo)
5. WhatsApp clonado (3,600/mo)
6. Seguranca digital guia (3,600/mo)

**Next to write:**
7. Golpe do WhatsApp (1,000/mo)
8. Golpes na internet (880/mo)
9. Como proteger o celular (320/mo)
10. Dados pessoais LGPD (4,400/mo)
11. Protecao de dados (2,900/mo)
12. VPN gratis vs paga — comparison
13. Como funciona uma VPN — educational
14. WiFi publico e seguro? — fear-based
15. Verificar se CPF vazou — tool + guide

---

## 9. Paid Ads Strategy

### Google Ads — Start with R$3,000/mo
- Campaign 1: "teste de velocidade" → /teste-de-velocidade (CPC ~R$0.50)
- Campaign 2: "vpn gratis" → homepage (CPC ~R$2-4)
- Campaign 3: "seguranca digital" → /seguranca-digital/ (CPC ~R$1.50)

### Meta Ads — Start with R$2,000/mo after 2 weeks
- Retargeting visitors who tested speed but didn't download
- Awareness: BR 18-45, interest in tech/privacy

### Expected ROI
- CAC (ads): R$50-70 per paid subscriber
- LTV (6 months at R$14.90): R$89.40
- Break-even: ~month 5-6

---

## 10. Traffic Projections (Conservative)

| Period | Organic visits/mo | With ads | Downloads | Paid subs |
|---|---|---|---|---|
| Month 1 | 150 | 3,150 | 315 | 16 |
| Month 3 | 3,500 | 8,500 | 850 | 43 |
| Month 6 | 15,000 | 22,000 | 2,200 | 110 |
| Month 12 | 35,000 | 42,000 | 4,200 | 210/mo |

---

## 11. Replication for Other Businesses

### Requirements
1. **Unique data per geographic area** (any structured dataset works)
2. **A product/service to sell** (the pages are the funnel)
3. **$50 for DataForSEO** (keyword research)
4. **Python 3 + Jinja2 + PostgreSQL** (or any data source)
5. **A server with nginx** (static hosting)

### Steps to replicate
1. Fork the seo-generator/ directory
2. Run keyword research with DataForSEO ($1-2)
3. Identify your "teste de velocidade" — the high-volume hook tool
4. Adapt templates to your niche
5. Connect to your data source
6. Generate pages
7. Deploy + submit sitemap
8. Write 6-10 blog posts targeting mid-volume keywords
9. Build your hook tool (scanner, calculator, checker, etc.)
10. Start paid ads on the hook tool (cheapest CPC)

### Data sources that work
- Government open data (census, crime, infrastructure)
- Telecom data (coverage, speed)
- Real estate (prices, inventory)
- Health (hospitals, coverage)
- Education (schools, test scores)
- Economic (jobs, income, cost of living)
- Environment (air quality, water)

The key: **genuinely unique data per page** + **proprietary score/index** + **internal link graph** + **hook tool for traffic**.
