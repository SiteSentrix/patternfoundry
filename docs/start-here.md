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
- **StitchFoundry** (github.com/Stitchwright/StitchFoundry): VectorCraft plus embroidery; the
  model for how a mode is added (its `docs/stitchfoundry-spec.md`, `crates/sf-core`, the
  `embroidery.*` commands, the AI features and the Foundry Account window), and its CI, release and
  translation set-up (23 complete languages).
- **StitchFoundry cloud** (cloud.stitchfoundry.app): the shared **Foundry Account** (one sign-in and one credit balance for every SiteSentrix app), credits, Stripe, AI.
- Websites on Cloudflare Pages from bwattendorf/sitesentrix-web; email through Cloudflare Email
  Routing and Resend; hosting on Railway Pro.

## Decided
- Name: **PatternFoundry**; sewing-pattern design software; built on VectorCraft like
  StitchFoundry.
- We build our own and never copy (see `CLAUDE.md`): commercial software is studied only through
  its public documentation, to match and beat its features.

## The owner decides (ask, don't assume)
Money, legal wording, anything public, domains, and the open decisions in `docs/plan.md`.

## First task
**PF-0**: merge VectorCraft in (`git remote add upstream https://github.com/storytold/vectorcraft`,
merge, rename the app and its installers to PatternFoundry as StitchFoundry's BR-01 did, keep
no ArtCraft branding), get `cargo xtask ci` green, push, and say what's next.
