import { chromium } from 'playwright';
import { execSync } from 'child_process';
import path from 'path';

const URL = process.argv[2] || 'https://yukihamada.jp/mv/local-ai.html';
const OUT_DIR = '/tmp/mv-recording';
const DURATION_MS = 125_000; // ~2:05 + buffer

console.log(`Recording: ${URL}`);
console.log(`Duration: ${DURATION_MS / 1000}s`);

const browser = await chromium.launch({
  headless: false, // need real rendering for audio sync visuals
  args: [
    '--autoplay-policy=no-user-gesture-required',
    '--window-size=1920,1080',
  ],
});

const context = await browser.newContext({
  viewport: { width: 1920, height: 1080 },
  recordVideo: {
    dir: OUT_DIR,
    size: { width: 1920, height: 1080 },
  },
});

const page = await context.newPage();
await page.goto(URL, { waitUntil: 'networkidle' });

// Wait for page to fully load
await page.waitForTimeout(2000);

// Click play button
console.log('Clicking play...');
try {
  await page.click('#start-overlay', { timeout: 5000 });
} catch {
  try {
    await page.click('#playBtn', { timeout: 3000 });
  } catch {
    // Try clicking anywhere to start
    await page.click('body');
  }
}

console.log(`Recording for ${DURATION_MS / 1000}s...`);
await page.waitForTimeout(DURATION_MS);

// Get the video path
const videoPath = await page.video().path();
console.log(`Raw video: ${videoPath}`);

await context.close();
await browser.close();

// Combine with audio using ffmpeg
const finalPath = '/tmp/local-ai-mv.mp4';
const audioPath = '/Users/yuki/workspace/yukihamada.jp/public/mv/assets/local-ai/track_en.mp3';

try {
  execSync(`ffmpeg -y -i "${videoPath}" -i "${audioPath}" -c:v libx264 -crf 20 -preset fast -c:a aac -b:a 192k -shortest "${finalPath}"`, { stdio: 'inherit' });
  console.log(`\nDone! Video saved to: ${finalPath}`);
} catch {
  console.log(`\nVideo without audio: ${videoPath}`);
  console.log(`Add audio manually: ffmpeg -i "${videoPath}" -i "${audioPath}" -c:v copy -c:a aac -shortest "${finalPath}"`);
}
