<h1 align="center">PatternFoundry</h1>

<p align="center">
  <b>Open-source sewing-pattern design software, by SiteSentrix.</b>
</p>

<p align="center">
  Draft garment patterns from body measurements, change the design, grade sizes, and print or
  export them at true scale. It runs natively on macOS, Windows, Linux and FreeBSD, and in the
  browser, and is written in pure Rust on top of the VectorCraft vector editor.
</p>

<p align="center">
  <a href="https://github.com/SiteSentrix/patternfoundry/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/SiteSentrix/patternfoundry/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="Status: early release" src="https://img.shields.io/badge/status-early%20release-f2546b">
  <img alt="Written in pure Rust" src="https://img.shields.io/badge/pure-Rust-b83a24?logo=rust&logoColor=white">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-555555">
</p>

By [SiteSentrix](https://sitesentrix.com), the makers of [StitchFoundry](https://stitchfoundry.app).

## Download

<p align="center">
  <a href="https://github.com/SiteSentrix/patternfoundry/releases/latest"><b>Download PatternFoundry (Windows, macOS, Linux)</b></a> ·
  <a href="https://sitesentrix.github.io/patternfoundry/"><b>Try it in your browser</b></a>
</p>

## Status
**Early release (0.1, November 2026).** The first version is out: you can draft, change, grade and print
patterns today, but it is young and a small fraction of what the biggest pattern programs do. Print a pattern
at 100% and check the 10 cm test square before you cut fabric. The exports for cutting rooms follow the
published specifications but have not yet been tried in a real cutting-room program.

What works today (see [`docs/plan.md`](docs/plan.md) for every milestone and
[`docs/market.md`](docs/market.md) for the features we aim to match):

- **Pattern pieces:** seam allowance per edge, notches, grainlines, labels and a Pieces panel.
- **True-scale printing:** home-printer tiles with alignment marks and the 10 cm test square, copy-shop
  paper sizes, and a projector view.
- **Drafting from measurements:** size charts and your own measurements (kept on your computer), with
  bodice, skirt, sleeve, trouser and men's shirt blocks.
- **Design tools:** darts, slash and spread, pleats, collars, pockets, wrap fronts, bands, and seam truing.
- **Grading:** grade rules and a graded nest of sizes, with any one size exported.
- **Markers:** lay pieces on fabric, cut on the fold, and match plaids and checks.
- **Export:** DXF-AAMA for cutting rooms, SVG, and PDF for projector use.
- **3D drape preview:** an avatar from your measurements and your pattern draped on it, with glTF export.
- **Optional AI help:** ask about patterns and check a muslin, through a Foundry Account. Nothing is sent
  until you ask.
- **MakerFoundry:** publish a pattern as a private draft, and open a pattern from a MakerFoundry link.

## Built on VectorCraft

PatternFoundry includes the full [VectorCraft](https://github.com/storytold/vectorcraft) editor:
pen, shape and brush tools, Pathfinder, type, gradients, live effects and artboards, and SVG, PDF,
EPS, DXF, EMF and raster files. Every action is an engine command, so agents can drive the app
over a JSON control channel and an MCP server the way a person does.

## Build and run

Rust 1.97 or newer.

```sh
cargo run --release -p vectorcraft                          # the app (the program is patternfoundry)
cargo run --release -p vectorcraft -- --control 7979        # + JSON control channel
cargo run --release -p vectorcraft-cli -- mcp               # MCP server (stdio)
cargo xtask ci                                              # fmt, clippy, tests, layering, wasm
```

See [`docs/development.md`](docs/development.md) for fonts, graphics processors, Wayland and other
platform notes.

## For contributors and agents
Start with [`CLAUDE.md`](CLAUDE.md) and [`docs/start-here.md`](docs/start-here.md); the editor's
own rules (clean-room, assets, no panics, quality gates) are in
[`docs/vectorcraft-CLAUDE.md`](docs/vectorcraft-CLAUDE.md) and [`AGENTS.md`](AGENTS.md).

PatternFoundry is written clean-room: behaviour comes from public documentation only, never from
commercial pattern software.

## Videos and support

- **Videos:** the [MakerFoundry YouTube channel](https://www.youtube.com/@MakerFoundryApp)
  (Help › MakerFoundry on YouTube in the app).
- **Support PatternFoundry:** [Buy Me a Coffee](https://buymeacoffee.com/bwattendorf)
  (Help › Support PatternFoundry in the app). Donations are handled by Buy Me a Coffee and its
  payment processor; the maintainer sees the name, email and message a supporter gives.

## License and credits

PatternFoundry is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option.

Copyright (c) 2026 Bruce Wattendorf (SiteSentrix) and the PatternFoundry contributors.<br>
Copyright (c) 2026 ArtCraft Team and the VectorCraft contributors.

Required notices are in [NOTICE](NOTICE). Bundled fonts, icons, images and other assets keep their
own open licences; each one is listed with its author, source and licence in [ASSETS.md](ASSETS.md).
The app icon is original artwork; its files are in [`assets/app-icon/`](assets/app-icon/README.md).

### Where it comes from

| Repository | What PatternFoundry uses | How |
|---|---|---|
| [storytold/vectorcraft](https://github.com/storytold/vectorcraft) | the whole vector editor PatternFoundry is built on | merged in from time to time (last: version 0.13.1, upstream `a466625c`, 2026-10-18); MIT OR Apache-2.0 |
| [storytold/craft-libs](https://github.com/storytold/craft-libs) | the PDF reader (patched [hayro](https://github.com/LaurenzV/hayro) crates) | fetched at a pinned revision when building; MIT OR Apache-2.0 |
| [storytold/craft-fonts](https://github.com/storytold/craft-fonts) | the Japanese fonts | optional build input, never copied here; OFL |

Every other dependency is an ordinary Rust crate from crates.io, listed in `Cargo.lock`.

PatternFoundry is based on [VectorCraft](https://github.com/storytold/vectorcraft) by the ArtCraft
team. It is an independent project, not made, sponsored or endorsed by the ArtCraft team.

<sub>Adobe and Illustrator are trademarks of Adobe Inc.; PatternFoundry is not affiliated with
Adobe.</sub>
