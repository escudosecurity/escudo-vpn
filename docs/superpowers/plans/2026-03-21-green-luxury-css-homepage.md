# Green Luxury CSS Foundation + Homepage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reskin escudovpn.com with the Green Luxury brand — new CSS foundation and homepage redesign as the first visible deliverable.

**Architecture:** Replace the existing `css/style.css` (cyan palette) with a new Green Luxury stylesheet. The site is static HTML/CSS/JS served by nginx from `/var/www/escudovpn/`. Source lives in `/home/dev/pulsovpn/escudo-vpn/site/`. No build tools — edit HTML/CSS directly, then copy to `/var/www/escudovpn/`.

**Tech Stack:** Static HTML, CSS (custom properties), vanilla JS, Google Fonts (Fraunces, Plus Jakarta Sans, JetBrains Mono), nginx

**Spec:** `docs/superpowers/specs/2026-03-21-green-luxury-brand-design.md`
**Style Reference:** `site/styles-v4.html` (approved style tile)
**Generated Assets:** `generated-assets/` (approved hero images, logos, section backgrounds)

---

## File Structure

```
site/
├── css/
│   ├── style.css          ← REPLACE (current cyan theme)
│   ├── style-old.css      ← BACKUP of current
│   └── tools.css           ← NEW: Operacional mode overrides (future Plan 2)
├── img/
│   ├── hero-*.png          ← COPY from generated-assets/hero/
│   ├── section-*.png       ← COPY from generated-assets/sections/
│   ├── logo-*.png          ← COPY from generated-assets/logo/
│   ├── og-image.jpg        ← REPLACE with generated-assets/og/
│   └── social-*.png        ← COPY from generated-assets/social/
├── js/
│   └── main.js             ← MODIFY (update animation thresholds if needed)
├── index.html              ← MODIFY (update markup for new design)
└── [other pages]           ← FUTURE plans
```

---

### Task 1: Backup Current Site & Deploy Assets

**Files:**
- Backup: `site/css/style.css` → `site/css/style-old.css`
- Copy: `generated-assets/hero/` → `site/img/`
- Copy: `generated-assets/sections/` → `site/img/`
- Copy: `generated-assets/logo/` (picked winners) → `site/img/`
- Copy: `generated-assets/og/` (picked winner) → `site/img/`
- Copy: `generated-assets/social/` (picked winner) → `site/img/`

- [ ] **Step 1: Backup current CSS**

```bash
cp site/css/style.css site/css/style-old.css
```

- [ ] **Step 2: Copy approved hero images**

```bash
# Copy the approved hero images (user picks from review page)
cp generated-assets/hero/hero-abstract-shield-v1.png site/img/
cp generated-assets/hero/hero-diamond-geometry-v1.png site/img/
cp generated-assets/hero/hero-golden-light-v1.png site/img/
```

- [ ] **Step 3: Copy approved section backgrounds**

```bash
cp generated-assets/sections/section-green-texture-v1.png site/img/
cp generated-assets/sections/section-gold-accent-v1.png site/img/
```

- [ ] **Step 4: Copy approved logo, OG, social**

```bash
# Copy the user's picked logo for web use
cp generated-assets/logo/<picked-logo>.png site/img/logo-green-luxury.png
cp generated-assets/og/<picked-og>.png site/img/og-image-new.png
cp generated-assets/social/<picked-social>.png site/img/social-square.png
```

- [ ] **Step 5: Commit backup and assets**

```bash
cd /home/dev/pulsovpn
git add -A
git commit -m "chore: backup old CSS, add Green Luxury generated assets"
```

---

### Task 2: New CSS Foundation — Custom Properties & Reset

**Files:**
- Create: `site/css/style.css` (overwrite)

- [ ] **Step 1: Write the CSS custom properties and reset**

Write the complete `:root` block with all Green Luxury tokens, reset, and base styles:

```css
/* ── Reset & Base ──────────────────────────────────── */
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }

:root {
  /* — Greens (Base) — */
  --green-900: #001a14;
  --green-800: #002a1c;
  --green-700: #003322;
  --green-600: #004d33;
  --green-500: #006847;
  --green-400: #008a5e;

  /* — Golds (Accent) — */
  --gold-500: #FFDF00;
  --gold-600: #e6c800;
  --gold-700: #c9a84c;
  --gold-glow: rgba(255, 223, 0, 0.15);

  /* — Neutrals — */
  --white: #ffffff;
  --white-60: rgba(255, 255, 255, 0.6);
  --white-35: rgba(255, 255, 255, 0.35);
  --white-15: rgba(255, 255, 255, 0.15);
  --white-06: rgba(255, 255, 255, 0.06);
  --white-12: rgba(255, 255, 255, 0.12);
  --white-04: rgba(255, 255, 255, 0.04);

  /* — Semantic — */
  --success: #00ff88;
  --danger: #ff4444;
  --warning: #ffbd2e;

  /* — Aliases (backward compat with existing HTML class names) — */
  --bg: var(--green-800);
  --bg-card: var(--white-04);
  --bg-raised: var(--green-700);
  --border: var(--white-06);
  --text: var(--white-60);
  --text-dim: var(--white-35);
  --text-bright: var(--white);
  --accent: var(--gold-500);
  --accent-dim: var(--gold-700);
  --green: var(--green-500);
  --red: var(--danger);

  /* — Layout — */
  --radius: 14px;
  --radius-sm: 8px;
  --radius-lg: 20px;
  --font: 'Plus Jakarta Sans', 'Inter', system-ui, sans-serif;
  --font-heading: 'Fraunces', 'Georgia', serif;
  --mono: 'JetBrains Mono', 'Fira Code', monospace;
}

html { scroll-behavior: smooth; font-size: 16px; }

body {
  font-family: var(--font);
  background: var(--bg);
  color: var(--text);
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
  overflow-x: hidden;
}

a { color: var(--green-400); text-decoration: none; transition: color .2s; }
a:hover { color: var(--white); }

img { max-width: 100%; display: block; }

::selection {
  background: rgba(255, 223, 0, 0.25);
  color: var(--white);
}
```

- [ ] **Step 2: Verify the file loads in browser**

```bash
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
```

Open `http://216.238.111.108/` — should see green background, gold selection highlight. Layout will be broken (expected — we're replacing incrementally).

- [ ] **Step 3: Commit foundation**

```bash
git add site/css/style.css
git commit -m "feat: Green Luxury CSS foundation — custom properties and reset"
```

---

### Task 3: Typography & Layout

**Files:**
- Modify: `site/css/style.css` (append)

- [ ] **Step 1: Add typography and layout rules**

Append to `style.css`:

```css
/* ── Typography ───────────────────────────────────── */
h1, h2, h3, h4 { color: var(--text-bright); line-height: 1.15; }
h1, h2 { font-family: var(--font-heading); font-weight: 400; }
h3, h4 { font-weight: 600; }
h1 { font-size: clamp(2.4rem, 5vw, 3.6rem); letter-spacing: -0.01em; }
h2 { font-size: clamp(1.8rem, 3.5vw, 2.6rem); }
h3 { font-size: 1.15rem; }
p { max-width: 60ch; }

h1 em, h2 em {
  font-style: italic;
  background: linear-gradient(135deg, var(--gold-500), var(--gold-700));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.label {
  font-size: 0.625rem;
  font-weight: 600;
  letter-spacing: 0.3em;
  text-transform: uppercase;
  color: var(--gold-500);
}

/* ── Layout ───────────────────────────────────────── */
.wrap {
  max-width: 1120px;
  margin: 0 auto;
  padding: 0 1.5rem;
}

section { padding: 6rem 0; }

/* ── Accent Line Divider ─────────────────────────── */
.accent-line {
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255, 223, 0, 0.3), transparent);
}
```

- [ ] **Step 2: Deploy and check typography**

```bash
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
```

Verify headings render in Fraunces serif, body in Plus Jakarta Sans.

- [ ] **Step 3: Commit**

```bash
git add site/css/style.css
git commit -m "feat: Green Luxury typography and layout"
```

---

### Task 4: Navigation

**Files:**
- Modify: `site/css/style.css` (append)
- Modify: `site/index.html` (update Google Fonts link, update nav logo)

- [ ] **Step 1: Update Google Fonts link in index.html**

Replace the existing Google Fonts `<link>` with:

```html
<link href="https://fonts.googleapis.com/css2?family=Fraunces:ital,wght@0,300;0,400;0,500;0,600;1,300;1,400&family=Plus+Jakarta+Sans:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;700&display=swap" rel="stylesheet">
```

- [ ] **Step 2: Update nav logo shield color**

In `index.html`, change the nav-logo `.shield` background from cyan to green. Update the shield div:

```html
<div class="shield" style="background: linear-gradient(135deg, var(--green-500), var(--green-400));">
```

- [ ] **Step 3: Add nav CSS**

Append to `style.css`:

```css
/* ── Nav ──────────────────────────────────────────── */
nav {
  position: fixed;
  top: 0; left: 0; right: 0;
  z-index: 100;
  background: rgba(0, 42, 28, 0.85);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border-bottom: 1px solid var(--border);
}

nav .wrap {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 64px;
}

.nav-logo {
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--text-bright);
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-family: var(--font);
}

.nav-logo .shield {
  width: 28px; height: 28px;
  background: linear-gradient(135deg, var(--green-500), var(--green-400));
  border-radius: 6px;
  display: grid;
  place-items: center;
}

.nav-logo .shield svg { width: 16px; height: 16px; }

.nav-links {
  display: flex;
  align-items: center;
  gap: 2rem;
  list-style: none;
}

.nav-links a {
  color: var(--text-dim);
  font-size: 0.9rem;
  font-weight: 500;
  transition: color .2s;
}

.nav-links a:hover { color: var(--text-bright); }

.nav-cta {
  background: linear-gradient(135deg, var(--gold-500), var(--gold-700));
  color: var(--green-800) !important;
  padding: 0.5rem 1.25rem;
  border-radius: var(--radius-sm);
  font-weight: 700;
  font-size: 0.875rem;
  transition: transform .2s, box-shadow .2s;
  box-shadow: 0 2px 12px var(--gold-glow);
}

.nav-cta:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 20px rgba(255, 223, 0, 0.25);
}

.nav-toggle { display: none; background: none; border: none; color: var(--text); cursor: pointer; }
```

- [ ] **Step 4: Deploy and verify nav**

```bash
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
sudo cp site/index.html /var/www/escudovpn/index.html
```

Nav should show green shield logo, gold "Baixar grátis" CTA.

- [ ] **Step 5: Commit**

```bash
git add site/css/style.css site/index.html
git commit -m "feat: Green Luxury navigation with gold CTA"
```

---

### Task 5: Hero Section

**Files:**
- Modify: `site/css/style.css` (append)
- Modify: `site/index.html` (update hero markup for new design)

- [ ] **Step 1: Update hero HTML in index.html**

Replace the hero section content with the Prestige mode hero:

```html
<section class="hero">
  <div class="wrap">
    <div class="label">A VPN do Brasil</div>
    <h1>Sua internet,<br>sem <em>rastreio.</em></h1>
    <p>O Escudo bloqueia anúncios, malware e rastreadores antes de chegarem ao seu dispositivo. Criptografia pós-quântica que nenhum outro VPN oferece.</p>
    <div class="hero-actions">
      <a href="#download" class="btn btn-primary">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12l7 7 7-7"/></svg>
        Baixar Escudo
      </a>
      <a href="#how" class="btn btn-ghost">Como funciona</a>
    </div>
  </div>
</section>
```

Remove the old hero-icon.jpg image and status-badge (we'll use the label tag instead).

- [ ] **Step 2: Add hero CSS with background image**

Append to `style.css`:

```css
/* ── Hero ─────────────────────────────────────────── */
.hero {
  padding: 12rem 0 6rem;
  position: relative;
  overflow: hidden;
}

.hero::before {
  content: '';
  position: absolute;
  inset: 0;
  background: url('/img/hero-abstract-shield-v1.png') center/cover no-repeat;
  opacity: 0.15;
  pointer-events: none;
}

.hero::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, var(--green-800) 0%, transparent 40%, transparent 60%, var(--green-800) 100%);
  pointer-events: none;
}

.hero .wrap { position: relative; z-index: 1; }

.hero .label { margin-bottom: 1.5rem; }

.hero h1 { margin-bottom: 1.25rem; }

.hero p {
  font-size: 1.15rem;
  color: var(--text-dim);
  margin-bottom: 2.5rem;
  max-width: 540px;
}

.hero-actions {
  display: flex;
  gap: 1rem;
  flex-wrap: wrap;
}
```

- [ ] **Step 3: Add button CSS**

```css
/* ── Buttons ─────────────────────────────────────── */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.9rem 2rem;
  border-radius: var(--radius);
  font-weight: 700;
  font-size: 0.95rem;
  border: none;
  cursor: pointer;
  transition: all .2s;
  font-family: var(--font);
}

.btn-primary {
  background: linear-gradient(135deg, var(--gold-500), var(--gold-700));
  color: var(--green-800);
  box-shadow: 0 4px 20px var(--gold-glow);
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 32px rgba(255, 223, 0, 0.25);
  color: var(--green-800);
}

.btn-ghost {
  background: transparent;
  color: var(--text);
  border: 1px solid var(--white-15);
}

.btn-ghost:hover {
  border-color: var(--white-35);
  background: var(--white-04);
  color: var(--text-bright);
}
```

- [ ] **Step 4: Deploy and verify hero**

```bash
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
sudo cp site/index.html /var/www/escudovpn/index.html
sudo cp site/img/hero-abstract-shield-v1.png /var/www/escudovpn/img/
```

Hero should show "A VPN DO BRASIL" gold label, Fraunces serif headline with gold italic "rastreio", gold CTA button, subtle hero image behind.

- [ ] **Step 5: Commit**

```bash
git add site/css/style.css site/index.html
git commit -m "feat: Green Luxury hero section with gold CTA and background"
```

---

### Task 6: Stats, Features, Steps Sections

**Files:**
- Modify: `site/css/style.css` (append)

- [ ] **Step 1: Add stats CSS**

```css
/* ── Numbers Section ──────────────────────────────── */
.numbers {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 1px;
  background: var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  margin-top: 3rem;
}

.numbers > div {
  background: var(--green-800);
  padding: 2rem 1.5rem;
  text-align: center;
}

.numbers .num {
  font-size: 2.5rem;
  font-weight: 700;
  color: var(--gold-500);
  font-family: var(--mono);
}

.numbers .num-label {
  font-size: 0.8rem;
  color: var(--text-dim);
  margin-top: 0.25rem;
}
```

- [ ] **Step 2: Add feature cards CSS**

```css
/* ── Features Grid ────────────────────────────────── */
.features-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1.25rem;
  margin-top: 3rem;
}

.feature-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 2rem;
  transition: border-color .3s, transform .3s;
}

.feature-card:hover {
  border-color: var(--white-12);
  transform: translateY(-2px);
}

.feature-icon {
  width: 40px; height: 40px;
  border-radius: var(--radius-sm);
  display: grid;
  place-items: center;
  margin-bottom: 1rem;
}

.feature-icon.shield  { background: rgba(0, 104, 71, 0.15); color: var(--green-400); }
.feature-icon.lock    { background: var(--gold-glow); color: var(--gold-500); }
.feature-icon.bolt    { background: var(--gold-glow); color: var(--gold-600); }
.feature-icon.globe   { background: rgba(0, 104, 71, 0.15); color: var(--green-400); }
.feature-icon.layers  { background: rgba(0, 39, 118, 0.15); color: #4080dd; }
.feature-icon.eye     { background: rgba(0, 104, 71, 0.15); color: var(--green-400); }

.feature-card h3 { margin-bottom: 0.5rem; }
.feature-card p { font-size: 0.9rem; color: var(--text-dim); }
```

- [ ] **Step 3: Add steps CSS**

```css
/* ── How It Works ─────────────────────────────────── */
.steps {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 2rem;
  margin-top: 3rem;
  counter-reset: step;
}

.step {
  position: relative;
  padding: 2rem;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  counter-increment: step;
}

.step::before {
  content: counter(step);
  display: block;
  width: 32px; height: 32px;
  line-height: 32px;
  text-align: center;
  background: linear-gradient(135deg, var(--gold-500), var(--gold-700));
  color: var(--green-800);
  font-weight: 700;
  font-size: 0.85rem;
  border-radius: var(--radius-sm);
  margin-bottom: 1rem;
}

.step h3 { margin-bottom: 0.5rem; }
.step p { font-size: 0.9rem; color: var(--text-dim); }
```

- [ ] **Step 4: Deploy and verify**

```bash
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
```

Stats should show gold numbers in JetBrains Mono. Feature cards should have green/gold icon boxes. Steps should have gold numbered badges.

- [ ] **Step 5: Commit**

```bash
git add site/css/style.css
git commit -m "feat: Green Luxury stats, features, and steps sections"
```

---

### Task 7: Pricing, Download, Footer

**Files:**
- Modify: `site/css/style.css` (append)

- [ ] **Step 1: Add pricing CSS**

```css
/* ── Pricing ──────────────────────────────────────── */
.pricing-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 1.5rem;
  margin-top: 3rem;
  max-width: 800px;
  margin-left: auto;
  margin-right: auto;
}

.price-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 2.5rem 2rem;
  display: flex;
  flex-direction: column;
}

.price-card.featured {
  border-color: var(--gold-700);
  border-top: 3px solid var(--gold-500);
  position: relative;
}

.price-card.featured::after {
  content: 'Popular';
  position: absolute;
  top: -14px; left: 50%;
  transform: translateX(-50%);
  background: linear-gradient(135deg, var(--gold-500), var(--gold-700));
  color: var(--green-800);
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.25rem 0.8rem;
  border-radius: 999px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.price-card h3 { margin-bottom: 0.5rem; }

.price-card .price {
  font-size: 2.5rem;
  font-weight: 700;
  color: var(--text-bright);
  font-family: var(--font-heading);
  margin: 1rem 0;
}

.price-card .price span {
  font-size: 0.9rem;
  font-weight: 400;
  color: var(--text-dim);
  font-family: var(--font);
}

.price-card ul { list-style: none; margin: 1.5rem 0; flex-grow: 1; }

.price-card li {
  padding: 0.4rem 0;
  font-size: 0.9rem;
  color: var(--text-dim);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.price-card li::before {
  content: '✓';
  color: var(--green-400);
  font-weight: 700;
}

.price-card .btn { width: 100%; justify-content: center; margin-top: auto; }
```

- [ ] **Step 2: Add download and footer CSS**

```css
/* ── Download ─────────────────────────────────────── */
.download-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1.5rem;
  margin-top: 3rem;
}

.download-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 2.5rem 2rem;
  text-align: center;
  transition: border-color .3s;
}

.download-card:hover { border-color: var(--white-12); }
.download-card .platform-icon { font-size: 2.5rem; margin-bottom: 1rem; }
.download-card h3 { margin-bottom: 0.25rem; }
.download-card .version { font-size: 0.8rem; color: var(--text-dim); font-family: var(--mono); margin-bottom: 1.5rem; }
.download-card .btn { width: 100%; justify-content: center; }
.download-card.coming-soon { opacity: 0.5; pointer-events: none; }

/* ── Footer ───────────────────────────────────────── */
footer { border-top: 1px solid var(--border); padding: 3rem 0; }
.footer-grid { display: grid; grid-template-columns: 2fr 1fr 1fr 1fr; gap: 2rem; }
.footer-brand p { font-size: 0.85rem; color: var(--text-dim); margin-top: 0.75rem; }
.footer-col h4 { font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); margin-bottom: 0.75rem; }
.footer-col ul { list-style: none; }
.footer-col li { margin-bottom: 0.4rem; }
.footer-col a { font-size: 0.875rem; color: var(--text-dim); }
.footer-col a:hover { color: var(--text-bright); }
.footer-bottom {
  margin-top: 2.5rem; padding-top: 1.5rem;
  border-top: 1px solid var(--border);
  display: flex; justify-content: space-between;
  font-size: 0.8rem; color: var(--text-dim);
}
```

- [ ] **Step 3: Deploy and verify**

```bash
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
```

Pricing: Pro card should have gold top border. Download: same glass card style. Footer: clean layout.

- [ ] **Step 4: Commit**

```bash
git add site/css/style.css
git commit -m "feat: Green Luxury pricing, download, and footer"
```

---

### Task 8: Animations & Responsive

**Files:**
- Modify: `site/css/style.css` (append)

- [ ] **Step 1: Add animations and status badge**

```css
/* ── Status Badge ─────────────────────────────────── */
.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.4rem 1rem;
  background: rgba(0, 104, 71, 0.1);
  border: 1px solid rgba(0, 104, 71, 0.25);
  border-radius: 999px;
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--green-400);
  margin-bottom: 1.5rem;
}

.status-badge .dot {
  width: 6px; height: 6px;
  background: var(--green-400);
  border-radius: 50%;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* ── Reveal Animation ─────────────────────────────── */
.reveal {
  opacity: 0;
  transform: translateY(24px);
  transition: opacity 0.6s ease, transform 0.6s ease;
}

.reveal.visible { opacity: 1; transform: none; }
```

- [ ] **Step 2: Add responsive breakpoints**

```css
/* ── Responsive ───────────────────────────────────── */
@media (max-width: 900px) {
  .features-grid, .download-grid, .steps { grid-template-columns: 1fr; }
  .pricing-grid { grid-template-columns: 1fr; max-width: 400px; }
  .numbers { grid-template-columns: repeat(2, 1fr); }
  .footer-grid { grid-template-columns: 1fr 1fr; }
}

@media (max-width: 640px) {
  section { padding: 4rem 0; }
  .hero { padding: 8rem 0 4rem; }
  .nav-links { display: none; }
  .nav-toggle { display: block; }
  .nav-links.open {
    display: flex; flex-direction: column;
    position: absolute; top: 64px; left: 0; right: 0;
    background: var(--green-800);
    border-bottom: 1px solid var(--border);
    padding: 1.5rem; gap: 1rem;
  }
  .numbers { grid-template-columns: 1fr 1fr; }
  .footer-grid { grid-template-columns: 1fr; }
  .footer-bottom { flex-direction: column; gap: 0.5rem; }
}
```

- [ ] **Step 3: Deploy and test all breakpoints**

```bash
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
```

Test at 1440px, 1024px, 768px, 375px widths.

- [ ] **Step 4: Commit**

```bash
git add site/css/style.css
git commit -m "feat: Green Luxury animations and responsive breakpoints"
```

---

### Task 9: Update index.html Meta & OG Tags

**Files:**
- Modify: `site/index.html`

- [ ] **Step 1: Update favicon to green/gold**

Replace the inline favicon SVG:

```html
<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><rect width='100' height='100' rx='20' fill='%23002a1c'/><path d='M50 20L25 35v20c0 15 10 27 25 32 15-5 25-17 25-32V35Z' fill='%23FFDF00'/></svg>">
```

- [ ] **Step 2: Update OG image reference**

```html
<meta property="og:image" content="https://escudovpn.com/img/og-image-new.png">
```

- [ ] **Step 3: Update Android platform icon fill colors in download section**

Change SVG `fill` attributes from `var(--green)` / `var(--accent)` to proper Green Luxury colors:
- Android: `fill="var(--green-400)"`
- Windows: `fill="var(--gold-500)"`

- [ ] **Step 4: Deploy complete homepage**

```bash
sudo cp site/index.html /var/www/escudovpn/index.html
sudo cp site/css/style.css /var/www/escudovpn/css/style.css
sudo cp site/img/og-image-new.png /var/www/escudovpn/img/ 2>/dev/null
```

- [ ] **Step 5: Full visual QA**

Open `http://216.238.111.108/` and verify:
- Nav: green shield, gold CTA
- Hero: gold "A VPN DO BRASIL" label, Fraunces headline, gold italic accent, gold button
- Stats: gold numbers in monospace
- Features: green/gold icon boxes, glass cards
- Pricing: Pro card has gold top border
- Download: glass cards
- Footer: clean
- Mobile: hamburger menu, stacked grid

- [ ] **Step 6: Commit final homepage**

```bash
git add site/index.html site/css/style.css
git commit -m "feat: Green Luxury homepage complete — meta, favicon, OG"
```

---

### Task 10: Deploy to Production

**Files:**
- Deploy: all files from `site/` to `/var/www/escudovpn/`

- [ ] **Step 1: Full deploy**

```bash
sudo cp -r site/css/ /var/www/escudovpn/css/
sudo cp -r site/img/ /var/www/escudovpn/img/
sudo cp site/index.html /var/www/escudovpn/index.html
```

- [ ] **Step 2: Verify live site**

```bash
curl -s -o /dev/null -w "%{http_code}" http://216.238.111.108/
```

Expected: 200

- [ ] **Step 3: Final commit**

```bash
cd /home/dev/pulsovpn
git add -A
git commit -m "feat: Green Luxury brand — CSS foundation + homepage redesign complete"
```
