# Escudo VPN — Website Design Playbook

> Adapted from the universal 10-phase playbook for Escudo VPN's **Tropicalhack** direction.
> Brazilian Bold + Hacker Brasil fusion — gold, green, navy, terminal energy.

---

## Quick Reference: The 10-Step Process

1. ~~Present 8 design directions → stakeholder picks~~ **DONE — Tropicalhack (D)**
2. ~~Lock BRAND_IDENTITY.md~~ **DONE — phase2-logo.html + phase2-spacing.html**
3. Configure Tailwind with exact color tokens
4. Set up globals.css (fonts, animations, effects)
5. Write Style Anchor for AI prompts → Generate images (Imagen 4.0)
6. Generate hero video (Veo 2)
7. Review assets → pick winners → re-generate rejects
8. Build pages (Next.js static export)
9. Page-by-page assembly
10. Polish, test mobile, ship

---

## Phase 1: Design Direction — COMPLETED

**Chosen: Tropicalhack (Option D)** — True 50/50 fusion of Brazilian Bold + Hacker Brasil

- Brazilian gradients (navy→black→green) as ambient backgrounds
- Monospace (JetBrains Mono) for data/stats, Inter for headings/body
- Gold (#FFDF00) as hero accent
- Brazilian green (#006847) replaces neon green
- Navy (#002776) for depth
- Dark cards with green/gold borders
- Terminal elements (scanlines, live counters, code blocks) woven into warm Brazilian frames
- Subtle scanline textures throughout
- "Dark mask" remix elements added
- Site and app share the same fusion aesthetic

---

## Phase 2: Brand Identity Lock — COMPLETED

### Escudo VPN Tropicalhack Palette

| Name | Hex | Role |
|------|-----|------|
| BG Deep | #07070b | Primary background |
| BG Alt | #0b0b10 | Section alternates |
| Card | #0e0e16 | Card backgrounds |
| Card Raised | #121219 | Elevated cards |
| Border | #1c1c28 | Default borders |
| Border Mid | #242434 | Mid-emphasis borders |
| Border Bright | #2e2e44 | High-emphasis borders |
| Green | #006847 | Brazilian green (primary) |
| Green Mid | #008a5e | Medium green |
| Green Bright | #00b87a | Bright green |
| Neon | #00ff88 | Terminal neon accent |
| Gold | #FFDF00 | Hero accent (Brazilian flag) |
| Gold Mid | #e6c800 | Gold hover |
| Navy | #002776 | Brazilian navy (depth) |
| Navy Mid | #003399 | Medium navy |
| Navy Bright | #0040bb | Bright navy |
| Text | #b0b8c4 | Body text |
| Text Bright | #f0f6fc | Headings |
| Red Dot | #ff5f57 | Terminal red |
| Yellow Dot | #ffbd2e | Terminal yellow |
| Green Dot | #28c840 | Terminal green |

### Color Ratios
- Dark sections: 80% dark / 12% text / 8% accent (gold or green)
- Cards: dark bg + green or gold border glow
- Terminal elements: neon green on near-black

### Typography

| Element | Font | Weight | Tracking |
|---------|------|--------|----------|
| Hero headings | Inter | 800-900 | -0.03em |
| Section headings | Inter | 700-800 | -0.02em |
| Body text | Inter | 400-500 | normal |
| Labels/captions | JetBrains Mono | 700 | +0.12em uppercase |
| Code/data/stats | JetBrains Mono | 400-500 | normal |
| Terminal elements | JetBrains Mono | 400 | normal |

### Spacing Scale (base 4px)

| Token | Value |
|-------|-------|
| --space-xs | 4px |
| --space-sm | 8px |
| --space-md | 12px |
| --space-base | 16px |
| --space-lg | 24px |
| --space-xl | 32px |
| --space-2xl | 48px |
| --space-3xl | 64px |
| --space-4xl | 96px |
| --space-5xl | 128px |

### Border Radii

| Token | Value |
|-------|-------|
| --radius-sm | 4px |
| --radius-md | 8px |
| --radius-lg | 12px |
| --radius-xl | 16px |
| --radius-full | 9999px |

### Shadows

| Token | Value |
|-------|-------|
| --shadow-card | 0 2px 12px rgba(0,0,0,0.45), 0 1px 3px rgba(0,0,0,0.3) |
| --shadow-raised | 0 4px 24px rgba(0,0,0,0.6), 0 2px 8px rgba(0,0,0,0.4) |
| --shadow-gold | 0 0 20px rgba(255,223,0,0.25), 0 0 40px rgba(255,223,0,0.1) |
| --shadow-neon | 0 0 16px rgba(0,255,136,0.3), 0 0 32px rgba(0,255,136,0.12) |

---

## Phase 3: Tailwind Config (TODO)

```typescript
import type { Config } from "tailwindcss";

export default {
  content: ["./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: { DEFAULT: "#07070b", alt: "#0b0b10" },
        card: { DEFAULT: "#0e0e16", raised: "#121219" },
        border: { DEFAULT: "#1c1c28", mid: "#242434", bright: "#2e2e44" },
        green: {
          DEFAULT: "#006847",
          mid: "#008a5e",
          bright: "#00b87a",
          neon: "#00ff88",
        },
        gold: {
          DEFAULT: "#FFDF00",
          mid: "#e6c800",
          dim: "rgba(255,223,0,0.15)",
        },
        navy: {
          DEFAULT: "#002776",
          mid: "#003399",
          bright: "#0040bb",
        },
        text: {
          DEFAULT: "#b0b8c4",
          dim: "#5a6270",
          muted: "#3e4450",
          bright: "#f0f6fc",
          white: "#ffffff",
        },
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "-apple-system", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
    },
  },
  plugins: [],
} satisfies Config;
```

---

## Phase 4: Typography & CSS Foundation (TODO)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&family=JetBrains+Mono:wght@400;500;600;700&display=swap');

html { scroll-behavior: smooth; }
body { background: #07070b; color: #b0b8c4; }

::selection {
  background: rgba(255,223,0,0.25);
  color: #f0f6fc;
}

/* Scanline texture overlay */
.scanlines::after {
  content: "";
  position: fixed;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(0,255,136,0.03) 2px,
    rgba(0,255,136,0.03) 4px
  );
  pointer-events: none;
  z-index: 9999;
}

/* Gold shimmer text */
.gradient-text-gold {
  background: linear-gradient(135deg, #FFDF00, #e6c800, #FFDF00);
  background-size: 200% auto;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  animation: shimmer 8s ease-in-out infinite;
}

/* Green glow text */
.gradient-text-green {
  background: linear-gradient(135deg, #00ff88, #00b87a, #00ff88);
  background-size: 200% auto;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  animation: shimmer 8s ease-in-out infinite;
}

@keyframes shimmer {
  0%, 100% { background-position: 0% center; }
  50% { background-position: 200% center; }
}

@keyframes fadeInUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fade-in { animation: fadeInUp 0.6s ease-out both; }

/* Terminal-style accent line */
.line-accent {
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(0,255,136,0.3), rgba(255,223,0,0.2), transparent);
}

.section-gap { padding: 80px 0; }
@media (min-width: 1024px) { .section-gap { padding: 120px 0; } }
```

---

## Phase 5: AI Image Generation (Google AI Studio API)

### Setup

1. Get API Key: https://aistudio.google.com/apikey
2. Free tier: ~10 requests/minute, Imagen 4.0 + Imagen 4.0 Ultra
3. Aspect ratios: 1:1, 9:16, 16:9, 4:3, 3:4

### Escudo VPN Style Anchor

> Append to EVERY prompt:

```
Style: Brazilian cyberpunk meets hacker terminal. Dark premium, technical, patriotic pride.
Color palette STRICT — only these colors: deep black (#07070b), Brazilian green (#006847),
gold (#FFDF00), navy (#002776), neon green (#00ff88) for terminal accents, white (#f0f6fc) for text.
NO cyan, NO purple, NO blue tech gradients, NO light backgrounds.
Gold must be Brazilian flag gold (#FFDF00), NOT matte gold, NOT brass, NOT orange.
Green must be deep Brazilian green (#006847), NOT lime, NOT emerald.
Typography feel: JetBrains Mono monospace for data elements, Inter bold for headings.
Mood: dark, protective, technically credible, Brazilian national pride, hacker energy.
Texture: subtle scanline overlay 2-3% opacity, terminal phosphor glow on green elements.
Lighting: dark ambient with gold and green accent lighting, no harsh shadows.
NO: light backgrounds, corporate clip-art, stock photo feel, 3D renders, glass morphism,
cute illustrations, cartoon style, flat design, Material Design colors.
YES: dark depth, terminal UI elements, Brazilian flag color accents, code/data visualization,
shield motifs, negative space, monospace typography, circuit board patterns, scanline textures.
ABSOLUTELY NO TEXT, NO WORDS, NO LETTERS, NO NUMBERS IN THE IMAGE. PURE VISUAL ONLY.
```

### Generation Script

```javascript
// generate-assets.mjs
import fs from 'fs';
import path from 'path';

const API_KEY = 'YOUR_GEMINI_KEY_HERE'; // from aistudio.google.com/apikey
const OUTPUT = './generated-assets';

const STYLE = `Style: Brazilian cyberpunk meets hacker terminal... [FULL ANCHOR ABOVE]`;

const ENDPOINTS = {
  'imagen-4.0-generate-001':
    'https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict',
  'imagen-4.0-ultra-generate-001':
    'https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-ultra-generate-001:predict',
};

const ASSETS = [
  {
    name: 'logo-tropicalhack',
    folder: 'logo',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '1:1',
    count: 4,
    prompt: `Shield logo for cybersecurity VPN app called Escudo. Dark background. Shield shape with Brazilian flag green diamond inside. Gold accent glow. Terminal/hacker aesthetic. Monoline geometric style. Must work at 16px. ${STYLE}`
  },
  {
    name: 'hero-terminal',
    folder: 'hero',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '16:9',
    count: 3,
    prompt: `Dark terminal interface showing network protection data. Brazilian green and gold accents on deep black. Scanline texture overlay. Data visualization showing blocked threats. Shield shape subtly visible in the data flow. Cinematic wide shot. ${STYLE}`
  },
  {
    name: 'section-shield',
    folder: 'sections',
    model: 'imagen-4.0-generate-001',
    aspect: '16:9',
    count: 3,
    prompt: `Abstract dark background with subtle shield wireframe in Brazilian green. Gold particle accents. Circuit board pattern fading into darkness. Left side clean for text overlay. ${STYLE}`
  },
  {
    name: 'social-square',
    folder: 'social',
    model: 'imagen-4.0-generate-001',
    aspect: '1:1',
    count: 2,
    prompt: `Dark square composition. Center: abstract shield shape made of green circuit lines on black. Gold glow emanating from center. Terminal scanlines. ${STYLE}`
  },
  {
    name: 'og-default',
    folder: 'og',
    model: 'imagen-4.0-generate-001',
    aspect: '16:9',
    count: 2,
    prompt: `Wide dark banner. Left third: abstract green shield shape with gold accent. Right two-thirds: dark with subtle terminal grid. Brazilian green and gold only. ${STYLE}`
  },
];

async function generateImage(asset, variantNum) {
  const endpoint = `${ENDPOINTS[asset.model]}?key=${API_KEY}`;
  const response = await fetch(endpoint, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      instances: [{ prompt: asset.prompt }],
      parameters: {
        sampleCount: 1,
        aspectRatio: asset.aspect,
        personGeneration: 'DONT_ALLOW'
      }
    })
  });
  if (!response.ok) {
    console.error(`  ERROR ${response.status}: ${(await response.text()).slice(0, 200)}`);
    return null;
  }
  const data = await response.json();
  if (data.predictions?.[0]?.bytesBase64Encoded) {
    const buffer = Buffer.from(data.predictions[0].bytesBase64Encoded, 'base64');
    const filename = `${asset.name}-v${variantNum}.png`;
    const dir = path.join(OUTPUT, asset.folder);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, filename), buffer);
    return { filename, size: buffer.length };
  }
  return null;
}

async function main() {
  console.log(`Generating ${ASSETS.length} asset types...\n`);
  for (let i = 0; i < ASSETS.length; i++) {
    const asset = ASSETS[i];
    console.log(`[${i + 1}/${ASSETS.length}] ${asset.folder}/${asset.name}`);
    for (let v = 1; v <= asset.count; v++) {
      const result = await generateImage(asset, v);
      if (result) console.log(`  v${v}: ${result.filename} (${(result.size / 1024).toFixed(0)}KB)`);
      await new Promise(r => setTimeout(r, 7000)); // rate limit
    }
  }
}

main().catch(console.error);
```

Run: `node generate-assets.mjs`

---

## Phase 6: Hero Video (Veo 2) (TODO)

Prompt for Escudo:
```
Dark terminal interface slowly revealing a glowing green shield shape.
Gold particles coalesce around the shield. Subtle scanline overlay.
Camera slowly pushing in. Brazilian green (#006847) and gold (#FFDF00) accent lighting.
Deep black background. Circuit board patterns fade in and out.
Cinematic. 24fps. Film grain. No text overlays. 8 seconds.
```

---

## Phase 7-10: Build & Ship

See universal playbook phases 7-10. Adapt all components to Tropicalhack palette.

Key differences from RASTRO:
- DARK MODE ONLY (no light sections)
- Terminal-style UI elements (window chrome with red/yellow/green dots)
- Scanline texture overlay on all sections
- Gold + green accents, never cyan
- Monospace for all data/stats
- Brazilian flag diamond motif as recurring design element

---

## Appendix: Escudo-Specific Prompt Tips

### What Works
- Shield + circuit patterns
- Brazilian flag color accents (green diamond, gold glow, navy depth)
- Terminal/hacker UI overlays
- Dark ambient with point lighting
- Data visualization / threat maps

### What Fails
- Light backgrounds (fights the brand)
- Cyan/blue tech colors (that's the OLD palette)
- Realistic flag imagery (looks cheap)
- Too many colors at once (stick to green + gold + navy on black)
- Text in images (AI hallucinates Portuguese badly)
