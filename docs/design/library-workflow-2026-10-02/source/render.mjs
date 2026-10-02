// Renders the mockups to PNG with playwright-core and the system Chromium, on
// the GPU (ANGLE on EGL). Run it under gpu-lock, one GPU job at a time:
//   NODE_PATH=/path/to/node_modules gpu-lock node source/render.mjs        every scene
//   NODE_PATH=/path/to/node_modules gpu-lock node source/render.mjs 01 05  scenes starting with these
// playwright-core is not a dependency of this repository; install it outside
// the tree and point NODE_PATH at it. The pages load the redesign's kit from
// ../../redesign-2026-10-01/source/assets, so both folders must be present.
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const { chromium } = createRequire(import.meta.url)('playwright-core');
const src = path.dirname(fileURLToPath(import.meta.url));
const out = path.dirname(src);
const S = [1024, 640], M = [1280, 760], L = [2560, 1440], SHEET = [1600, 1000];
const both = ['dark', 'light'], dark = ['dark'];
// [png name, html file, sizes, themes, query]
const scenes = [
  ['01-main', '01-main.html', [S, M, L], both],
  ['02-audition-library', '02-audition-library.html', [S, M], dark],
  ['03-audition-keep-menu', '02-audition-library.html', [M], dark, 'menu=keep'],
  ['04-audition-cloud', '04-audition-cloud.html', [M], dark],
  ['05-cannot-play-family', '05-cannot-play.html', [M], dark, 'case=family'],
  ['06-no-pedal', '05-cannot-play.html', [M], dark, 'case=nopedal'],
  ['07-pro-read-only', '07-pro-read-only.html', [M], dark],
  ['08-drag-tone-to-slot', '08-drag.html', [M], dark, 'case=slot'],
  ['09-drop-confirm', '08-drag.html', [M], dark, 'case=confirm'],
  ['10-drag-preset-to-tones', '08-drag.html', [M], dark, 'case=keep'],
  ['11-drag-cloud-to-slot', '08-drag.html', [M], dark, 'case=cloud'],
  ['12-drop-several', '08-drag.html', [M], dark, 'case=several'],
  ['13-drag-setlist-to-pedal', '08-drag.html', [M], dark, 'case=setlist'],
  ['14-publish', '08-drag.html', [M], dark, 'case=publish'],
  ['15-drag-and-drop-map', '15-map.html', [SHEET], dark],
  ['16-device-markers', '16-markers.html', [SHEET], both],
  ['17-menus', '17-menus.html', [SHEET], dark],
  ['18-mine', '18-mine.html', [M], dark],
  ['19-mine-server', '18-mine.html', [M], dark, 'server=1'],
  ['20-mine-delete', '18-mine.html', [M], dark, 'server=1&dialog=delete'],
  ['21-pedal-pages', '21-pedal-pages.html', [S, M], dark],
  ['22-pedal-pages-pro', '21-pedal-pages.html', [M], dark, 'pro=1'],
  ['23-setlist-slot-audition', '23-setlists.html', [M], dark],
];
const wanted = process.argv.slice(2);
const browser = await chromium.launch({
  executablePath: '/usr/bin/chromium',
  args: ['--allow-file-access-from-files', '--use-angle=gl-egl', '--use-gl=angle', '--force-color-profile=srgb', '--font-render-hinting=none'],
});
{
  const probe = await browser.newPage();
  const gl = await probe.evaluate(() => {
    const c = document.createElement('canvas').getContext('webgl');
    const e = c && c.getExtension('WEBGL_debug_renderer_info');
    return e ? c.getParameter(e.UNMASKED_RENDERER_WEBGL) : 'no webgl';
  });
  console.log('renderer:', gl);
  await probe.close();
}
for (const [name, file, sizes, themes, query] of scenes) {
  if (wanted.length && !wanted.some(w => name.startsWith(w))) continue;
  for (const [w, h] of sizes) {
    for (const theme of themes) {
      const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: 1 });
      page.on('console', m => { if (m.type() === 'error') console.log(name, 'console:', m.text()); });
      page.on('pageerror', e => console.log(name, 'error:', e.message));
      const q = [query, theme === 'light' ? 'theme=light' : ''].filter(Boolean).join('&');
      await page.goto('file://' + path.join(src, file) + (q ? '?' + q : ''));
      await page.waitForFunction(() => document.body.dataset.ready === '1', null, { timeout: 15000 });
      await page.waitForTimeout(150);
      const target = path.join(out, `${name}-${w}x${h}-${theme}.png`);
      await page.screenshot({ path: target });
      console.log('rendered', path.basename(target));
      await page.close();
    }
  }
}
await browser.close();
