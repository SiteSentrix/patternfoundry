---
name: debug
description: Systematic debugging workflow (reproduce, probe, examine, fix) for PatternFoundry. Use when a test or `cargo xtask ci` fails for a reason that is not obvious, a command gives the wrong result, a print or export does not measure true to scale, the app crashes or misdraws, a change "should have helped" and did not, or any bug resists the obvious approaches. Based on Kuhlmann, The Book of Debugging (No Starch Press); chapter numbers below refer to it, and docs/debug-book-notes.md has the detail.
---

# Debugging workflow

Use this when something fails and the cause is not obvious. The book's core claim: the computer is
deterministic, so every bug has a cause that can be found. A workflow keeps effort from scattering.
Loop through the four steps until the bug cannot be reproduced; always start with Reproduce. The
detail behind each step, with page numbers, is in `docs/debug-book-notes.md`.

## 0. Before anything else

- Take responsibility: assume the bug is in our code, not in egui, wgpu, a crate or "the file" (Intro).
- Open or create a debugging log for this bug (see Log below) and write down the symptom, what you
  expected, and the exact command that shows it. Symptoms only; keep theories out of that line (Part I).
- Do one bug at a time. Finish or park it before starting another (ch. 18).
- **Check you are running what you think** (p. 8): make an obvious change (a log line, a wrong
  label) and confirm it shows up. Stale `target/`, another app on the control port (7979 is shared
  by the sibling apps' agents), an old export file or a cached web build all fake results.

## 1. Reproduce (ch. 1-3)

- Get the failure to happen on demand, from a named input and a command line. A result nobody can
  re-run is not a result.
- **Shorten the loop first.** `cargo xtask ci` takes a long time; reproduce with the narrowest command
  that still fails: `cargo test -p <crate> <test name>`, then `patternfoundry-cli run --in FILE --cmd
  ID --params JSON --export out.png`, or JSON lines on `--control 7979` against the running app.
  Fast loops are worth more than clever theories.
- **Cut the case down** (ch. 20, minimal repro): the smallest document, path, piece or command
  sequence that still fails. If it stops failing, the part you removed matters; put it back in halves.
- Vary one thing at a time to learn which steps matter: order, units (mm or inches), zoom, page
  size, platform (native or wasm), language.
- Write down how often it happens (attempts and failures).
- Automate the repeat. Keep the repro as a test once the bug is fixed (ch. 3, 25): a unit test in
  the crate, or an engine test that runs the commands.

## 2. Probe: find an entry point (ch. 4-9)

- Make an educated guess, but cap the time: stop after about an hour, or after two or three shortcuts
  that did not pan out, or when the guesses stop being based on evidence (ch. 4). Switch strategy then.
- Rule systems out as well as in. Check the code that has broken before and the glue between layers
  (a command's params, a tool's Begin/Preview/Commit, the UI's `app.run`) before old trusted code.
- **Bisect when it used to work** (ch. 6): find a good commit and a bad one and `git bisect` between
  them (an upstream VectorCraft merge is a natural boundary); then read the offending diff for the one
  change that matters.
- **Instrument before theorising** (ch. 8): assertions as early as possible, check every return
  value, make bad data look obviously bad. Keep instrumentation optional (`log::debug!` under
  `RUST_LOG=patternfoundry*=debug,vectorcraft*=debug`, an env switch), off by default, and remove
  probes that are not switchable (p. 69). No panics in shipped code, even in a probe. Never log body
  measurements (CLAUDE.md). Re-check the bug still happens with the instrumentation in.
- **Remove randomness** (ch. 9): fixed seeds, fixed order, no clock. A "random" result has a cause.
- Look at what the program actually produced before reasoning about it (ch. 15): `ui.screenshot` or
  `ui.render` and read the PNG, `doc_inspect` / the command's JSON result, the exported SVG or PDF
  text, the log file (`<settings dir>/logs/patternfoundry.log`).
- **Stub the subsystem** (p. 212): give a suspect layer known-good answers to rule it in or out.

## 3. Examine: find the root cause (ch. 13-19)

- **Follow the data** (ch. 15): find the original input, follow it through each transformation (file →
  document → command → geometry → render or export), find where it first becomes bad ("last good,
  first bad" along the data flow, p. 126), compare good and bad cases side by side.
- **Question everything** (ch. 16): list your assumptions and verify each one. Do not trust a feature
  because it exists; check that it actually fires (the command ran, the branch was taken, the option
  reached the writer). A symptom can sit far from its cause.
- **True scale:** when a size is wrong, check the units at every step (mm internally, points in the
  document, inches only for display) and measure the 10 cm test square in the output itself.
- **Experiment with a hypothesis** (ch. 17). For each experiment write down: what is the hypothesis,
  how will it be tested, and what will be learned either way. Test with the least effort. Try the
  hackiest possible fix first to confirm you found the right place, then do the clean one.
  Make experiments revertible: keep a clean tree so `git diff` shows them.
- **Do not revert a failed experiment as "no good" until it has been re-tested on a baseline known
  to be correct.** If a bug in the baseline is found later, re-run the reverted experiments.
- **Keep a debugging log** (ch. 18): every idea, every result, ruled-out areas, a prioritised list of
  next leads so there is always a plan. Reference it from the commit.
- **Build tools** (ch. 19): when you keep wanting to see something, make a command, a CLI flag or a
  control method for it, and document it. A hidden tool does not help.
- Talk it through (ch. 23) when stuck: stating the problem (in the log, or to the user) often
  exposes the wrong assumption. Ask after about 20 minutes of real effort without progress (p. 80).

## 4. Fix (ch. 23-30)

- A great fix is minimal, complete, revertible, and addresses the root cause, not the symptom.
  One bug per change, with its task id (ch. 23, 24).
- **Verify** (ch. 23): reproduce several times; undo the change and confirm the bug returns; re-apply
  and confirm it goes. Test the fix alone, without other local edits. Then `cargo xtask ci`, and for
  print or export paths the 10 cm test-square test.
- Check side effects separately from the fix (ch. 24), including **dependent behaviour** (p. 198):
  code that compensated for the bug may now be wrong. Never break wasm (`cargo xtask wasm`).
- **Share what you learned** (ch. 25): commit message with root cause, symptom and solution; update
  the progress docs; ask how the cause got in (ignorance or oversight, p. 187) and what safeguard
  prevents it (an assert, a test, a lint, an `xtask` check). Turn the repro into a regression test.
- **Search for related problems** (ch. 26): the same mistake may exist elsewhere. Fix the family while
  the context is fresh; make bulk changes with care.
- If a clean fix is not possible yet: deploy a quick fix or workaround and say so (ch. 28), or rewrite
  the offending piece from scratch and keep the old one as an oracle to compare against (ch. 29; the
  oracle is our own code or a published spec, never another vendor's program), or change the
  requirement (ch. 30). Disable rather than delete, and record the debt (pp. 215-219).

## Anti-patterns (avoid)

- Changing things one at a time hoping a test turns green (desperate guesses, ch. 4).
- Reverting an idea on one bad run, and trusting a baseline that had a latent bug.
- Reading a pass or a count as proof without looking at why.
- Running the full `cargo xtask ci` when one `cargo test -p … <name>` answers the same question.
- Claiming a feature works because it compiled; check that it ran and produced what you think.
- Skipping, ignoring or loosening a test to get green.

## Debugging log template

Create `docs/debug-logs/YYYY-MM-DD-<short-topic>.md`:

```
# <topic>  (command, input, commit)
Symptom (objective, no theory):
Expected:
Repro command + input:   (fast version: ...)
Frequency (attempts / failures) and measured numbers:

## Leads (prioritised: effort vs chance)
1. ...

## Experiments
- [date] Hypothesis / test / result / what it rules in or out (attach file paths, numbers)

## Ruled out
- ...

## Root cause (when found) / how it got in / fix / verification / related places checked
```

Every number in the log names its command and input and says whether it was measured or estimated.
Logs never hold anyone's body measurements.
