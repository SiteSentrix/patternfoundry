# PatternFoundry — instructions for agents

PatternFoundry is **sewing-pattern design software** by **SiteSentrix**: draft garment patterns
from body measurements, adjust and grade them, add seam allowance, notches and grainlines, and
print them at true scale (home printer tiles, copy-shop A0, projector) or export them for cutting.
Siblings: [StitchFoundry](https://github.com/Stitchwright/StitchFoundry) (embroidery digitizing,
built on VectorCraft), LightFoundry (laser) and MakerFoundry (the marketplace, where patterns can
be sold too).

**Planned base:** like StitchFoundry, PatternFoundry is VectorCraft plus its own mode: a
clean-room, Rust-native vector illustration app (macOS, Windows, Linux, web/WASM). Pattern
pieces are paths; drafting, grading and printing are commands. Merging VectorCraft in is the
first task (`docs/plan.md`, PF-0). Once it is in, **every rule in
[`docs/vectorcraft-CLAUDE.md`](docs/vectorcraft-CLAUDE.md) applies here unchanged** (Adobe
clean-room, assets, no panics, everything is a command, layering, thin UI, Rust only, never
break wasm, `cargo xtask ci` before every commit, progress docs), with "StitchFoundry" read as
"PatternFoundry".

## Start every session here
1. Read [`docs/start-here.md`](docs/start-here.md) (the owner, what exists, decisions), then
   [`docs/plan.md`](docs/plan.md) (the product, the milestones and task ids) and
   [`docs/market.md`](docs/market.md) (the software already out there, as features to match).
2. Unless the user gives you a task, take the next unfinished one. Orient → plan → implement +
   test → verify → record → commit.

## Non-negotiables for pattern work
- **Clean room, no copying.** We build our own software that matches and beats what is out
  there; we never copy it. Behaviour comes only from public documentation (vendor help pages,
  manuals, published standards, public videos and forum posts), cited by URL. Never install,
  run, decompile or measure commercial pattern software (Gerber AccuMark, Lectra Modaris,
  Optitex, Tukatech, CLO, Wild Ginger, Garment Designer, PatternMaker…) to recreate it, and never
  copy their code, text, icons, block libraries or sample files. Never copy GPL/AGPL code:
  **Seamly2D and Valentina are GPL**, so read their public docs for ideas only. Never copy
  Roidery code. Sewing-pattern drafting methods from books and schools may be implemented from
  their published descriptions; their text and drawings are not copied.
- **True scale is sacred.** A pattern printed at 100% must measure exactly: every print and
  export path has a test that checks a 10 cm test square, and tiled pages carry alignment marks
  and the test square. Units are exact (mm internally; inches shown when chosen).
- **Measurements are personal data.** Body measurements stay on the user's computer unless they
  choose to share them; never log them.
- **Standards.** Exports follow the published specifications (DXF-AAMA/ASTM D6673 for cutting
  rooms, PDF/ISO 32000, SVG), tested against files written from those specifications.

## Debugging
When something fails and the cause isn't obvious, follow the `/debug` skill
(`.claude/skills/debug/SKILL.md`, based on Kuhlmann, *The Book of Debugging*). What matters most:
- **Reproduce first, and make the loop fast**: one named input and one command; never the full CI
  as the loop. Run the one failing test, or the command on its own with the CLI; for scale bugs measure the 10 cm square in the output file.
- **Look at what the program actually did** before theorising, and build a flag or command when you
  keep wanting to see something.
- **One hypothesis per experiment**, written down with what either result will teach; cap guessing
  at about an hour, then switch strategy (bisect, instrument, follow the data).
- **Verify the fix** by undoing it (the bug returns) and redoing it (it goes); keep the repro as a
  regression test; look for the same mistake elsewhere.
- Keep a debugging log in `docs/debug-logs/YYYY-MM-DD-<topic>.md` for anything that takes more
  than one sitting, and reference it from the commit.

## Commit authorship
Commits are authored by **Bruce Wattendorf**; AI assistance is a trailer, never the author.
- `git config user.name "Bruce Wattendorf"` and `git config user.email "bwattendorf@gmail.com"`
  before the first commit (or `--author="Bruce Wattendorf <bwattendorf@gmail.com>"`).
- End every commit message with: `Co-Authored-By: Claude <noreply@anthropic.com>`
- One task id per commit (`PF-1.2: seam allowance`). No model names in commits, code or docs.

## Licence
MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). VectorCraft's own licence and brand terms
apply to the merged code: carry no ArtCraft name, wordmark or logo (saying in plain text that
PatternFoundry is based on VectorCraft by the ArtCraft team is fine).
