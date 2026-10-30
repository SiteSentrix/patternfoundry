# 2026-10-30: draped trousers sit 63 mm below the waist

**Symptom.** The trouser block (test body: waist 700, hip 960, crotch depth 260, waist to ankle
1040), draped on the avatar, settles with its top at 1037 mm, the avatar's waist at 1100.6: 63 mm
low all the way round (lowest at the front), its hems on the floor, the crotch reading 64% tight.
PF-10.10 put it down to the block's crotch seam (657 mm) being shorter than the avatar's crotch
(about 755 mm, waist to waist between the legs).

**Reproduce.** `scratchpad/trouser_probe.py` (the CLI: measurements, `block.trousers`, the two
pieces cut 2 mirrored, the side seams, inseams and centre seams sewn, `drape.make`); about a minute
a run.

## Leads and experiments

1. *The avatar's crotch is too long.* ANSUR II's crotch length from the navel is 0.609 of the
   buttock girth (women, 620.8 / 1021.2; men 0.611), about 585 mm on these hips; the avatar's
   torso kept the hips' full depth down to the crotch, about 755. Shaped the crotch ring to the
   survey's length (front and back drawn in below the hips). **Result: top 1037.0 → 1037.7.** No
   change: not (alone) what holds the trousers down.
2. *Where is the waistline low?* The cloth's top in 30° sectors: 1024 to 1038 everywhere, lowest
   in front. All round, not a dip at the centre front and back: not a crotch seam pulling the
   middle down (as far as this shows).
3. *Still sliding?* Steps 60 / 120 / 240 / 480: top 1045 / 1038 / 1038 / 1038. Settled.
4. *The crotch seam pulling?* Draped without the two centre seams: top 1020 (lower), tightest
   24% (from 64%), stretch 9% (from 30%). The centre seam strains the cloth but holds the trousers
   *up* a little; it isn't what pulls them down.
5. *Soft cloth?* Three times the corrections a step: top 1038.3, stretch 29.7%. The solver isn't
   too soft; the strain is geometric.
6. *The waistband?* Its top row's 3D length over its length on the pattern: +3%. Not stretched
   enough to explain 63 mm.
7. *Resting on the floor?* The hems are on the floor and the drop (63) is about the ankle bone's
   height (61). Without the floor (a probe): top 1035.8. Not the floor.
8. *Sewing or gravity?* Without gravity at all (a probe): top 1038.3, hems on the floor. **The
   sewing alone puts them 62 mm low.**
9. *Where is the trousers' crotch?* The cloth near the middle (|y| < 40) reaches no higher than
   about 775 mm; the avatar's crotch ring is at 840. The trousers' crotch hangs 65 mm below the
   body's, as much as their top. **Cause:** the thighs (rings centred at the hips' half breadth
   less the thigh's, so each reached 12 mm past the middle) overlapped across the middle just
   below the torso; the crotch seam, sewn shut there, was inside both thighs and pushed down out
   of them to where they part, taking the trousers down with it.

## Fix

- The thighs side by side: each top thigh ring half the hips' breadth less a 5 mm gap across,
  centred on its half, as round as the survey's thigh (so deeper than broad). Top 1037.7 →
  1092.7 (8 mm below the waist), tightest 64% → 39%, stretch 30% → 32%, the hems 47 mm up.
- Both changes are needed: with the thighs apart but the old crotch (full depth to the bottom,
  a probe), the trousers reach the waist (1100.6) but strain to 187% tight, 94% stretch, a seam
  44 mm open: the crotch 100 mm longer than the pattern's crotch seam. With the survey's crotch
  too: 39%, 32%, 6 mm.

## Ruled out

Solver stiffness, sliding (it settles), the waistband's stretch, the floor and the hems, gravity.
