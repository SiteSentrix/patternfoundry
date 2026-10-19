# PatternFoundry: the plan

**Status:** 2026-10-19. PF-0, PF-1 and PF-2 done: PatternFoundry is VectorCraft renamed, with
pattern pieces and true-scale printing; measurements (PF-3) are next. Owner: Bruce Wattendorf (SiteSentrix).

## What it is
Software for designing sewing patterns, for home sewists, small pattern designers and small
studios: draft from measurements, change the design, grade sizes, and print or export at true
scale. Free and open source; AI help paid with StitchFoundry-style credits.

## How it is built
VectorCraft (github.com/storytold/vectorcraft), the clean-room Rust vector editor StitchFoundry
is built on, plus a pattern mode, the way StitchFoundry adds embroidery:
- pattern pieces are paths with pattern data (seam allowance per edge, notches, grainline,
  piece name, cut count, fabric);
- drafting, grading, truing and printing are engine commands (`piece.*` and later families;
  VectorCraft already uses `pattern.*` for fill patterns), so the menus, the control channel, the
  CLI and MCP all reach them;
- a pattern core crate (measurements, formulas, blocks, grading) below the UI layers, as
  StitchFoundry's `sf-core` is.
- reuse before rewriting: CADCraft (same licence; see `docs/start-here.md`) has the constraint solver
  for formulas (PF-3, PF-5), the DXF reader/writer for cutting-room export (PF-7) and layout plotting
  (PF-2).

## Milestones and task ids
| Id | What | Done when |
|---|---|---|
| PF-0 | Merge VectorCraft in (as StitchFoundry did, with `upstream` = storytold/vectorcraft); rename the app to PatternFoundry; CI green | the app opens as PatternFoundry on all platforms and the web |
| PF-1 | Pattern pieces: piece data on paths, seam allowance (per edge, corners mitred/squared), notches, grainline, labels, a Pieces panel | a piece prints with its allowance and marks |
| PF-2 | True-scale printing: tiled Letter/A4 with alignment marks and the 10 cm test square, A0/36-inch copy-shop PDF, layers per size | a printed tile measures exactly |
| PF-3 | Measurements: size charts and personal measurement sets (kept on the computer), formulas in point positions so a draft follows the measurements | changing the bust redraws the bodice |
| PF-4 | Blocks: drafted bodice, skirt, trouser and sleeve blocks from published drafting methods | a bodice block drafts from measurements |
| PF-5 | Design tools: dart move/rotate, slash and spread, add fullness, pleats, facings, collars, pockets; truing (walk seams, match lengths, smooth curves across seams) | a princess-seam bodice from the block |
| PF-6 | Grading: by rules per point and by redrafting from each size; nested size view | a size range from one pattern |
| PF-7 | Export for cutting: DXF-AAMA/ASTM D6673, SVG; projector mode | a DXF opens in a cutting-room tool |
| PF-8 | AI help (StitchFoundry's AI layer): draft from a description, fit advice from muslin photos, step-by-step help | "a wrap dress for my measurements" drafts |
| PF-9 | Marker making / nesting; selling patterns on MakerFoundry | a marker for 2 m of fabric |
| PF-10 | 3D fit preview (as Marvelous Designer and CLO do, from public docs only): avatar from measurements, sewing lines, cloth simulation, drape view beside the 2D pattern; glTF/OBJ export for game and 3D artists | a skirt drapes on an avatar of the user's size |

## Decided
- Domain: **patternfoundry.app**, bought by the owner (2026-10-19). Installers and the app link to
  the GitHub repository until the site is live; switching them is a one-line change each.

- Hosting (2026-10-19): the app stays on **GitHub Pages** (the web build) and **GitHub Releases**
  (the desktop downloads), as StitchFoundry does. No server of its own, no Railway.
- Accounts and AI (2026-10-19): PatternFoundry uses the shared **Foundry Account** (one sign-in and
  one credit balance, "Foundry credits", across all SiteSentrix apps), served by the StitchFoundry
  cloud service at https://cloud.stitchfoundry.app. PatternFoundry never holds an Anthropic key:
  AI requests go through that service, which adds the owner's key and charges the user's credits.
  Users may still enter their own Anthropic key in the app's AI settings instead. Details for PF-8
  are in `docs/start-here.md`.

## Open decisions (the owner's)
- Whether PatternFoundry is its own app (as planned) or a mode inside StitchFoundry.
- Which drafting methods to implement first (by the books or courses the owner's users follow).

## Progress log
- 2026-10-19, **PF-0**: VectorCraft 0.13.1 (upstream `a466625c`) merged with its history
  (`upstream` = storytold/vectorcraft). The programs are `patternfoundry` and `patternfoundry-cli`
  (the Cargo packages keep VectorCraft's names, so `cargo run -p vectorcraft` still works), the app
  id is `com.sitesentrix.patternfoundry`, documents save as `.patternfoundry` (`.vectorcraft` and
  `.drawcraft` still open), and settings live in their own PatternFoundry folder (VectorCraft's
  are read once). User-visible text, its 22 translations, the installers (MSI with new GUIDs,
  DMG, deb/rpm/AppImage, Flatpak, FreeBSD, web) and the MCP server say PatternFoundry. No ArtCraft
  name, logo or link remains (`cargo xtask brands` now rejects them); the About box credits
  VectorCraft in plain text. An original placeholder icon (a bodice pattern piece over a tape
  measure). `.github/workflows/ci.yml` runs `cargo xtask ci`; VectorCraft's own workflows run only
  in its repository. Not done: a release pipeline for PatternFoundry (signing secrets are the
  owner's), and the VectorCraft progress docs (ROADMAP, parity docs) still describe VectorCraft.
- 2026-10-19, **PF-1**: pattern pieces. `crates/pf-core` (no workspace dependencies, like
  StitchFoundry's `sf-core`) holds the piece data (name, cut count, mirrored pairs, fabric, seam
  allowance per edge in mm, mitred or squared corners, notches, grainline) and the geometry: the
  cutting line, notches (slit, double, V; 6 mm deep, never past the seam line) and the grainline
  arrow. **Pattern › Make Pattern Piece** (`piece.make`) turns each selected closed path into a
  piece: a group whose seam line is the path (dashed), with a solid cutting line, the grainline
  and a label ("Front Bodice / Cut 2 (mirrored pairs) · Linen") generated as locked children, so
  they print and export as ordinary art. Every edit redraws a piece whose seam line or data
  changed (as dimensions follow their art); moving a piece moves its grainline. `piece.set`,
  `piece.addNotch`, `piece.removeNotch`, `piece.info`, `piece.list`, `piece.release`; the
  **Pattern Pieces** panel (Pattern and Window menus) edits name, fabric, cut, mirrored pairs and
  the allowance; every new label translated in all 23 languages. Tests: 15 in `pf-core`, 8 engine
  tests (`tests_piece.rs`: geometry at true scale, following edits, undo, save, export, bad input).
  Not yet: notches and per-edge allowances from the panel (commands only), notches placed by
  clicking, a dock icon for the panel, self-intersection cleanup where inward curves are tighter
  than the allowance, and translations of the commands' error messages.
- 2026-10-19, **PF-2**: true-scale printing, built on VectorCraft's own print tiling (Tile Full
  Pages / Tile Imageable Areas, overlap, margins, tile range, the Print Tiling tool), which
  already existed. New: **Pattern Marks** in File › Print (`patternMarks`): every page carries a
  10 cm and a 1 in test square; every tile its name (rows as letters, columns as numbers: A1 is
  top left, with the grid's size), its printable area's edge and the lines where the next pages
  begin, to trim and tape by (`pf_core::tiling`). Copy-shop paper: A2, A1, A0 and the 36 in roll
  (36 × 48 in). A warning when Tile Full Pages would leave the printer's margins unprinted
  between pages (tile the imageable areas instead). Tests read the PDFs back
  (`crates/pdf/src/tests_truescale.rs`): a 10 cm square measures 10 cm at 100% on Letter, on
  every tile and on A0, A1 and the roll, both test squares are on every page, pages are their
  paper or artboard sizes, and exported PDFs (`document.exportPdf`) keep artboards at 100%.
  Not yet: layers per size (waits for grading, PF-6); the printer driver's own "fit to page"
  can't be checked from here, so the test squares are how a sewist checks it.
