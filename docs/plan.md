# PatternFoundry: the plan

**Status:** 2026-10-19. PF-0 to PF-4 done and PF-5 started: PatternFoundry is VectorCraft renamed, with
pattern pieces, true-scale printing, drafting from measurements, bodice and skirt blocks, and
dart rotation, splitting and slash and spread (a princess-seam bodice from the block). Owner: Bruce Wattendorf (SiteSentrix).

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

- Drafting method (2026-10-19): **Aldrich** (Winifred Aldrich, *Metric Pattern Cutting for
  Women's Wear*), British metric, first. Blocks are built from public descriptions of the method
  (cited by URL in `crates/pf-core/src/block.rs`), never from the book's text or drawings.

## Open decisions (the owner's)
- Whether PatternFoundry is its own app (as planned) or a mode inside StitchFoundry.

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
- 2026-10-19, **PF-3**: measurements and drafting. The document's measurement set (values in
  mm, and formulas over them: `quarter = (bust + ease) / 4`), a size chart read from a table
  (comma, semicolon or tab separated; mm, cm or in) whose sizes fill in the set, and point
  formulas on any path or a piece's seam line (`x`, `y` in mm from the draft's origin, which
  moves with the path). Every edit places drafted points again, then pieces redraw: changing the
  bust redraws the bodice and its cutting line, in one undo step. `measure.set`, `measure.info`,
  `measure.clear`, `measure.importChart`, `measure.useSize`, `draft.setPoint`, `draft.info`,
  `draft.clear`; the **Measurements** panel (Pattern and Window menus). The formula language is
  CADCraft's `expr.rs`, adapted (same licence; NOTICE). **Personal data:** measurements live in
  the document only; the copies that PDF, SVG and EPS exports embed (Preserve Editing) leave the
  measured values out unless the set's `share` is on (formulas stay); nothing logs them. Tests:
  10 more in `pf-core`, 5 engine tests (`tests_measure.rs`). Not yet: point formulas from the
  panel or the canvas (commands only), personal sets kept apart from documents in the settings
  folder, curves' handles placed by formulas, standard size charts bundled (charts are the
  user's own: standards bodies' tables can't be copied in).
- 2026-10-19, **PF-4**: blocks, Aldrich-style (British metric). **Draft Bodice Block**
  (`block.bodice`: back and front close-fitting half bodices from bust, waist, nape to waist,
  back width, chest, shoulder, neck and armscye depth; 5 cm ease, shoulder and waist darts) and
  **Draft Skirt Block** (`block.skirt`: back and front straight skirt from waist, hip, waist to
  hip and skirt length; 1 cm hip ease, the waist reduction two thirds at the side seams, the rest
  in back and front darts), in the Pattern menu; `block.measurements` names what a block needs
  and what the set lacks. Each part is a path that remembers its block: when a measurement
  changes every edit draws it again (inside a pattern piece too, whose cutting line follows),
  where it was moved to, in one undo step. Built from public tutorials of the method, cited in
  `pf_core::block`; the owner can check them against the book. Tests: 4 in `pf-core`, 5 engine
  tests (`tests_block.rs`). Not yet: sleeve and trouser blocks (PF-4.1, PF-4.2), a block panel
  with the method's options (ease, dart sizes), the bodice's armhole checked against the sleeve
  head once the sleeve exists.
- 2026-10-19, **PF-5** (first part): design tools. **Rotate Dart** (`design.rotateDart`: the
  pivot method; the side of the outline with fewer points swings round the dart's point so the
  dart opens elsewhere on the outline, at a point or anchor; all of it or a share, leaving two
  darts) and **Split at Points** (`design.split`: a closed path cut in two between two points,
  each part closed by a straight seam; at a point two darts share, no seam is added), in the
  Pattern menu (with two or one selected points); `design.length` measures along an outline to
  true seams that sew together. Works on a piece's seam line too (the piece redraws; a split
  needs the piece released first). A block part changed this way stops following the
  measurements. Geometry in `pf_core::design`. The milestone: a princess-seam bodice from the
  front block (bust dart into the armhole, split through the bust point; the two princess seams
  match within the front waist's slope). **Slash and spread** (`design.spread`): the outline cut
  straight between two points (anchors, or points on it) and opened by an amount, hinged at the
  far end (flare: a skirt widened at the hem, its waist kept) or parallel (even fullness, room
  for a pleat); commands only for now (it needs an amount: a dialog later). **Truing**
  (`design.matchLength`, PF-5.2): a seam's end point moves along the seam until the seam is a
  given length or as long as another seam (the princess seams then match exactly). Tests: 6 in
  `pf-core`, 5 engine tests (`tests_design.rs`). Not yet (PF-5.x): pleat and gathering marks, facings, collars,
  pockets; more truing (walking seams with notches, smoothing curves across seams, the princess seam curved through
  the bust); dart shortening from the bust point; design steps replayed when the measurements
  change.
