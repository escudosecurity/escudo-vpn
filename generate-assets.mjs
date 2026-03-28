import fs from 'fs';
import path from 'path';

const API_KEY = 'AIzaSyDVdu0S2UcgJKTjRSL4VwoFw1Ya55tyKNE';
const OUTPUT = './generated-assets';

const STYLE = `Style: premium Brazilian security brand. Deep forest green and gold palette. Luxury editorial meets cybersecurity. Think private banking meets defense tech. Color palette STRICT: deep green (#002a1c, #003322, #004d33), gold (#FFDF00, #c9a84c), white, black. NO cyan, NO purple, NO bright blue, NO neon, NO lime green. Gold must be Brazilian flag gold, warm and rich, NOT yellow, NOT brass. Green must be DEEP forest green, NOT bright, NOT lime, NOT emerald. Mood: premium, protective, technically sophisticated, quietly Brazilian. Texture: subtle grain 2-3% opacity for premium feel. Lighting: warm golden accent lighting on deep green, diffused, no harsh shadows. NO: neon glow, terminal green, bright backgrounds, stock photo feel, cartoon, 3D renders, glass morphism, purple tech colors, busy patterns. YES: negative space, editorial composition, depth of field, architectural precision, subtle gold foil texture, shield motifs, Brazilian flag diamond geometry. ABSOLUTELY NO TEXT, NO WORDS, NO LETTERS, NO NUMBERS IN THE IMAGE. PURE VISUAL ONLY.`;

const ENDPOINTS = {
  'imagen-4.0-generate-001':
    'https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict',
  'imagen-4.0-ultra-generate-001':
    'https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-ultra-generate-001:predict',
};

const ASSETS = [
  // LOGOS
  {
    name: 'logo-shield-mark',
    folder: 'logo',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '1:1',
    count: 3,
    prompt: `Minimalist shield logo mark for a cybersecurity VPN brand. Geometric shield shape with a subtle Brazilian flag diamond shape integrated inside. Deep forest green (#002a1c) background. The shield is rendered in warm gold (#FFDF00 to #c9a84c gradient). Clean, modern, works at 16px. Single monoline geometric mark, no fills except gold. Luxury brand feel like Bottega Veneta or Porsche crest. Negative space inside the shield suggests protection and a keyhole. ${STYLE}`
  },
  {
    name: 'logo-shield-filled',
    folder: 'logo',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '1:1',
    count: 3,
    prompt: `Premium filled shield logo for a Brazilian VPN security brand. Deep green (#004d33) shield shape on near-black (#002a1c) background. A gold (#FFDF00) diamond shape (Brazilian flag inspired) centered inside the shield. The diamond has subtle gold foil texture. Below the diamond a small gold curved line suggesting a horizon or globe band. Ultra clean, geometric, premium. Must be recognizable at very small sizes. ${STYLE}`
  },
  {
    name: 'logo-app-icon',
    folder: 'logo',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '1:1',
    count: 3,
    prompt: `Mobile app icon for a premium Brazilian VPN called Escudo. Rounded square with deep forest green (#002a1c to #004d33 gradient) background. Centered: a minimal gold (#FFDF00) shield silhouette with a small diamond cutout inside. Gold has subtle metallic sheen. Ultra clean, modern, luxury feel. Must be instantly recognizable at phone icon size. No text. ${STYLE}`
  },

  // HERO IMAGES
  {
    name: 'hero-abstract-shield',
    folder: 'hero',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '16:9',
    count: 2,
    prompt: `Abstract visualization of digital protection. Deep forest green (#002a1c) background. Subtle gold (#FFDF00) light particles emanating from center, forming the vague suggestion of a shield shape. Particles dissolve into the darkness at edges. Warm golden hour atmosphere. Cinematic, wide, lots of negative space on left for text overlay. Photorealistic lighting. Atmospheric depth. ${STYLE}`
  },
  {
    name: 'hero-diamond-geometry',
    folder: 'hero',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '16:9',
    count: 2,
    prompt: `Abstract geometric composition inspired by Brazilian flag diamond shape. Deep dark green (#002a1c) background. Thin gold (#c9a84c) wireframe lines forming an angular diamond pattern, dissolving into particles at the edges. Warm golden light source from behind the diamond. Minimal, architectural, premium. Large clean area on left for text overlay. Subtle film grain. ${STYLE}`
  },
  {
    name: 'hero-golden-light',
    folder: 'hero',
    model: 'imagen-4.0-ultra-generate-001',
    aspect: '16:9',
    count: 2,
    prompt: `Cinematic atmospheric shot. Deep forest green (#002a1c) darkness with a single beam of warm gold (#FFDF00) light cutting diagonally across the frame. The gold light creates subtle caustic patterns on a dark surface. Moody, protective, luxurious atmosphere. Like light through a high-security vault door. Film grain 2%. Left two-thirds clean for text overlay. ${STYLE}`
  },

  // SECTION BACKGROUNDS
  {
    name: 'section-green-texture',
    folder: 'sections',
    model: 'imagen-4.0-generate-001',
    aspect: '16:9',
    count: 2,
    prompt: `Subtle abstract texture background. Deep forest green (#002a1c to #003322) gradient. Very faint gold (#c9a84c) thread-like lines at 5-8% opacity creating a barely visible angular pattern suggesting a diamond grid. Minimal, quiet, premium texture. Must work as a background with white text on top. ${STYLE}`
  },
  {
    name: 'section-gold-accent',
    folder: 'sections',
    model: 'imagen-4.0-generate-001',
    aspect: '16:9',
    count: 2,
    prompt: `Dark background section image. Deep green (#002a1c) base. A single diagonal gold (#FFDF00 to #c9a84c) accent stripe at 15-20% opacity crossing the lower right corner. Rest is clean dark green. Subtle noise texture. Minimal, elegant, editorial. Left side completely clean for text. ${STYLE}`
  },

  // SOCIAL MEDIA
  {
    name: 'social-square',
    folder: 'social',
    model: 'imagen-4.0-generate-001',
    aspect: '1:1',
    count: 2,
    prompt: `Square social media image. Deep forest green (#002a1c) background. Center: abstract gold (#FFDF00) shield silhouette made of dissolving particles, radiating warm light. Clean, minimal, premium. Instagram/LinkedIn post format. No text. ${STYLE}`
  },

  // OG IMAGE
  {
    name: 'og-default',
    folder: 'og',
    model: 'imagen-4.0-generate-001',
    aspect: '16:9',
    count: 2,
    prompt: `Open Graph sharing image. Deep forest green (#002a1c) background. Left third: subtle gold (#c9a84c) shield mark at low opacity. Right two-thirds: clean dark green space for text overlay. A thin gold gradient line runs horizontally at 40% height. Minimal, premium, editorial. ${STYLE}`
  },
];

async function generateImage(asset, variantNum) {
  const endpoint = `${ENDPOINTS[asset.model]}?key=${API_KEY}`;

  console.log(`    Calling ${asset.model}...`);
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
    const errText = await response.text();
    console.error(`    ERROR ${response.status}: ${errText.slice(0, 300)}`);
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

  console.error(`    No image data in response`);
  return null;
}

async function main() {
  const total = ASSETS.reduce((sum, a) => sum + a.count, 0);
  console.log(`\nGenerating ${total} images across ${ASSETS.length} asset types...\n`);

  let generated = 0;
  for (let i = 0; i < ASSETS.length; i++) {
    const asset = ASSETS[i];
    console.log(`[${i + 1}/${ASSETS.length}] ${asset.folder}/${asset.name} (${asset.count} variants)`);

    for (let v = 1; v <= asset.count; v++) {
      const result = await generateImage(asset, v);
      if (result) {
        generated++;
        console.log(`    v${v}: ${result.filename} (${(result.size / 1024).toFixed(0)}KB) ✓`);
      }
      // Rate limit: free tier ~10 req/min → 7s between requests
      if (v < asset.count || i < ASSETS.length - 1) {
        console.log(`    Waiting 7s (rate limit)...`);
        await new Promise(r => setTimeout(r, 7000));
      }
    }
  }

  console.log(`\nDone! Generated ${generated}/${total} images.`);
  console.log(`Output: ${path.resolve(OUTPUT)}`);
}

main().catch(console.error);
