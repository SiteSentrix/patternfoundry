# PatternFoundry: the plan

**Status:** 2026-10-19. Idea and plan only; no code yet. Owner: Bruce Wattendorf (SiteSentrix).

## What it is
Software for designing sewing patterns, for home sewists, small pattern designers and small
studios: draft from measurements, change the design, grade sizes, and print or export at true
scale. Free and open source; AI help paid with StitchFoundry-style credits.

## How it is built
VectorCraft (github.com/storytold/vectorcraft), the clean-room Rust vector editor StitchFoundry
is built on, plus a pattern mode, the way StitchFoundry adds embroidery:
- pattern pieces are paths with pattern data (seam allowance per edge, notches, grainline,
  piece name, cut count, fabric);
- drafting, grading, truing and printing are engine commands (`pattern.*`), so the menus, the
  control channel, the CLI and MCP all reach them;
- a pattern core crate (measurements, formulas, blocks, grading) below the UI layers, as
  StitchFoundry's `sf-core` is.

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

## Open decisions (the owner's)
- Whether PatternFoundry is its own app (as planned) or a mode inside StitchFoundry.
- Which drafting methods to implement first (by the books or courses the owner's users follow).
- Pricing of AI help. (Decided 2026-10-19: AI help is paid with the shared **Foundry Account** and its Foundry credits, served by the StitchFoundry cloud service, not a separate account.)
- Domain: patternfoundry.app.
