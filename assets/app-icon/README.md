# PatternFoundry app icon

**Subject:** sewing patterns: a bodice front pattern piece on manila pattern paper, with its dashed
stitching line, a waist dart, an armhole notch and a red grainline arrow, over a yellow tape measure.

**Palette:**

| Colour | Hex | Used for |
|---|---|---|
| Teal | `#1f5560` | the full-bleed field |
| Manila | `#f6eedb` | the pattern paper |
| Ink | `#16262b` | cut edge, stitching line, dart, notch, tape marks |
| Red | `#c4472f` | the grainline |
| Yellow | `#f2b33d` | the tape measure |

**Tile:** `viewBox="0 0 512 512"`, a rounded square with `rx=112`, clipped. The macOS files add Apple's
transparent margin (tile = 824/1024 of the canvas); Windows and Linux use the tile edge to edge.

**Provenance:** original artwork made for PatternFoundry, written by hand as SVG (paths and strokes, no
third-party material). `patternfoundry.svg` is the canonical artwork; `patternfoundry-small.svg` is the
same file for `hicolor/scalable`. Licence: see `LICENSE.txt`.

## Files

| File | What | Used by |
|---|---|---|
| `patternfoundry.svg` | master vector | source for everything below |
| `patternfoundry-small.svg` | the same artwork | `hicolor/scalable` |
| `patternfoundry-1024.png` | full tile, 1024 px | docs, store listings |
| `patternfoundry-macos-512.png` | tile with Apple margin, 512 px | runtime Dock icon (`apps/vectorcraft/src/main.rs`) |
| `patternfoundry.icns` | macOS icon set, 16 to 1024 px | `cargo xtask bundle` (`CFBundleIconFile`) |
| `patternfoundry.ico` | Windows icon, 16 to 256 px | `apps/vectorcraft/build.rs` (embedded in the `.exe`) |
| `hicolor/<size>/apps/com.sitesentrix.patternfoundry.png` | Linux theme icons, 16 to 512 px; the 256 one is also the runtime icon on Windows and Linux, the 128 one the brand mark in the app bar and About box (`crates/ui-egui/src/brand.rs`) | `packaging/linux/com.sitesentrix.patternfoundry.desktop` |
| `hicolor/scalable/apps/com.sitesentrix.patternfoundry.svg` | Linux scalable icon | as above |

## How it reaches each OS

- **macOS:** the app sets the Dock / app-switcher icon at runtime (`ViewportBuilder::with_icon`); the
  `.app` from `cargo xtask bundle` carries `patternfoundry.icns`.
- **Windows:** `build.rs` embeds `patternfoundry.ico` and VERSIONINFO with `winresource` (taskbar, Start
  menu, Explorer, Alt-Tab); the runtime icon covers the title bar.
- **Linux:** install `packaging/linux/com.sitesentrix.patternfoundry.desktop` to `share/applications/`
  and `hicolor/` to `share/icons/`. The app sets its Wayland app ID / X11 class to
  `com.sitesentrix.patternfoundry`, so the dock matches the window to the launcher.

## Regenerate

Edit or replace `patternfoundry.svg` (and `patternfoundry-small.svg`), then run `packaging/icons.sh`
(needs `resvg`). It writes every PNG, the `.ico` (via `cargo xtask ico`) and the `.icns` (via
`cargo xtask icns`).
