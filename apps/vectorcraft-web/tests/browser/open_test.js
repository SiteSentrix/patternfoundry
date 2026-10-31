// PF-12: the web app opening ?open= links, in Chromium, with MakerFoundry mocked by page.route.
// Usage: node open_test.js <site dir>   (the built page: index.html, vectorcraft-web.js, _bg.wasm)
const { chromium } = require('playwright');
const http = require('http');
const fs = require('fs');
const path = require('path');

const site = process.argv[2];
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = http.createServer((req, res) => {
  const p = path.join(site, decodeURIComponent(new URL(req.url, 'http://x').pathname).replace(/^\/$/, '/index.html'));
  fs.readFile(p, (err, data) => {
    if (err) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { 'content-type': types[path.extname(p)] || 'application/octet-stream' });
    res.end(data);
  });
}).listen(8777, '127.0.0.1');

const SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50"><rect x="10" y="10" width="80" height="30" fill="#3a7"/></svg>';
const FILE = 'https://makerfoundry.app/api/v1/designs/1/file/test.svg';
const TOKEN = '0123456789abcdef0123456789abcdef0123456789abcdef';
const cors = { 'access-control-allow-origin': '*', 'access-control-expose-headers': 'Content-Disposition' };

let failures = 0;
function check(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : ': ' + JSON.stringify(detail)}`);
  if (!ok) failures++;
}

// Open the page with `link` (already a full address) in ?open=, answering MakerFoundry with `answer`.
async function run(browser, link, answer) {
  const context = await browser.newContext();
  await context.addCookies([{ name: 'mf_session', value: 'secret', domain: 'makerfoundry.app', path: '/', secure: true, sameSite: 'None' }]);
  const page = await context.newPage();
  const seen = [];
  await context.route(/^https:\/\/(www\.)?makerfoundry\.app\//, async (route) => {
    const req = route.request();
    seen.push({ url: req.url(), method: req.method(), headers: await req.allHeaders() });
    await answer(route);
  });
  await context.route(/^https:\/\/evil\.com\//, async (route) => {
    seen.push({ url: route.request().url(), evil: true });
    await route.fulfill({ status: 200, headers: { ...cors, 'content-type': 'image/svg+xml' }, body: SVG });
  });
  const url = `http://127.0.0.1:8777/?lang=en&open=${encodeURIComponent(link)}#top`;
  await page.goto(url);
  await page.waitForFunction(() => window.vectorcraft && window.vectorcraft.request, null, { timeout: 120000 });
  let inspect = null;
  for (let i = 0; i < 60; i++) {
    inspect = await page.evaluate(() => window.vectorcraft.request('ui.inspect'));
    if (inspect.documents.length || inspect.ui.status) break;
    await page.waitForTimeout(500);
  }
  const address = page.url();
  await context.close();
  return { inspect, address, seen };
}

(async () => {
  const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    // 1. A design file opens, named from Content-Disposition, fetched without cookies.
    let r = await run(browser, `${FILE}?t=${TOKEN}`, (route) => route.fulfill({
      status: 200, headers: { ...cors, 'content-type': 'image/svg+xml', 'content-disposition': 'attachment; filename="Tote bag.svg"' }, body: SVG }));
    check('opens the design', r.inspect.documents.map(d => d.title).join() === 'Tote bag.svg', r.inspect.documents);
    check('fetches the checked address once', r.seen.length === 1 && r.seen[0].url === `${FILE}?t=${TOKEN}` && r.seen[0].method === 'GET', r.seen);
    check('sends no cookie', r.seen.length === 1 && !('cookie' in r.seen[0].headers), r.seen[0] && r.seen[0].headers);
    check('takes ?open= out of the page address', r.address === 'http://127.0.0.1:8777/?lang=en#top', r.address);

    // 2. MakerFoundry refuses: its own sentence in the status bar.
    r = await run(browser, FILE, (route) => route.fulfill({
      status: 404, headers: { ...cors, 'content-type': 'application/json' }, body: JSON.stringify({ error: { message: "That design isn't on MakerFoundry." } }) }));
    check('says MakerFoundry\'s sentence', r.inspect.ui.status === "MakerFoundry: That design isn't on MakerFoundry." && !r.inspect.documents.length, r.inspect.ui.status);

    // 3. A link elsewhere is never fetched.
    r = await run(browser, 'https://evil.com/api/v1/designs/1/file/test.svg', (route) => route.abort());
    check('refuses a link elsewhere unfetched', r.seen.length === 0 && /isn't a design file on MakerFoundry/.test(r.inspect.ui.status), [r.seen, r.inspect.ui.status]);
    r = await run(browser, 'https://makerfoundry.app.evil.com/api/v1/designs/1/file/test.svg', (route) => route.abort());
    check('refuses a look-alike host unfetched', r.seen.length === 0 && /isn't a design file/.test(r.inspect.ui.status), [r.seen, r.inspect.ui.status]);

    // 4. A redirect is not followed (Chromium doesn't follow a mocked 302 even for a plain fetch:
    // redirect_test.js checks this against a real server).
    r = await run(browser, FILE, (route) => route.fulfill({ status: 302, headers: { ...cors, location: 'https://evil.com/api/v1/x.svg' } }));
    check('follows no redirect', !r.seen.some(s => s.evil) && !r.inspect.documents.length && /couldn't be downloaded/.test(r.inspect.ui.status), [r.seen, r.inspect.ui.status]);

    // 5. Too large: stopped, not opened.
    r = await run(browser, FILE, (route) => route.fulfill({ status: 200, headers: { ...cors, 'content-type': 'image/svg+xml' }, body: Buffer.alloc(33 * 1024 * 1024, 32) }));
    check('refuses more than 32 MB', !r.inspect.documents.length && /too large to open/.test(r.inspect.ui.status), r.inspect.ui.status);

    // 6. Not an SVG or DXF.
    r = await run(browser, 'https://makerfoundry.app/api/v1/designs/1/file/test.pdf', (route) => route.fulfill({ status: 200, headers: { ...cors, 'content-type': 'application/pdf' }, body: '%PDF-1.7' }));
    check('refuses other kinds', !r.inspect.documents.length && /SVG and DXF/.test(r.inspect.ui.status), r.inspect.ui.status);
  } finally {
    await browser.close();
    server.close();
  }
  console.log(failures ? `${failures} FAILED` : 'ALL PASSED');
  process.exit(failures ? 1 : 0);
})();
