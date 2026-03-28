# Escudo VPN — Green Luxury Brand Design Spec

> Approved 2026-03-21. One unified brand, three intensity modes.

---

## Design Direction: Green Luxury Unified

Brazilian flag colors done premium — deep forest green + gold on dark. Not kitsch patriotism, not hacker cosplay. A luxury security brand that happens to be proudly Brazilian.

Reference tile: `styles-v4.html`

---

## Color Palette

### Greens (Base)
| Name | Hex | Role |
|------|-----|------|
| Green 900 | #001a14 | Deepest shadow |
| Green 800 | #002a1c | Primary background |
| Green 700 | #003322 | Section alternates |
| Green 600 | #004d33 | Card hover, highlights |
| Green 500 | #006847 | Brazilian green (accents) |
| Green 400 | #008a5e | Links, active states |

### Golds (Accent)
| Name | Hex | Role |
|------|-----|------|
| Gold 500 | #FFDF00 | Primary accent (bright) |
| Gold 600 | #e6c800 | Hover state |
| Gold 700 | #c9a84c | Matte gold (subtle uses) |
| Gold glow | rgba(255,223,0,0.15) | Glow/shadow effects |

### Neutrals
| Name | Hex | Role |
|------|-----|------|
| White | #ffffff | Headings, emphasis |
| White 60 | rgba(255,255,255,0.6) | Body text |
| White 35 | rgba(255,255,255,0.35) | Secondary/muted text |
| White 15 | rgba(255,255,255,0.15) | Disabled text |
| White 06 | rgba(255,255,255,0.06) | Card borders |
| White 12 | rgba(255,255,255,0.12) | Hover borders |
| White 04 | rgba(255,255,255,0.04) | Card fill |

### Semantic
| Name | Hex | Role |
|------|-----|------|
| Success | #00ff88 | Secure/connected (tool pages) |
| Danger | #ff4444 | Threats/exposed |
| Warning | #ffbd2e | Caution states |

---

## Typography

### Font Stack
- **Headings:** Fraunces (serif), weight 300-600, italic for accent words
- **Body:** Plus Jakarta Sans, weight 300-600
- **Data/Stats:** JetBrains Mono, weight 400-700
- **Fallback:** Inter, system-ui, sans-serif

### Scale
| Element | Font | Size | Weight | Tracking | Notes |
|---------|------|------|--------|----------|-------|
| Hero headline | Fraunces | 48-64px | 400 | -0.01em | Gold italic on accent word |
| Section heading | Fraunces | 28-36px | 400 | normal | White |
| Card title | Plus Jakarta Sans | 15-17px | 600 | normal | White |
| Body | Plus Jakarta Sans | 14-15px | 300-400 | 0.2px | White 60 |
| Tag/label | Plus Jakarta Sans | 10px | 600 | 0.3em | Gold, uppercase |
| Stat number | JetBrains Mono | 28-48px | 700 | normal | Gold |
| Stat label | Plus Jakarta Sans | 10-11px | 500 | 0.15em | White 35, uppercase |
| Data readout | JetBrains Mono | 14-20px | 500-700 | normal | Gold or White |
| Nav link | Plus Jakarta Sans | 13px | 500 | normal | White 60 |
| Button | Plus Jakarta Sans | 12-13px | 600-700 | 0.05-0.1em | Uppercase on primary |

---

## Component Styles

### Cards
```
background: rgba(255,255,255,0.04)
border: 1px solid rgba(255,255,255,0.06)
border-radius: 14px
padding: 24px
hover: border-color rgba(255,255,255,0.12)
```

### Buttons
**Primary (Gold):**
```
background: linear-gradient(135deg, #FFDF00, #c9a84c)
color: #002a1c
padding: 15px 36px
border-radius: 10px
font-weight: 700
box-shadow: 0 4px 20px rgba(255,223,0,0.15)
```

**Ghost:**
```
background: transparent
border: 1px solid rgba(255,255,255,0.15)
color: rgba(255,255,255,0.6)
padding: 15px 36px
border-radius: 10px
```

### Accent Line (section divider)
```
height: 1px
background: linear-gradient(90deg, transparent, rgba(255,223,0,0.3), transparent)
```

---

## Three Intensity Modes

### 1. Prestige — Homepage & Marketing
- Background: #002a1c gradient
- Fraunces dominant for headings
- Gold used sparingly: hero accent word, CTA, stat numbers
- Generous whitespace, editorial pacing
- Feature cards: green icon box + white title + muted body
- Pricing: glass cards, Pro has gold top border

### 2. Operacional — Tool Pages
- SAME green background (not black)
- JetBrains Mono more prominent for data
- Gold for live data values and results
- Cards slightly brighter borders (0.12 opacity)
- Subtle scanline overlay: repeating-linear-gradient(0deg, transparent, transparent 3px, rgba(0,255,136,0.012) 3px, rgba(0,255,136,0.012) 4px)
- Input fields: same glass card style, gold border on focus
- Result badges: green dot + "SEGURO" / red dot + "EXPOSTO"

### 3. Compacto — Mobile App
- Dark variant: #0a0a0a base (Black Gold)
- Gold shield glow circle for connection status
- Same card glass style at mobile scale
- Same gold buttons, touch-sized
- Fraunces for status labels, JetBrains Mono for stats
- Bottom nav or minimal navigation

---

## Imagery Style Anchor (for Imagen 4.0)

```
Style: premium Brazilian security brand. Deep forest green (#002a1c) and gold (#FFDF00) palette.
Luxury editorial meets cybersecurity. Think private banking meets defense tech.
Color palette STRICT: deep green (#002a1c, #003322, #004d33), gold (#FFDF00, #c9a84c),
white, black. NO cyan, NO purple, NO bright blue, NO neon, NO lime green.
Gold must be Brazilian flag gold (#FFDF00), warm and rich, NOT yellow, NOT brass.
Green must be DEEP forest green, NOT bright, NOT lime, NOT emerald.
Typography feel: elegant serif (Fraunces) for headlines, clean sans-serif for body.
Mood: premium, protective, technically sophisticated, quietly Brazilian.
Texture: subtle grain 2-3% opacity for premium feel.
Lighting: warm golden accent lighting on deep green, diffused, no harsh shadows.
NO: neon glow, terminal green, bright backgrounds, stock photo feel, cartoon,
3D renders, glass morphism, purple/blue tech colors, busy patterns.
YES: negative space, editorial composition, depth of field, architectural precision,
subtle gold foil texture, shield motifs, Brazilian flag diamond geometry.
ABSOLUTELY NO TEXT, NO WORDS, NO LETTERS, NO NUMBERS IN THE IMAGE. PURE VISUAL ONLY.
```

---

## Asset Generation Plan

### Phase 1: Logo (Imagen 4.0 Ultra)
- Shield mark: geometric shield with Brazilian flag diamond, deep green + gold
- Wordmark: "ESCUDO" in Fraunces or custom serif
- Icon variants: 512px, 192px, 32px, 16px
- Must work on both green and dark backgrounds

### Phase 2: Hero Images (Imagen 4.0 Ultra)
- Abstract shield visualization on deep green
- Gold light particles / golden hour lighting
- Brazilian flag diamond geometry as abstract pattern

### Phase 3: Section Backgrounds (Imagen 4.0)
- Abstract green textures with gold accents
- Subtle patterns for section variety
- Clean areas for text overlay

### Phase 4: Social / OG (Imagen 4.0)
- Square format for social
- 16:9 for OG/sharing
- Brand-consistent compositions

### Phase 5: App Assets
- Launcher icon (all densities)
- Feature graphic (Play Store)
- Screenshots with branded frames

---

## Pages to Redesign

### Prestige Mode
- index.html (homepage)
- comparativo.html + sub-pages
- privacy.html, termos.html
- blog/ pages

### Operacional Mode
- teste-de-velocidade.html
- scanner.html
- meu-ip.html
- bloqueador-de-anuncios.html
- teste-de-privacidade.html
- vazamentos.html
- verificar-senha.html
- verificar-link.html

### Compacto Mode
- Android app redesign
- (Future: iOS, Windows)
