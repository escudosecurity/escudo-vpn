# Escudo VPN — Pearl & Precision Site Redesign

> Approved 2026-03-22. Replaces Green Luxury Unified direction.

---

## Design Direction: Pearl & Precision

White base, gold gradient accents, dark data blocks for contrast. Customer-focused language — no jargon, no emojis. Shows what Escudo SOLVES, not how it works internally.

Green logo mark preserved in nav and footer.

---

## Color Palette

### Base
| Name | Value | Role |
|------|-------|------|
| White | #ffffff | Primary background |
| Off-white | #fafaf7 | Alternate section background |
| Dark | #111111 | Text, dark sections, footer |

### Gold (Accent)
| Name | Value | Role |
|------|-------|------|
| Gold bright | #c9a84c | Gradient start, primary accent |
| Gold deep | #8b6914 | Gradient end |
| Gold gradient | linear-gradient(135deg, #c9a84c, #8b6914) | CTAs, numbers, highlights |

### Brand Green (Logo only)
| Name | Value | Role |
|------|-------|------|
| Escudo green | #003322 | Logo mark background only |

### Semantic
| Name | Value | Role |
|------|-------|------|
| Success | #00884a | "Com Escudo" states, checkmarks |
| Danger | #cc3333 | "Sem Escudo" states |
| Status green | #00cc6a | Status dot (operational) |

### Text hierarchy
| Name | Value | Role |
|------|-------|------|
| Heading | #111111 | Headlines, strong text |
| Body | rgba(17,17,17,0.45) | Paragraphs |
| Muted | rgba(17,17,17,0.35) | Nav links, secondary |
| Faint | rgba(17,17,17,0.2) | Dividers, labels |

### Dark section (data block, footer)
| Name | Value | Role |
|------|-------|------|
| Dark bg | #111111 | Section background |
| Dark text | rgba(255,255,255,0.35) | Body text on dark |
| Dark muted | rgba(255,255,255,0.15) | Secondary on dark |
| Card bg | rgba(255,255,255,0.04) | Cards on dark |
| Card border | rgba(255,255,255,0.06) | Card borders on dark |

---

## Typography

### Font Stack
- **Headings:** Inter, weight 700-800
- **Body:** Inter, weight 400-500
- **Data/Stats:** JetBrains Mono, weight 700 (gold gradient text in dark blocks only)
- **Fallback:** system-ui, sans-serif

### Scale
| Element | Size | Weight | Notes |
|---------|------|--------|-------|
| Hero headline | 56px | 800 | letter-spacing: -2.5px |
| Section title | 36px | 800 | letter-spacing: -1.5px |
| Card title | 16px | 700 | |
| Body | 13-15px | 400-500 | color: rgba(17,17,17,0.45) |
| Section label | 11px | 700 | uppercase, letter-spacing: 0.15em, gold color |
| Nav link | 13px | 500 | color: rgba(17,17,17,0.4) |
| Stat number | 36px | 700 | JetBrains Mono, gold gradient |
| Badge | 11px | 600 | pill shape, #f5f5f0 bg |

---

## Component Styles

### Buttons
- **Primary:** pill (border-radius: 100px), gold gradient bg, white text, 15px/36px padding
- **Ghost:** pill, transparent bg, 1px border rgba(17,17,17,0.1), muted text

### Cards (light sections)
- White bg, 1px border rgba(0,0,0,0.05), border-radius: 14px, 32px padding

### Cards (dark section)
- rgba(255,255,255,0.04) bg, 1px border rgba(255,255,255,0.06), border-radius: 14px

### Nav
- Sticky, white bg with backdrop-filter: blur(20px), 1px bottom border
- Green shield logo mark (32x32, #003322 bg, gold shield SVG)
- Gold gradient pill CTA

### Section dividers
- Thin line with centered uppercase label text

### Badge (hero)
- Pill shape, #f5f5f0 bg, green dot + text

---

## Page Sections (Homepage)

1. **Nav** — Logo (green mark) + links + gold CTA pill
2. **Hero** — Badge, headline ("Assista o que quiser. De qualquer lugar."), subtitle (customer problem), two CTAs. No hero image for launch.
3. **Problems** — "Tres problemas que todo brasileiro conhece." Before/after cards showing Sem Escudo vs Com Escudo.
4. **Data Block** — Dark (#111) section with gold JetBrains Mono numbers. 50+ servers, ISP residential IPs, 0 logs, Rust.
5. **How It Works** — 3 steps: Baixe, Escolha, Assista. Gold numbered circles.
6. **Pricing** — 4 plans (Gratis/Escudo/Pro/IP Dedicado). Pro highlighted with gold border.
7. **Comparison** — Table vs NordVPN and Surfshark. Escudo column highlighted.
8. **Footer** — Dark bg, green logo mark, product/tools/company links.

---

## Language Rules

- All content in Brazilian Portuguese
- No emojis anywhere
- No technical jargon (no "WireGuard", "ChaCha20", "nftables" in customer-facing copy)
- Focus on what it SOLVES: streaming, privacy, ad blocking
- Tech details only in the data block and comparison table
- "Rust" mentioned only as credibility signal, not explained

---

## What Changes From Current Site

| Element | Current (Green Luxury) | New (Pearl & Precision) |
|---------|----------------------|----------------------|
| Background | Deep green (#002a1c) | White (#ffffff) |
| Text | White on dark | Dark on white |
| Accent | Gold on green | Gold gradient on white |
| Headings | Fraunces serif | Inter 800 sans-serif |
| Body font | Plus Jakarta Sans | Inter 400-500 |
| Cards | Glass (rgba white on dark) | White with subtle border |
| Buttons | Gold gradient on dark | Gold gradient pills on white |
| Logo | Gold on green | Green mark + dark text |
| Hero | Video + green bg | Text-only, clean white |
| Tone | Luxury/editorial | Clean/modern/problem-solving |

---

## Files to Modify

- `site/css/style.css` — Complete rewrite of color variables and component styles
- `site/index.html` — Restructure sections, new copy, remove video hero
- Existing tool pages (teste-de-velocidade, meu-ip, etc.) — Apply new color scheme

## Files NOT Changed
- `site/img/` — Keep existing assets for now
- `site/js/` — Keep existing scripts
- Blog content — Keep as-is, restyle with new CSS
