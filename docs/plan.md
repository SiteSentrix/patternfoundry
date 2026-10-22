# PatternFoundry: the plan

**Status:** 2026-10-20. PF-0 to PF-4 done and PF-5 started: PatternFoundry is VectorCraft renamed,
with pattern pieces, true-scale printing, drafting from measurements, bodice, skirt, sleeve and
trouser blocks, and the first design tools (dart rotation, splitting, slash and spread, seam
matching, facings, knife pleats: a princess-seam bodice from the block). Owner: Bruce Wattendorf
(SiteSentrix).

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
- Sleeve method (2026-10-20): **Natalie Bray**'s one-piece sleeve, from a public tutorial
  (cited in `crates/pf-core/src/block.rs`): no public description of Aldrich's sleeve was found,
  and her book's own text is not copied.
- Trouser method (2026-10-20): **The Shapes of Fabric**'s basic pants block (metric, women's;
  the same author as the skirt block), from her public tutorial, cited in
  `crates/pf-core/src/block.rs`. Fashion Capital's was the other candidate (more detailed curves,
  but a step cut off in its text).

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
- 2026-10-19/20, **PF-5** (design tools, first parts). **Rotate Dart** (`design.rotateDart`: the
  pivot method; the side of the outline with fewer points swings round the dart's point so the
  dart opens elsewhere on the outline, at a point or anchor; all of it or a share, leaving two
  darts) and **Split at Points** (`design.split`: a closed path cut in two between two points,
  each part closed by a straight seam; at a point two darts share, no seam is added), in the
  Pattern menu (with two or one selected points); `design.length` measures along an outline. Works
  on a piece's seam line too (the piece redraws; a split needs the piece released first). A block
  part changed this way stops following the measurements. Geometry in `pf_core::design`. The
  milestone: a princess-seam bodice from the front block (bust dart into the armhole, split
  through the bust point). **Slash and spread** (`design.spread`, PF-5.1): the outline cut
  straight between two points (anchors, or points on it) and opened by an amount, hinged at the
  far end (flare: a skirt widened at the hem, its waist kept) or parallel (even fullness, room for
  a pleat); a command only for now (it needs an amount: a dialog later). **Truing**
  (`design.matchLength`, PF-5.2): a seam's end point moves along the seam until the seam is a
  given length or as long as another seam (the princess seams then match exactly). **Facings**
  (`design.facing`, PF-5.3; Pattern › Make Facing with two selected points): a new path whose
  outer edge is a run of the outline (a neckline, an armhole) and whose inner edge is that run
  moved inward by the width (5 cm unless given; curves cut into pieces that turn at most 1/32 of a
  turn, within 0.05 mm of the true offset), its ends along the seams either side. **Knife pleats**
  (`design.pleat`, PF-5.4): a parallel spread of twice the pleat's depth along a line from one
  point to another, folding toward the side that stays, with its fold line (solid, an arrow to the
  placement line) and placement line (dashed) added in the piece, so they move with it; a command
  only for now (it needs a depth). **Inverted pleats** (`design.pleat` with `kind: "inverted"`,
  PF-5.5): four times the depth, both edges folded to the middle, two fold lines and arrows and
  one placement line. **Gathering marks** (`design.gather`, PF-5.6; Pattern › Gather with two
  selected points): a wavy line 5 mm inside an edge and a tick across it at each end; with `into`,
  the length it gathers into and the ratio are reported and the mark is named after it. **Flat
  collars** (`design.collar`, PF-5.7; Pattern › Draft Collar with the bodice block's back and
  front selected), from The Creative Curator's published steps (cited in `pf_core::design`): the
  front neckline laid against the back's at the neck point, the shoulders overlapping (3 cm unless
  given; the page says 2 to 4), the neckline lowered if asked (1 cm for a block), the collar 6 cm
  wide unless given, its front rounded to centre front; the neck edge is as long as both
  necklines. Corners where an offset edge turns are now joined by an arc outside and trimmed
  inside (an edge offset twice had spiked out at the neck point: caught by drawing it, then by a
  test). **Patch pockets** (`design.pocket`, PF-5.8): the pocket piece, its finished width and
  depth plus a hem (2.5 cm unless given) folded inside at the top with its fold line (dashed),
  square, rounded or pointed at the bottom; with `on` and `at`, the finished outline marked on the
  garment (dashed, in the piece); a command only for now (it needs a size). **Walking seams**
  (`design.walk`, PF-5.9): two seams that sew together, each from the ends that meet and the
  shorter way round its outline; their lengths and the difference, and with `notches` or `every`
  matching notches at the same distances along both pieces, in one undo step. Walking the skirt
  block's side seams found the front's 0.23 mm shorter (its hip curve's handles left out the rise
  above the waist line); fixed, they now match. **Smoothing across a seam** (`design.smooth`,
  PF-5.10): where a hem, waist, neckline or armhole meets a seam on two pieces that sew together,
  both curves' handles are turned to one line (the second piece laid against the first along the
  seam), so the curve runs on without a corner once sewn; handle lengths are kept and the corner
  there was is reported. **Box pleats** (`design.pleat` with `kind: "box"`, PF-5.11): an inverted
  pleat turned over, the slash spread four times the depth, the fold lines a depth in from the
  spread's edges with arrows out to placement lines a depth past them; the box shows twice the
  depth wide and the fabric meets behind it. **Stand collars** (`design.standCollar`, PF-5.12;
  Pattern › Draft Stand Collar), from The Creative Curator's published method
  (https://www.thecreativecurator.com/draft-mandarin-collar-pattern/): half a collar, centre back
  on the fold, as long as the back and front necklines (measured on the bodice block, or on any
  path given its neck points); the neck edge straight to the shoulder, then curving up 15 mm by
  centre front (so it hugs the neck), on past centre front by the extension (half the button stand
  when the fronts meet, all of it when they overlap); the top edge parallel at the stand's height
  (35 mm), the front end square, the centre front line dashed. **In-seam pockets**
  (`design.inSeamPocket`, PF-5.13; Pattern › In-Seam Pocket), after the published methods of
  Seamwork (https://www.seamwork.com/articles/design-your-own-in-seam-pockets) and Doina Alexei
  (https://www.doinaalexei.com/beginnersewingtutorialblog/video-tutorial-how-to-make-a-sewing-pattern-for-a-skirt-on-seam-pocket):
  a pocket bag hanging inside the piece from its seam, its seam edge following the seam from 15 mm
  above the opening to 15 mm below it (the opening 140 mm long, 90 mm down by default), square in
  by the allowance, then curving out 130 mm and down 150 mm below the opening; on pattern pieces,
  notches at the opening's ends on the seam and on the seam it sews to, in the same undo step.
  **Welt pockets** (`design.weltPocket`, PF-5.14; Pattern › Welt Pocket), after The Shapes of
  Fabric's published tutorial
  (https://www.theshapesoffabric.com/2019/06/24/welt-pocket-tutorial/): single or double welts;
  the welt strip (the opening plus 15 mm at each end, twice the welt plus 7 mm each side, folded
  along its middle; cut two for double welts), the lower bag and the upper one deeper by the
  opening's height, both as long as the welt; with `on` and `at`, the opening box (dashed) and its
  cut lines (along the middle, out to the corners) on the garment. **Shirt collars**
  (`design.shirtCollar`, PF-5.15; Pattern › Draft Shirt Collar), after the published method in
  NIMI's Fashion Design Technology trade theory
  (https://nimi.co.in/cits_ebooks1/Fashion_Design_Technology/Fashion_Design_Technology_TT/files/basic-html/page164.html):
  the stand (a stand collar, 30 mm high, 10 mm rise, 15 mm past centre front) and above it the
  collar that turns down over it, its neck edge as long as the stand's top edge to centre front
  and curving 13 mm down at the front, 40 mm high at centre back, its point 51 mm long and 19 mm
  out. Tests: 19 in `pf-core`, 18 engine tests (`tests_design.rs`). Not yet (PF-5.x): box pleats
  wider than twice their depth (two slashes), notches at a pleat's ends, rounded or winged stand
  fronts, flap pockets; more truing (the princess seam curved through the bust); dart shortening
  from the bust point; design steps replayed when the measurements change; the sewing terms in the
  translations checked by sewists who speak each language.
- 2026-10-20, **PF-4.1**: the one-piece straight sleeve block (`block.sleeve`, Pattern › Draft
  Sleeve Block), Natalie Bray's method from a public tutorial: the crown depth from the top arm by
  the tutorial's table, 5 cm ease round the top arm, the crown through marks on the back and front
  lines and the tutorial's midpoint offsets, straight sides, the wrist curved down at the back and
  up at the front; it follows the measurements (`top_arm`, `sleeve_length`). The result reports
  the crown's length. By the table alone the crown came out about as long as the bodice's armhole
  (1.6 mm short with the test's made-up measurements), not the 3 to 4.5 cm longer a set-in sleeve
  needs, so an optional `sleeve_crown` measurement (a formula such as `armhole + 30`) sets the
  crown's length, and the crown depth is solved to match it. Blocks drafted without an origin now
  start 20 mm inside the artboard by their own extent, so the sleeve, which reaches left of its
  origin, isn't cut off. Tests: 1 more in `pf-core`, 1 more engine test (`tests_block.rs`). Not
  yet: the bodice's armhole measured into the set by a command (it is `design.length` by hand
  now), notches for the crown's balance points, a two-piece sleeve, an elbow dart.
- 2026-10-20, **PF-4.2**: the basic trouser block (`block.trousers`, Pattern › Draft Trouser
  Block), front and back side by side, from The Shapes of Fabric's metric steps: half the hip and
  2 cm ease wide, the crotch line 1.5 cm below the crotch depth, crotch extensions of an eighth of
  the half hip out from the centre front (3 cm more at the back), the centre front 0.5 cm in and 1
  cm down at the waist, the centre back 4 cm in and 2.5 cm above, waists of a quarter waist and
  1.5 cm (front) and 0.5 cm with a 2 × 10 cm dart (back), hems 1 cm under and over half the hem,
  the crease half way between fork and side seam, the knees from the line between hem and fork (1
  cm wider each side at the back). Measurements: `waist`, `hip`, `waist_to_hip` (shared with the
  skirt), `crotch_depth`, `waist_to_knee`, `waist_to_ankle`, `trouser_hem`. The test drafts the
  tutorial's own example and gets its numbers (forks 6 and 9 cm out, hems 22 and 24 cm, waists 20
  and 19 cm). The curves the tutorial leaves to its drawings are chosen in the code and marked;
  the block was looked at as an exported picture. Tests: 1 more in `pf-core`, 1 more engine test.
  Not yet: a front dart (the author's comments add about 1 cm at the front side seam for one),
  walking the inseams and side seams (the tutorial's own check), a waistband.
- 2026-10-21, **PF-7.1**: DXF-AAMA export for cutting rooms (`document.export {format: aama}`,
  File › Export › Export As › DXF-AAMA (cutting room)), written from the published descriptions of
  ASTM D6673 (DH Patterns and Fit's overview,
  https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf, and Patro's
  notes, https://fabricesalvaire.github.io/Patro/resources/file-format/dxf-astm.html; the standard
  itself is not bought). DXF R13 (or R12 for older readers): a block per piece, inserted at the
  origin, its cutting line on layer 1, seam line on 14, both copied to the validation layers 84
  and 87, turn points (2) and curve points (3) of both, notches as points on the cutting line
  carrying depth, width and angle (4), the grainline (7), the paths drawn inside the piece as
  internal lines (8), and the piece system text (Piece Name, Quantity as right and left pieces,
  Material); the style system text (Style Name, dates, Author, Sample Size, Units: METRIC, the
  standard's version, Curve Tolerance) on layer 1 at the origin. Millimetres, y up, the pieces
  where they are relative to each other. An unnamed piece goes by its seam line's name; every
  piece name is once in the file. The DXF writer (`vectorcraft-cad`) gained blocks besides model
  and paper space. Checked: the 10 cm square test (its seam line exactly 100 mm across in the
  file, the cutting line 120), and the skirt block's file read by an independent reader (ezdxf
  1.4.5's recover and strict readers: no errors, no fixes), drawn the right way up. Tests: 3 in
  `vectorcraft-cad`, 3 engine tests (`tests_aama.rs`). Not yet: grading (graded nests and the rule
  table, PF-6), drill holes, mirror lines and the NM attribute, notch shapes besides slit and V
  (layers 80 to 83), opening AAMA files, an options dialog (the style name and sample size are
  parameters), a test in a cutting-room program. Seen while checking: the cutting line runs its
  allowance round a dart's legs, as if the dart were cut out; a cutting line should bridge the
  dart (PF-1 follow-up).
- 2026-10-21, **PF-1.1**: the cutting line bridges darts. A dart (a sharp inward V of two straight
  legs, closer than 50°) is no longer cut round: the cutting line crosses it with the dart's cap,
  constructed as a pattern maker does by folding the dart closed (pressed toward the side before
  it), drawing the allowance and opening it again; the allowance lines meet the legs where the
  folded line crosses them, equally far from the point, and the cap peaks (or dips, where closing
  the dart bends the seam) over the dart's middle line where the folded-under dart reaches. Found
  while checking the DXF-AAMA export (PF-7.1), whose pieces had their allowance run round the
  darts' legs as if the darts were cut out. Tests: 3 in `pf-core` (`allowance.rs`: nothing of the
  cutting line inside a dart, the cap's points mirrored, the cap folding flat onto a straight
  waist and onto a sloping one); they fail with the dart test switched off. The skirt and bodice
  blocks made pieces were looked at: every waist, shoulder and bust dart bridged. Not yet: a dart
  pressed toward the other side (a choice per dart), curved dart legs, darts inside a curved edge.
- 2026-10-21, **PF-7.1** (moved onto CADCraft's DXF crate, as the owner asked): DXF-AAMA files are
  now written and read on `crates/pf-dxf`, CADCraft's standalone DXF tag reader and writer
  (`crates/dxf`, from the published DXF reference; same licence, copyright kept in `NOTICE`, the
  CADCraft name kept out of the crate's name and messages), by a new `crates/pf-aama` (layer 1):
  the same file as before (R13 or R12, a block per piece, the layers and texts of ASTM D6673), now
  built as DXF tags, and read back (the style and piece system text, cutting and sew lines,
  notches, the grainline and internal lines; lightweight polylines and pieces placed away from the
  origin too), which every file written is checked against. VectorCraft's DXF writer
  (`vectorcraft-cad`) is back as it was. Checked again with ezdxf 1.4.5: the skirt block's pieces
  in R13 and R12, no errors, no fixes, the same extents. Tests: 5 in `pf-aama` (layers, true
  scale, read back as written, an insert and a lightweight polyline read, bad input), the vendored
  crate's 14, the 3 engine tests.
- 2026-10-21, **PF-2.1**: SVG exports at true scale. VectorCraft wrote the SVG's size without a
  unit (`width="283.465"`), which every SVG reader takes as CSS pixels, 96 per inch: a 10 cm
  square came out 75 mm in browsers, Inkscape and cutting software. The size is now in points
  (`width="283.465pt"`, the viewBox unchanged), so it measures 10 cm everywhere. Opening such a
  file again reads a size in points that matches a `0 0 w h` viewBox (our own files) as before, a
  user unit a point, so nothing goes through usvg's pixels and back. Test:
  `crates/svg/tests/true_scale.rs` (the 10 cm test square measured by usvg at 96 per inch: 100 mm;
  75 mm with the fix undone), and the SVG tests that drew exported files with usvg now read them
  at 72 per inch (points as pixels), as they did.
- 2026-10-21, **PF-7.2**: SVG for cutting (`document.export {format: svgcut}`), for laser and
  blade cutters and plotters that take SVG. The page is sized in millimetres
  (`width="…mm"`, viewBox in mm, one user unit a millimetre) with a 10 mm margin round the
  pieces. Each piece's cutting line is a red hairline path (`id="cut-N"`, `data-name`) in a `cut`
  group, notches are blue strokes in `notches`, and with `marks` on (the default) a `draw` group
  carries the dashed seam line, grainline, internal lines and the piece's name and cut count; with
  `marks` off only the cuts and notches are written. Tests: `crates/engine/src/tests_svgcut.rs`
  (a 10 cm square with 10 mm allowance cuts at 120 mm and its seam line at 100 mm, read back from
  the file; without marks only cuts are written).
- 2026-10-21, **PF-7.3**: PDF for projector (`document.export {format: projector}`), for
  projecting a pattern onto fabric on the cutting table, made as projector sewists describe such
  files (https://www.craftstorming.com/2020/05/tips-for-using-pdf-sewing-patterns-on-a-projector):
  one page at 100% holding every piece, set side by side (`layout: row`, the default; blocks are
  drafted touching, so their allowances would overlap) or where they are drawn (`asDrawn`), with
  a margin round them (`margin`, 25 mm); cutting lines and notches 3 pt thick (`lineWidth`),
  dashed seam lines, grainlines, and names and cut counts in 10 mm type. PDF layers: a 10 cm grid
  (on), a 4 in grid (off), a 10 cm test square in a band above the pieces, and the pieces; with
  `invert`, white lines on a black page (a "Background" layer); with `mirror`, the page mirrored
  left to right for cutting with the fabric's wrong side up (the type stays readable). Tests:
  `crates/engine/src/tests_projector.rs` reads the PDF back: the test square measures 100 × 100
  mm, the cutting line of a 10 cm square with 10 mm allowance 120 mm, the layers and their
  visibility, inverted and mirrored pages, touching pieces set apart. Not yet: the projector
  view in the app itself (full-screen, calibrated to the cutting mat), pieces cut on the fold
  opened out, layers per size (PF-6).
- 2026-10-21, **PF-6.1**: grading by redrafting (**Pattern › Grade Nest**, `grade.nest`;
  `grade.clear`). A PatternFoundry draft follows its measurements (PF-3), so each size of the
  size chart is the same draft made again from that size's measurements (blocks, point formulas,
  pieces with their allowance, dart caps and notches); grading "allows the one pattern to be made,
  then sized up and down" (https://uphance.com/blog/pattern-grading-a-beginners-guide/). The
  sizes are drawn over each other as a nest on a locked layer "Graded Nest", a sublayer per size
  ("Size 12"), each size in its own colour and the size the document is drafted in heavier; the
  document's own measurements and pieces don't change. `line`: the cutting lines (default) or the
  seam lines; `align`: each piece's sizes lined up where they are drafted (default) or on a corner
  or the centre of their bounds, for every piece or per piece (the skirt: the back on its top
  left, centre back at the waist; the front on its top right). The result lists each size's
  pieces with their width, height and line length in mm, the grade check; a size that draws the
  same as the document (its chart row lacks the measurements the draft uses) is a warning. A
  PDF saved with layers carries a layer per size (PF-2's "layers per size"). The nest stores
  outlines only, never measurements. Tests: `crates/engine/src/tests_grade.rs` (a skirt in three
  sizes: each piece grows by about a quarter of the hip step; size 12 is the document's own
  cutting lines; measurements untouched; replaced, undone, cleared; PDF layers per size;
  alignment; bad requests). Not yet: grade rules per point (PF-6.2) for patterns not drafted
  from measurements, grade rules in DXF-AAMA files, printing one chosen size.
- 2026-10-21, **PF-6.2**: grade rules per point, for patterns that aren't drafted from
  measurements (`grade.setSizes`, `grade.setRule`, `grade.info`). As the cutting-room standard
  describes rule grading (ASTM D6673, from DH Patterns and Fit's overview,
  https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf), chosen
  points of a piece carry a grade rule: how far the point moves in x and y for each size. The
  document keeps a size range (smallest first) and its base size; a rule is either the same move
  at every size step from the base (`step: [dx, dy]`, mm, x right and y down) or each named
  size's move from the base (`sizes: {"L": [dx, dy]}`). Points between ruled points move in
  proportion to how far along the seam line they are (`pf_core::grade::moves`), so a curve
  between two graded points keeps its shape; with one rule the whole piece moves. `grade.nest`
  grades into the size range when there is one: each size is drafted from the size chart's
  measurements where the chart has it, then the rules move the points, so the two kinds of
  grading add up. The rules live on the piece (saved with it); the document is never graded
  itself. Tests: 4 in `pf-core` (sizes, regular and irregular rules, points in proportion,
  wrapping round the outline) and 4 more engine tests (`tests_grade.rs`: a square graded 90, 100,
  110 mm; irregular rules; rules on top of redrafting; bad sizes and rules). Not yet: a grading
  panel (rules from the canvas), grade rules written into DXF-AAMA files (the RUL table),
  printing one chosen size, rules copied between pieces.
- 2026-10-21, **PF-6.3**: one size of a graded pattern written for cutting. DXF-AAMA, SVG for
  cutting and the projector PDF take `size` (`document.export {format: aama, size: "14"}`): the
  pieces are written as the graded nest draws that size (drafted from the size chart's
  measurements where it has the size, moved by the grade rules where the size range has it),
  and the document stays in its own size with its own measurements; DXF-AAMA's sample size is
  the size written unless `sampleSize` says otherwise. Tests: 2 more in `tests_grade.rs` (a
  ruled square's cutting SVG at 130, 140 and 150 mm pages, the AAMA sample size, the projector
  page; the skirt's chart sizes exported redrafted, measurements untouched). Not yet, and why:
  graded DXF-AAMA (the base size with a grade rule table, the `.RUL` file) needs the rule
  table's exact syntax, which no public source found so far gives (the standard itself is
  sold by ASTM; DH Patterns and Fit's overview was unreachable): the owner's call whether to
  buy ASTM D6673 or wait.
- 2026-10-21, **PF-6.4**: the **Grading** panel (Pattern and Window menus): the size range typed
  as "S, M, L" with its base size (Set Size Range; without a base chosen, the middle size), the
  selected piece's seam line points each with its step rule (x and y, mm a size; × removes it;
  a rule naming each size shows as "per size"), and Grade Nest / Clear Graded Nest (a size the
  pattern can't make is said in the status bar). Everything goes through `grade.*`. Tests:
  `crates/ui-egui/src/panels/tests_grading.rs` (drawn headless: the hint without a piece, a row
  per point, the nest buttons, the typed size range sent). Not yet: picking a point on the
  canvas, per-size rules edited in the panel, rules copied between pieces.
- 2026-10-21, **PF-6.5**: grade rules copied between pieces (`grade.copyRules {from, to?,
  mirror?}`; the Grading panel's "Copy Rules To", with Mirrored). Each ruled point's rule goes to
  the target's point nearest the same place in its bounds (a share of the width and height), so a
  front takes a back's rules though its points differ (`pf_core::grade::copy_rules`); mirrored
  (x), places flip left to right and x moves turn the other way, for the opposite half drawn the
  other way round. The target's own rules are replaced. Tests: 1 in `pf-core` (nearest points,
  mirroring, both rule kinds), 1 engine test (a front graded from a back grows to the left
  mirrored, to the right not; refusals) and 1 panel test. Not yet: points picked on the canvas,
  per-size rules edited in the panel.
- 2026-10-21, **PF-8.1**: **Ask About Patterns** (Help and Pattern menus, `ui.askAboutPatterns`)
  and **Foundry Account** (Help menu, `ui.account`), adapted from StitchFoundry's AI client, as
  the owner approved (`crates/sf-core/src/ai` and `crates/ui-egui/src/embroidery/{ask,account,
  prompt}.rs` in github.com/Stitchwright/StitchFoundry). A new `crates/pf-ai` (standalone, no
  network code) builds the Messages API requests and reads the replies: the conversation, Claude's
  two tools (`find_commands`, `run_command`), the Foundry cloud service's requests (sign in, the
  account, checkout) and AI requests sent through it paid in credits (only the body keys the
  service accepts; model `claude-opus-5-5`, at most 16,000 tokens). The app sends them through the
  host: `ureq` on a thread on the desktop (`apps/vectorcraft/src/ai_service.rs`, with the API key
  and session in private files next to the settings), the browser's fetch on the web. Claude can
  look at the pattern and change it through PatternFoundry's commands: reads run at once, changes
  wait for the user's Allow (each an undo step), files, exports, preferences, plug-ins and windows
  are out of reach. Measurements are personal data: the pattern summary sent with a question
  names the pieces, blocks, sizes and which measurements are set, never their values, and
  measure.info (the only command that returns them) waits for the user's Allow like a change.
  Tests: 9 in `pf-ai`, 7 in `crates/ui-egui/src/ai/tests.rs` (with a fake service: the summary
  without values, reads at once and the skirt drafted after Allow, a declined read, what Claude may
  run, no key or question, the web's fetch, a request paid with credits going to the service and
  its out-of-credits refusal). Not yet: a real request checked against the service, drafting a
  whole garment from a description (the "wrap dress" of PF-8's done-when: needs a wrap bodice in
  the design tools), fit advice from muslin photos.
- 2026-10-21, **PF-5.16**: **Make Wrap Front** (`design.wrapFront`, Pattern menu), for PF-8's
  wrap dress, as Ploen Patterns drafts a wrap blouse from the bodice block
  (https://ploenpatterns.com/blogs/news/how-to-draft-a-wrap-blouse-step-by-step-pattern-making-tutorial):
  the bust dart moved into the armhole first ("so there is room for the wrap"), the half front
  opened out across centre front ("because the front is asymmetric, don't place it on a
  center-front fold"; the block reflected across centre front, as Sewing with Numbers drafts a
  wrap skirt, https://sewingwithnumbers.substack.com/p/how-to-draft-a-wrap-skirt), and cut along
  the wrap line from the shoulder 4.5 cm from the neck point to the far side's waist; `curve`
  bows the wrap edge into the piece (the source's gaping fix). The fold is found (the longest
  straight edge on the outline's left or right side) or named; `from`/`to` move the line. A
  piece becomes cut as a mirrored pair. A dart the wrap line cuts away is reported (move the bust
  dart out of the shoulder first), and a dart of the far side whose point falls past the line is
  left out. Geometry: `pf_core::design::{opened_out, bow}`. Tests: 1 in `pf-core`, 2 engine tests
  (`tests_design.rs`: the bodice block's wrap front, the warning, the line 45 mm along the
  shoulder, nothing past it, cut 2 mirrored, one undo; a bowed edge and refusals). Not yet: the
  ties and the waistband, the back's matching changes (the source's lowered armhole, extended
  shoulder).
- 2026-10-22, **PF-5.17**: the wrap dress's other pieces. **Make Wrap Front** with `edge:
  "straight"` makes a wrap skirt's front: the skirt block's front opened out across centre front
  and cut by an edge parallel to it, `overlap` past it, from the waist to the hem (Sewing with
  Numbers marks where "top wrap ends" on the waistline,
  https://sewingwithnumbers.substack.com/p/how-to-draft-a-wrap-skirt; Rookie's overlap panel
  extends past centre front, https://www.rookiemag.com/2015/06/diy-wrap-skirt/); without
  `overlap`, the whole front (to match a wrap bodice that ends at the far side's waist). An edge
  that runs down a dart's opening is warned about. **Make Band** (`design.band`) draws a straight
  band to make a piece: a waistband (Ploen Patterns: about 8 cm wide, 1.5 to 2 times the waist),
  ties (Rookie: 3.5 × 25 in), cuffs, with a dashed fold line along its middle. Tests: 1 more in
  `tests_design.rs`. So a wrap dress is: the bodice back; the front's bust dart rotated into the
  armhole, then Make Wrap Front; the skirt back; the skirt front with Make Wrap Front (straight);
  a waistband and ties with Make Band.
- 2026-10-22, **PF-8.2**: **Check My Muslin** (Help and Pattern menus, `ui.checkMuslin`), PF-8's
  fit advice, adapted from StitchFoundry's Check My Sew-out as the owner approved
  (`crates/sf-core/src/ai/sewout.rs`, `crates/ui-egui/src/embroidery/sewout.rs`). Up to six photos
  of a muslin tried on (turned upright, made no larger than Claude looks at, sent as JPEG), the
  pattern's summary when it is open (pieces, blocks, sizes and which measurements are set, never
  their values) and a few answers (the muslin's fabric, the final fabric, how it feels, notes) go
  to Claude with fitting instructions (`pf_ai::muslin`): read the drag lines and folds, say where,
  give each problem's cause and the pattern alteration with how much, talk about the garment
  never the body's looks, never guess measurements from the photos. The answer, and the
  alterations it offers (each after the user's Allow; reading the measurements too), are in the
  Ask About Patterns window in its muslin mode, and questions after it go with the same
  instructions. The photos show the user: they go with the check only, are let go of once sent or
  when the window closes, and nothing logs them. Tests: 2 in `pf-ai` (the first message, the
  request) and 2 in `crates/ui-egui/src/ai/tests.rs` (a photo made smaller, a file that isn't one
  refused, the check's message with no measured value, the alteration made after Allow, the
  follow-up's instructions; a check without a photo). Not yet: a real request checked against
  the service, the pattern drawn as a picture to compare with the photos.
- 2026-10-22, **PF-9.1**: marker making, **Pattern › Make Marker…** (`marker.make`). A marker is
  "a diagram of the precise arrangement of pattern pieces" for a cut, its efficiency the share of
  the fabric the pieces use
  (https://www.onlineclothingstudy.com/2011/11/marker-planning-and-marker-making.html). Every copy
  of a fabric's pieces (the cut count; a piece cut in mirror-image pairs gives half its copies
  flipped) is laid on the fabric's width with its grainline along the length, parallel to the
  selvedge (https://www.threadsmagazine.com/2018/02/22/fabric-grain-explained), either way up, or
  all one way for a nap or a one-way print (`oneWay`,
  https://www.threadsmagazine.com/2020/06/11/how-to-lay-out-patterns-on-napped-fabrics), `gap`
  apart (2 mm), optionally in one `size` of the range. The laying (`pf_core::marker::lay`) is a
  bottom-left fill (Hopper & Turton, EJOR 128, 2001): largest piece first, each where its end
  comes furthest along, overlap tested on a grid (`precision`, 5 mm) where each piece covers every
  cell its outline grown by half the gap touches, so pieces never overlap. The marker is drawn on
  the layer "Marker <fabric>" (made again it replaces it): the fabric's outline, a caption (width,
  length, % used), and each copy's cutting line, seam line, grainline and notches as plain art
  with its name. Pieces too wide for the fabric are listed. The bodice, skirt and sleeve blocks,
  two of each, take 1.37 m of 150 cm fabric at 68% (precision 3 mm). Tests: 3 in `pf-core` (rectangles fill the
  width then the length, concave pieces never overlap and stay on the fabric, one-way and too
  wide), 2 in `tests_marker.rs` (copies, mirror images, true-scale fabric, the lining's own
  marker, replaced and undone; the blocks on two metres, untouched when the measurements change).
  Not yet: folded fabric (pieces cut on the fold laid against the fold), a small tilt off grain,
  plaid and stripe matching, pieces moved by hand with the length and efficiency kept up to date,
  a better search (trying orders, as nesting software does), the marker in DXF-AAMA.
- 2026-10-22, **PF-9.2**: pieces cut on the fold, and markers on folded fabric. A piece's fold
  (`piece.set {fold: edge | "auto" | null}`; the Pieces panel's **Cut on fold**, "auto" the
  longest straight edge) is a straight edge of its seam line: it takes no allowance and the label
  says "Cut 1 on fold". **Make Marker** lays such a piece opened out across the fold, whole, with
  one cutting line round it and the seam line and grainline on both halves
  (`pf_core::marker::opened_out`). With `folded` (the fabric folded lengthwise, selvedges
  together, the usual home layout: https://www.threadsmagazine.com/2009/06/30/the-fold-line) the
  marker is half the width with the fold dashed along its top edge: pieces on the fold lie against
  it, and every other piece cuts a mirror-image pair through both layers, so a piece cut 2 is laid
  once (an odd count is warned about: it cuts one more). Tests: 2 more in `pf-core` (a label on
  the fold; opening out, lying against the fold) and 1 in `tests_marker.rs` (no allowance on the
  fold, opened out single, against the fold when folded, pairs, the odd count, the fold taken
  off). Not yet: the fold in the projector PDF and in DXF-AAMA (its mirror line), a fold that is
  not straight.
- 2026-10-22, **PF-7.4**: the fold in the exports. **DXF-AAMA** writes a piece cut on the fold as
  the half with its fold as the mirror line, a LINE on layer 6 ("Mirror line", "Symmetry of
  fold" in Patro's layer table,
  https://fabricesalvaire.github.io/Patro/resources/file-format/dxf-astm.html), and reads it back.
  The **projector PDF** shows such a piece opened out, whole, for fabric laid out single (the
  cutting line round the whole, the seam line, notches and inner lines on both halves, the fold
  drawn inside), or with `openFolds: false` the half with its fold dashed, to lay on the fold of
  folded fabric; its label says "on fold". The **SVG for cutting** keeps the half where it is
  drawn with its fold dashed in `draw` (opening it out could cut into a neighbouring piece) and
  cuts it whole with `openFolds: true`. Tests: 1 more each in `pf-aama` (the mirror line),
  `tests_aama.rs`, `tests_projector.rs` (opened out 220 mm, the half 110 mm, the 10 cm square)
  and `tests_svgcut.rs`. Not yet: drill holes, the other mirror-line uses in AAMA (symmetric
  pieces drawn as halves without being cut on the fold).
- 2026-10-22, **PF-4.4**: the armhole checked against the sleeve head. **Pattern › Check Sleeve
  Ease** (`ui.checkSleeveEase`; the query `design.sleeveEase`) measures the sleeve's crown from
  underarm to underarm and the back and front armholes from the shoulder points to the side seam,
  and tells the ease the crown takes in against the range for the sleeve's style: the University
  of Kentucky extension's figures, measured notch to notch, 1½ to 2 in for a classic set-in sleeve,
  ¾ to 1 in for a shorter casual cap, ½ to ¾ in for a shirt sleeve, less for knits (FCS2-807,
  https://publications.ca.uky.edu/sites/publications.ca.uky.edu/files/fcs2807.pdf), or `min` and
  `max` given. Too little: the sleeve may pull across the cap; too much: the cap may pucker. It
  gives the `sleeve_crown` that redrafts the sleeve block with the ease in the middle of the range.
  The drafted blocks are found by themselves (`pf_core::block::armhole`: where each part's
  armhole runs, while the part has its drafted anchors); a block changed since is named by hand
  (`{id, from, to}`, as Seam Length). It showed that the sleeve drafted from the top arm table
  alone has no ease against the bodice block with the same made-up measurements (crown 430.5 mm,
  armhole 432 mm): setting `sleeve_crown` as it says fixes that. Tests: 1 in `pf-core` (the
  anchors against the drafts), 1 in `tests_block.rs` (found and named seams agree, the
  `sleeve_crown` it gives brings the ease into range, styles, two sleeves), 1 in the UI (the
  message). Not yet: notches placed for the ease (front and back balance marks), two-piece
  sleeves.
