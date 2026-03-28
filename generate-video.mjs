import fs from 'fs';

const API_KEY = 'AIzaSyDVdu0S2UcgJKTjRSL4VwoFw1Ya55tyKNE';

async function generateVideo() {
  const endpoint = `https://generativelanguage.googleapis.com/v1beta/models/veo-2.0-generate-001:predictLongRunning?key=${API_KEY}`;

  const prompt = `Slow cinematic camera push-in through darkness. Deep forest green atmosphere with golden particles of light slowly drifting through frame. A faint shield shape emerges from the darkness, made of warm gold light. Particles coalesce around it. The mood is protective, luxurious, premium. Colors: deep green (#002a1c) and warm gold (#FFDF00) only. No text. No people. Cinematic depth of field. Film grain 2%. 24fps. Slow, elegant movement.`;

  console.log('Starting video generation...');
  const response = await fetch(endpoint, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      instances: [{ prompt }],
      parameters: {
        aspectRatio: '16:9',
        durationSeconds: 8,
        personGeneration: 'DONT_ALLOW'
      }
    })
  });

  const data = await response.json();

  if (data.name) {
    console.log(`Operation: ${data.name}`);
    for (let i = 0; i < 30; i++) {
      await new Promise(r => setTimeout(r, 15000));
      const pollUrl = `https://generativelanguage.googleapis.com/v1beta/${data.name}?key=${API_KEY}`;
      const pollResponse = await fetch(pollUrl);
      const pollData = await pollResponse.json();
      if (pollData.done) {
        const samples = pollData.response?.generateVideoResponse?.generatedSamples;
        if (samples?.[0]?.video?.uri) {
          const videoResponse = await fetch(samples[0].video.uri);
          const videoBuffer = Buffer.from(await videoResponse.arrayBuffer());
          fs.writeFileSync('./generated-assets/hero/hero-video.mp4', videoBuffer);
          console.log(`Video saved (${(videoBuffer.length / 1024 / 1024).toFixed(1)}MB)`);
          return;
        }
        console.log('Done but no video URL found:', JSON.stringify(pollData).slice(0, 500));
        return;
      }
      console.log(`  Polling ${i + 1}/30...`);
    }
    console.log('Timeout after 7.5 minutes');
  } else {
    console.log('Error:', JSON.stringify(data).slice(0, 500));
  }
}

generateVideo().catch(console.error);
