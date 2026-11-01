# Browser tests for opening MakerFoundry links (PF-12)

These run the built web app in Chromium with Playwright. They are not part of `cargo xtask ci`.

1. Build the page:
   ```sh
   cargo build --release --target wasm32-unknown-unknown -p vectorcraft-web
   wasm-bindgen --target web --no-typescript --out-dir site target/wasm32-unknown-unknown/release/vectorcraft-web.wasm
   ```
   Copy `apps/vectorcraft-web/index.html` into `site/`, replacing the `<link data-trunk rel="rust" …/>`
   line with
   `<script type="module">import init from "./vectorcraft-web.js"; init({module_or_path: "./vectorcraft-web_bg.wasm"});</script>`.
2. `node open_test.js site`: MakerFoundry is mocked with `page.route`. The test checks that the
   design opens under its `Content-Disposition` name, that no cookie is sent, and that `?open=` leaves
   the page address. It also checks that MakerFoundry's sentence is shown for a 404, that other
   hosts are never fetched, and that files over 32 MB and other kinds are refused.
3. `node redirect_test.js site`: a mocked 302 isn't followed even by a plain fetch, so this test
   uses a real local HTTPS server that Chromium resolves `makerfoundry.app` and `evil.com` to. Make
   its certificate first:
   ```sh
   openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 2 \
     -subj "/CN=makerfoundry.app" -addext "subjectAltName=DNS:makerfoundry.app,DNS:evil.com"
   ```
   A plain fetch is the control: it follows the redirect, and the app must not.
