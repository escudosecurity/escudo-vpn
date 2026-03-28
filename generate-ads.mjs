import fs from 'fs';
import path from 'path';

const API_KEY = 'AIzaSyDVdu0S2UcgJKTjRSL4VwoFw1Ya55tyKNE';
const OUTPUT = './generated-assets/ads';
const ULTRA = 'https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-ultra-generate-001:predict';
const STD = 'https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict';

const STYLE = `Deep forest green (#002a1c) background with warm gold (#FFDF00) accents. Premium, luxurious, clean. Shield motif. Brazilian VPN brand. NO text, NO words, NO letters, NO numbers. Pure visual only. NO neon, NO cyan, NO purple.`;

const ASSETS = [
  // Square 1:1 (1200x1200) — for display ads
  { name: 'ad-square-shield', aspect: '1:1', model: ULTRA,
    prompt: `Square ad image. Dark green background. Center: abstract gold shield shape glowing warmly, with subtle gold particles around it. Clean, premium, lots of negative space. ${STYLE}` },
  { name: 'ad-square-protection', aspect: '1:1', model: STD,
    prompt: `Square ad image. Dark green gradient background. A golden dome of light protecting a small device silhouette at center. Warm, safe feeling. Premium luxury brand. ${STYLE}` },
  // Landscape 16:9 (1200x628) — for responsive display ads
  { name: 'ad-landscape-shield', aspect: '16:9', model: ULTRA,
    prompt: `Wide landscape ad banner. Deep green background. Right side: abstract golden shield made of light particles. Left side: clean space. Warm gold glow. Premium VPN brand feel. ${STYLE}` },
  { name: 'ad-landscape-network', aspect: '16:9', model: STD,
    prompt: `Wide landscape ad. Deep dark green background. Subtle network of thin gold lines connecting dots across the frame, suggesting protection and connectivity. Minimal, premium. ${STYLE}` },
  // Portrait 9:16 — for mobile display ads
  { name: 'ad-portrait-shield', aspect: '9:16', model: STD,
    prompt: `Tall portrait ad image. Deep green background. Gold shield silhouette at top third, glowing warmly. Lower two-thirds clean dark green space. Premium, minimal. ${STYLE}` },
];

fs.mkdirSync(OUTPUT, { recursive: true });

async function gen(asset, v) {
  const r = await fetch(`${asset.model}?key=${API_KEY}`, {
    method: 'POST', headers: {'Content-Type':'application/json'},
    body: JSON.stringify({ instances: [{prompt: asset.prompt}], parameters: {sampleCount:1, aspectRatio:asset.aspect, personGeneration:'DONT_ALLOW'} })
  });
  if (!r.ok) { console.log(`  ERR ${r.status}`); return null; }
  const d = await r.json();
  if (d.predictions?.[0]?.bytesBase64Encoded) {
    const buf = Buffer.from(d.predictions[0].bytesBase64Encoded, 'base64');
    const fn = `${asset.name}-v${v}.png`;
    fs.writeFileSync(path.join(OUTPUT, fn), buf);
    return { fn, kb: (buf.length/1024).toFixed(0) };
  }
  return null;
}

async function main() {
  console.log('Generating ad images...\n');
  for (let i=0; i<ASSETS.length; i++) {
    const a = ASSETS[i];
    console.log(`[${i+1}/${ASSETS.length}] ${a.name} (${a.aspect})`);
    for (let v=1; v<=2; v++) {
      const r = await gen(a, v);
      if (r) console.log(`  v${v}: ${r.fn} (${r.kb}KB)`);
      await new Promise(r=>setTimeout(r,7000));
    }
  }
  console.log('\nDone!');
}
main().catch(console.error);
