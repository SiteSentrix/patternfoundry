# Start here: notes for a new session

Written 2026-10-19 from the StitchFoundry session where PatternFoundry was decided. Read this,
then `docs/plan.md` and `docs/market.md`.

## The owner
**Bruce Wattendorf**, doing business as **SiteSentrix** (Rhode Island). He runs StitchFoundry
(embroidery digitizing) and is building a family of maker software: StitchFoundry,
LightFoundry (laser), MakerFoundry (marketplace) and PatternFoundry (sewing patterns). Works from
a Windows PC and the Claude app on his phone; plain, step-by-step instructions work best for
anything he clicks through.

## What exists to build on
- **VectorCraft** (github.com/storytold/vectorcraft): the Rust vector editor to merge in (PF-0).
- **CADCraft** (github.com/storytold/cadcraft), the ArtCraft team's Rust CAD app, under the same
  licence as PatternFoundry (MIT OR Apache-2.0), so its code may be reused with its copyright notice
  kept (in `NOTICE`) and none of the ArtCraft branding. Worth taking rather than rewriting (owner's
  pointer, 2026-10-19): `crates/constraints` (a parametric solver for geometric and dimensional
  constraints with expressions: PF-3 formulas and PF-5 drafting), `crates/dxf` (a standalone DXF
  tag reader/writer from the published DXF reference: the base for DXF-AAMA in PF-7), and its
  paper-space layouts, page setups and plotting to PDF (PF-2). Leave its `dwg` crate (it wraps the
  MPL-2.0 `acadrust`); DWG is not on our plan.
- **StitchFoundry** (github.com/Stitchwright/StitchFoundry): VectorCraft plus embroidery; the
  model for how a mode is added (its `docs/stitchfoundry-spec.md`, `crates/sf-core`, the
  `embroidery.*` commands, the AI features and StitchFoundry Account), and its CI, release and
  translation set-up (23 complete languages).
- **StitchFoundry cloud** (cloud.stitchfoundry.app): accounts, credits, Stripe, AI.
- Websites on Cloudflare Pages from bwattendorf/sitesentrix-web; email through Cloudflare Email
  Routing and Resend; hosting on Railway Pro.

## Decided
- Name: **PatternFoundry**; sewing-pattern design software; built on VectorCraft like
  StitchFoundry.
- We build our own and never copy (see `CLAUDE.md`): commercial software is studied only through
  its public documentation, to match and beat its features.

## The owner decides (ask, don't assume)
Money, legal wording, anything public, domains, and the open decisions in `docs/plan.md`.

## Next task
PF-0 is done (see the progress log in `docs/plan.md`). Next is **PF-1**, pattern pieces: piece data
on paths (name, cut count, fabric, grainline), seam allowance per edge with mitred or squared
corners, notches, labels and a Pieces panel, as `pattern.*` engine commands with tests, in a
pattern core crate below the UI (as StitchFoundry's `sf-core`). Look at CADCraft first for what can
be reused (see above).
