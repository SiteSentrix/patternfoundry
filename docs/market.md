# The software already out there

**Status:** 2026-10-19, a first pass from public listings and roundups; prices and details to be
checked on each vendor's own pages (cite them here as they are checked). This is a list of
**features to match and beat** with our own clean-room code. We never install, run or copy any of
these (see `CLAUDE.md`).

## Home sewists and small studios
| Software | What it is | Price (as listed) | Notes |
|---|---|---|---|
| Wild Ginger (PatternMaker, Cameo) | made-to-measure pattern drafting from measurements; style options per garment | to check | long-standing home-sewing tool |
| Garment Designer (Cochenille) | drafting from measurements, with a symmetry function; sewing and knitting | from $199 one-time ([Capterra](https://www.capterra.com/p/156370/Garment-Designer/reviews/)) | |
| PatternMaker | start from a garment collection, or draft/alter with CAD | contact vendor ([Capterra](https://www.capterra.com/p/28892/PatternMaker/alternatives/)) | |
| Pattern Maker Professional | pattern drafting | from $60/user/month ([Capterra](https://www.capterra.co.uk/alternatives/5929/pattern-maker-professional)) | the only subscription price found so far |
| Sewist (web) | browser-based made-to-measure patterns | to check | |
| Seamly2D, Valentina | open-source parametric drafting (formulas from measurements), grading, layouts | free | **GPL: ideas from their docs only, never their code** |
| FreeSewing | open-source made-to-measure patterns as code (JavaScript) | free | check its licence before reading its code; Rust only here anyway |

## Industry (cutting rooms and brands)
| Software | What it is | Notes |
|---|---|---|
| Gerber AccuMark (now Lectra) | 2D/3D pattern creation, grading, marker making; AccuNest nesting | [roundup](https://www.fibre2fashion.com/industry-article/10161/advancements-in-pattern-engineering-software-in-apparel-industry) |
| Lectra Modaris | 2D/3D pattern making and grading; positions and grades logos in 2D and 3D | same roundup |
| EFI Optitex (O/Dev) | pattern design, grading (incl. angle and variable grading), digitizing, wide CAD import/export | |
| Tukatech TUKAcad | pattern design suite, bought or subscribed | [roundup](https://onlineclothingstudy.com/?p=10027) |
| PolyPattern | industrial grading and marking | from €1000 one-time ([Capterra](https://www.capterra.com/p/110451/PolyPattern/)) |
| CLO 3D | 3D garment simulation on avatars with 2D pattern editing, for fashion production | for later (3D fit preview) |

## Marvelous Designer (the one the ArtCraft community talks about)
The 3D clothing tool used for games, film and animation, by CLO Virtual Fashion (CLO 3D's
maker): draw flat 2D pattern pieces, sew their edges together, and drape the garment on a 3D
avatar with cloth simulation (fabric weight, stretch, folds, wind), then export it for game
engines and renderers. It is sold by subscription: a personal plan was $39/month or $280/year
in 2022 ([CG Channel](https://www.cgchannel.com/?p=121535); check today's prices on
marvelousdesigner.com/pricing), an indie studio licence was added in January 2026
([Digital Production](https://digitalproduction.com/2026/01/12/marvelous-designer-adds-indie-pricing-quietly/)),
and a student plan at $8.25/month in July 2026
([AAP release](https://www.aap.com.au/aapreleases/cision20260709ae01014/)).

What PatternFoundry takes from it (as features, from public docs and videos only):
- **2D ↔ 3D together**: edit a pattern piece and see the draped garment update.
- **Sewing lines** between edges (with seam length checks) that drive both the 3D sewing and
  the real-world sewing instructions.
- **Fabric presets** (cotton, denim, jersey, silk…) with weight and stretch.
- **Avatars** sized from the user's own measurements, so the 3D preview is a fit check, not just
  a picture.
- **Exports** for game and 3D artists (OBJ/glTF/FBX of the draped garment), a market Marvelous
  Designer owns and StitchFoundry's ArtCraft-adjacent users know.

Where PatternFoundry can differ: one app that makes **real, sewable, true-scale patterns** and
the **3D preview**, free, with AI help; Marvelous Designer's patterns are made for screens, CLO's
for factories.

## What they share (our feature list to match)
- Drafting from **measurement tables** (standard size charts and the user's own), with formulas
  so a pattern redrafts when a measurement changes.
- **Blocks** (bodice, skirt, trouser, sleeve) and design changes on them: dart moves, slash and
  spread, pleats, flares, collars, facings, pockets.
- **Seam allowance** per edge, **notches**, **grainlines**, drill holes, labels, piece info.
- **Grading** to a size range (rules per point, or by redrafting from each size's measurements).
- **Truing**: matching seam lengths, walking seams, smooth curves across seams.
- **Printing**: tiled home-printer pages with marks, copy-shop A0/36-inch, projector mode, layers
  per size.
- **Export**: DXF-AAMA/ASTM for cutting rooms and plotters, PDF, SVG.
- **Marker making / nesting** for cutting fabric efficiently.
- **3D fit preview** (industry), later.

## Where PatternFoundry can win
- Free and open source, on Windows, Mac, Linux and the web, with a modern vector editor
  underneath (VectorCraft).
- **AI help**, as in StitchFoundry: "draft a wrap dress with a shawl collar for my
  measurements", fit advice from a photo of a muslin, explaining each step to beginners.
- Projector-ready output and true-scale printing that just works.
- Selling patterns on MakerFoundry, and embroidery on pattern pieces with StitchFoundry.
