# 2026-10-24: draping a bodice and sleeves (PF-10.6)

Goal: the bodice block (back and front, each on the fold) sewn at the sides and shoulders, and
the sleeve block (cut as a mirror-image pair) sewn into its armholes, draped on the avatar.

Repro: `drape_probe.py` in the session scratchpad (`patternfoundry-cli run` with the blocks drafted
from one measurement set, chosen seams sewn, `drape.make`, the view exported), and switchable
probes in `drape.rs` (since removed): each seam's widest gap at the start and the end, the most
stretched triangles and where they are, which arm each sleeve went to.

## Leads and experiments

1. **Bodice speckled, every seam 11–15 mm open, stretch 50%.** Hypothesis: the shoulder seam
   with a dart in it is mis-sewn. Ruled out: with only the side seams sewn and nothing to hold the
   cloth (it fell through the floor), the side seams still stayed 14.6 mm open. So the stitches
   contradicted each other: pairing *the particles nearest points along the two runs* sewed one
   particle to two a step apart where the two edges slant differently (the front and back side
   seams do; a skirt's two panels are mirror images, so it never showed). Fix: pair the
   particles that lie along each run by how far along it they are, in order, each partner once.
   Free fall: 14.6 → 0.5 mm; the bodice: seams within 6.5 mm, stretch 14%.
2. **Bodice rides 48 mm above the waist.** Friction off for the first third: no change (it hangs
   from the shoulders, it isn't stuck). The block's back is 395 mm at the centre back for a
   410 mm nape to waist; checked against the published draft (fashioncapital.co.uk): the source
   measures the waist from its top line, not from the nape point, so the block is right. The
   avatar took the nape to waist (measured down the curve of the back) as a straight height:
   ANSUR II gives 425.4 down the back for 415.5 straight up (BACK_RISE = 0.977). Without the darts
   sewn the bodice dropped 25 mm *below* the waist: the darts lift it, as cones over a bust the
   avatar's elliptic chest doesn't have (ANSUR II has no underbust or bust point measurement to
   model one from). Left as a known limit: it now sits about 15 mm high.
3. **Sleeves: the top slid off to the hips.** Starting the seams gently (sewing phase), friction
   from the start, a cap-shaped crown: none kept it up. Without gravity at all it still came
   apart (shoulder seams 40 mm open): not weight, conflicting seams. Cut down: the underarm seam
   alone is fine; both crown halves alone tear the shoulders. The arms hung clear of the chest, so
   the middle of each crown half started ~70 mm from the armhole it is sewn to and the arm (a
   solid) kept the sleeve out: the bodice was dragged out to it. Fix: the arm grows out of the
   shoulder (at the armpit it overlaps the side of the chest by 0.45 of its radius) and hangs at
   an A pose of 30° so it parts from the body below the armpit at once (at 10° the sleeve's tube
   was squeezed between arm and chest; 15° and 20° let it slide off again). Also: the armpit at
   the measured armscye depth (the bodice's underarm is drafted there); each sleeve turned so the
   crown half sewn to the front armhole faces the front (the mirrored copy was twisted); the
   first third sewn up without gravity, the seams pulling from a hundredth as hard to full.
4. **Still: the armpits and the top of the bodice read tight (red) with sleeves in.** Not the
   ease (a crown no longer than the armhole was worse), not the order of the push-outs where
   arm and chest meet (pushing out again until clear of both: no change). The most stretched
   triangles are at the armpit, the cloth pinched where the avatar's arm and chest (two lofted
   tubes) meet. Left as a known limit; the fix is a better avatar there.

## Ruled out
Seam pairing direction (checked by hand for each run), the fold opening (both pieces opened,
all four darts found), which arm each sleeve goes to (by its armhole), the crown's ease.

## 2026-10-25: the avatar's bust and armpit (PF-10.8)

One change at a time, the bodice block alone and with sleeves (worst stretch / tightest):

| change | bodice | with sleeves | sleeves stay on |
|---|---|---|---|
| before | 34% / 33% | 208% / 638% | yes |
| bust forward of a flat back | 23% / 31% | 201% / 557% | yes |
| creases rounded (30 mm), no bust | 39% / 47% | 141% / 181% | no: top 1247 |
| bust + 15 mm rounding | 28% / 36% | 121% / 145% | no: top 1262 |
| bust + 8 mm | 25% / 34% | 206% / 645% | yes |
| arms at 15° or 20° + rounding | 33–52% | 135–175% | no: the sleeves slide off |

The rounded crease eases the armpit but the armhole, a loop round the angled arm, slides down
it: the crease was what held it. Kept: the bust. The armpit wants a real body mesh.
