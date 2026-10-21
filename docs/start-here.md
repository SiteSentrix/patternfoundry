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
  `embroidery.*` commands, the AI features and the Foundry Account window), and its CI, release and
  translation set-up (23 complete languages).
- **StitchFoundry cloud** (cloud.stitchfoundry.app): the shared **Foundry Account** (one sign-in and
  one credit balance for every SiteSentrix app), credits, Stripe, AI (see Hosting, accounts and AI
  below).
- Websites on Cloudflare Pages from bwattendorf/sitesentrix-web; email through Cloudflare Email
  Routing and Resend; hosting on Railway Pro.

## Decided
- Name: **PatternFoundry**; sewing-pattern design software; built on VectorCraft like
  StitchFoundry.
- We build our own and never copy (see `CLAUDE.md`): commercial software is studied only through
  its public documentation, to match and beat its features.

## Hosting, accounts and AI (owner's decisions, 2026-10-19)
- **Hosting:** GitHub Pages for the web build and GitHub Releases for the desktop downloads, as
  StitchFoundry does. PatternFoundry has no server of its own and nothing on Railway.
- **Accounts and AI:** the shared **Foundry Account** (one sign-in and one balance of "Foundry
  credits" across all SiteSentrix apps), served by the StitchFoundry cloud service,
  https://cloud.stitchfoundry.app. PatternFoundry never holds an Anthropic key: AI requests go
  through the service, which adds the owner's key and charges the user's credits. Users may still
  enter their own Anthropic key in the app's AI settings instead.
- **Building the AI features (PF-8):** reuse StitchFoundry's client code
  (`crates/sf-core/src/ai/cloud.rs`, and the Foundry Account window in
  `crates/ui-egui/src/embroidery/account.rs`), and point builds at the service with `SF_CLOUD_URL`,
  as StitchFoundry's build workflows do.
- **The service:** `POST /v1/signup`, `/v1/signin`, `/v1/signout`, `/v1/account`,
  `/v1/checkout {pack}` and `/v1/messages`, with a bearer token `sfs_…`. `/v1/messages` accepts
  only `model`, `max_tokens`, `system`, `messages`, `tools`, `tool_choice`, `output_config`,
  `fallbacks` and `thinking`; no streaming; `max_tokens` at most 16,000; the model must be one the
  service allows (today `claude-opus-5-5`). For another model or limit, ask the StitchFoundry chat:
  the service lives in github.com/Stitchwright/StitchFoundry, `apps/stitchfoundry-cloud`, documented
  in its `docs/cloud.md`.
- Body measurements still stay on the user's computer: an AI request sends only what that request
  needs, and nothing is logged (CLAUDE.md).

## The owner decides (ask, don't assume)
Money, legal wording, anything public, domains, and the open decisions in `docs/plan.md`.

## Next task
PF-0 to PF-4 are done (with PF-4.1 and PF-4.2, the sleeve and trouser blocks) and PF-5 has begun
(dart rotation, splitting, slash and spread, seam length and matching, walking seams, smoothing
across seams, facings, knife, inverted and box pleats, gathering marks, flat and stand collars,
patch pockets; see the progress log in `docs/plan.md`). Next is the rest of **PF-5**: shirt
collars, and in-seam and welt pockets.
