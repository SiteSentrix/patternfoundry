// PF-12: a redirect from MakerFoundry is not followed. A real HTTPS server stands in for
// makerfoundry.app and evil.com (Chromium resolves both to it), because a mocked 302 isn't
// followed even by a plain fetch. A plain fetch from the page is the control: it does follow.
const { chromium } = require('playwright');
const https = require('https'), http = require('http'), fs = require('fs'), path = require('path');
const site = process.argv[2];
const hits = [];
const cors = { 'access-control-allow-origin': '*' };
https.createServer({ key: fs.readFileSync('key.pem'), cert: fs.readFileSync('cert.pem') }, (req, res) => {
  hits.push(`${req.headers.host}${req.url}`);
  if (req.headers.host.startsWith('evil.com')) { res.writeHead(200, { ...cors, 'content-type': 'image/svg+xml' }); res.end('<svg xmlns="http://www.w3.org/2000/svg"/>'); return; }
  res.writeHead(302, { ...cors, location: 'https://evil.com/api/v1/designs/1/file/evil.svg' }); res.end();
}).listen(8443, '127.0.0.1');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = http.createServer((req, res) => {
  const p = path.join(site, new URL(req.url, 'http://x').pathname.replace(/^\/$/, '/index.html'));
  fs.readFile(p, (err, data) => { if (err) { res.writeHead(404); res.end(); return; } res.writeHead(200, { 'content-type': types[path.extname(p)] || 'application/octet-stream' }); res.end(data); });
}).listen(8777, '127.0.0.1');
(async () => {
  const b = await chromium.launch({ args: ['--proxy-server=direct://', '--proxy-bypass-list=*', '--host-resolver-rules=MAP makerfoundry.app 127.0.0.1:8443, MAP evil.com 127.0.0.1:8443', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const c = await b.newContext({ ignoreHTTPSErrors: true });
  const p = await c.newPage();
  const file = 'https://makerfoundry.app/api/v1/designs/1/file/test.svg';
  await p.goto(`http://127.0.0.1:8777/?lang=en&open=${encodeURIComponent(file)}`);
  await p.waitForFunction(() => window.vectorcraft && window.vectorcraft.request, null, { timeout: 120000 });
  let st = null;
  for (let i = 0; i < 60; i++) { st = await p.evaluate(() => window.vectorcraft.request('ui.inspect')); if (st.ui.status || st.documents.length) break; await p.waitForTimeout(500); }
  const appHits = hits.slice();
  const control = await p.evaluate(async (u) => fetch(u).then(r => r.status + ' ' + r.url).catch(e => String(e)), file);
  const ok = appHits.length === 1 && !appHits.some(h => h.startsWith('evil.com')) && !st.documents.length && /couldn't be downloaded/.test(st.ui.status);
  console.log('app requests:', appHits, '| status:', st.ui.status, '| documents:', st.documents.length);
  console.log('control (plain fetch):', control, '| all hits:', hits);
  const controlFollowed = hits.some(h => h.startsWith('evil.com'));
  console.log(ok && controlFollowed ? 'PASS follows no redirect (control followed it)' : 'FAIL');
  await b.close(); server.close(); process.exit(ok && controlFollowed ? 0 : 1);
})();
