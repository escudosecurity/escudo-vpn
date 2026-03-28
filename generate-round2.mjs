import fs from 'fs';
import path from 'path';

const API_KEY = 'AIzaSyDVdu0S2UcgJKTjRSL4VwoFw1Ya55tyKNE';
const OUTPUT = './generated-assets';

const STYLE = `Style: premium Brazilian security brand. Deep forest green and gold palette. Luxury editorial meets cybersecurity. Color palette STRICT: deep green (#002a1c, #003322, #004d33), gold (#FFDF00, #c9a84c), white, black. NO cyan, NO purple, NO bright blue, NO neon, NO lime green. Gold must be Brazilian flag gold, warm and rich. Green must be DEEP forest green. Mood: premium, protective, technically sophisticated. NO: neon glow, terminal green, stock photo feel, cartoon, 3D renders, glass morphism. YES: negative space, editorial composition, metallic gold texture, shield motifs. ABSOLUTELY NO TEXT, NO WORDS, NO LETTERS, NO NUMBERS IN THE IMAGE. PURE VISUAL ONLY.`;

const ULTRA = 'https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-ultra-generate-001:predict';

const ASSETS = [
  // APP ICONS — completely different approaches
  {
    name: 'app-icon-minimal-shield',
    folder: 'logo',
    prompt: `App icon design. Rounded square shape. Background is a rich deep green to black gradient (#004d33 to #001a14). In the center, a very simple geometric shield outline in gold (#FFDF00), just 2-3 clean strokes forming a shield silhouette. Nothing else. Ultra minimal. Like the Apple or Tesla logo level of simplicity. The gold has a subtle metallic warmth. ${STYLE}`
  },
  {
    name: 'app-icon-diamond-gold',
    folder: 'logo',
    prompt: `App icon design. Rounded square shape. Solid deep green (#002a1c) background. Center: a small gold (#FFDF00) diamond shape (like the Brazilian flag diamond) with a subtle metallic gold foil finish. Nothing else — just the diamond on green. Extremely clean and premium. The diamond has very slight 3D depth from lighting. Think luxury fashion brand app icon. ${STYLE}`
  },
  {
    name: 'app-icon-shield-cutout',
    folder: 'logo',
    prompt: `App icon design. Rounded square shape. Background: rich gold (#FFDF00 to #c9a84c) metallic gradient. In the center, a shield shape CUT OUT revealing deep green (#002a1c) underneath. Like a gold surface with a shield-shaped window. Clean, bold, instantly recognizable at small size. Premium metallic texture on the gold. ${STYLE}`
  },
  {
    name: 'app-icon-green-shield-gold-border',
    folder: 'logo',
    prompt: `App icon design. Rounded square shape. Deep dark green (#001a14) background. A bold shield shape in slightly lighter green (#004d33) centered, with a thin gold (#FFDF00) outline around the shield edge only. Inside the shield, a tiny gold diamond dot at center. Minimal, clean, elegant. Like a luxury watch brand icon. ${STYLE}`
  },
  {
    name: 'app-icon-split',
    folder: 'logo',
    prompt: `App icon design. Rounded square shape. The icon is split diagonally — top-left half is deep green (#002a1c), bottom-right half is gold (#FFDF00 to #c9a84c metallic). A small simple shield silhouette sits at the center, crossing both halves — green on the gold side, gold on the green side. Bold, graphic, modern. ${STYLE}`
  },

  // MORE LOGO VARIATIONS
  {
    name: 'logo-mark-v4-keyhole',
    folder: 'logo',
    prompt: `Logo mark for a premium VPN security brand. Deep green (#002a1c) background. A geometric shield shape in gold (#FFDF00) with a keyhole negative space cut into the center — the keyhole suggests both security and privacy. The shield has clean angular lines. Metallic gold finish. Minimal, works at 16px. ${STYLE}`
  },
  {
    name: 'logo-mark-v5-diamond-shield',
    folder: 'logo',
    prompt: `Logo mark. Deep green (#002a1c) background. A shield shape where the top half is rendered in gold (#FFDF00) wireframe lines, and the bottom half has a solid Brazilian flag diamond shape in gold. The diamond sits inside the shield like a badge. Clean geometric construction. Premium and minimal. ${STYLE}`
  },
  {
    name: 'logo-mark-v6-abstract-e',
    folder: 'logo',
    prompt: `Logo mark. Deep green (#002a1c) background. An abstract monogram that combines the letter E shape with a shield silhouette — the left vertical of the E forms the shield's left edge, the three horizontal bars of the E create internal structure. All in gold (#FFDF00) with clean geometric lines. Luxury brand quality. ${STYLE}`
  },
  {
    name: 'logo-mark-v7-crest',
    folder: 'logo',
    prompt: `Luxury crest logo mark. Deep green (#002a1c) background. A shield shape with a thin gold (#c9a84c) border. Inside: a centered gold diamond (Brazilian flag reference) with two thin gold laurel-like lines curving on each side. Extremely refined, like a coat of arms simplified to its essence. No text. Premium, heraldic but modern. ${STYLE}`
  },
  {
    name: 'logo-mark-v8-negative',
    folder: 'logo',
    prompt: `Logo mark using negative space. Deep green (#002a1c) background. A solid gold (#FFDF00) circle, with a shield shape cut out of the center as negative space revealing the green background. The shield cutout is simple and bold. Around the circle, a thin green gap, then the green background. Like a gold seal or coin with a shield impression. ${STYLE}`
  },
];

async function generateImage(asset, variantNum) {
  const endpoint = `${ULTRA}?key=${API_KEY}`;
  console.log(`    Calling Ultra...`);
  const response = await fetch(endpoint, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      instances: [{ prompt: asset.prompt }],
      parameters: { sampleCount: 1, aspectRatio: '1:1', personGeneration: 'DONT_ALLOW' }
    })
  });
  if (!response.ok) {
    console.error(`    ERROR ${response.status}: ${(await response.text()).slice(0, 300)}`);
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
  console.log(`\nRound 2: ${ASSETS.length} new images (5 app icons + 5 logos)...\n`);
  let ok = 0;
  for (let i = 0; i < ASSETS.length; i++) {
    const a = ASSETS[i];
    console.log(`[${i+1}/${ASSETS.length}] ${a.folder}/${a.name}`);
    // Generate 2 variants each
    for (let v = 1; v <= 2; v++) {
      const r = await generateImage(a, v);
      if (r) { ok++; console.log(`    v${v}: ${r.filename} (${(r.size/1024).toFixed(0)}KB) ✓`); }
      if (v < 2 || i < ASSETS.length - 1) { console.log(`    Waiting 7s...`); await new Promise(r => setTimeout(r, 7000)); }
    }
  }
  console.log(`\nDone! ${ok}/20 images generated.`);
}
main().catch(console.error);
