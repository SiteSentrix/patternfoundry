# 2026-10-24: the draped skirt falls off the avatar (PF-10.3)

**Symptom.** `drape.make` on two A-line panels sewn at the sides: the skirt ends 25 m below
the floor (`top: -25070`), its seams closed, hardly stretched at the end.

**Reproduced** with the narrowest test (`cargo test -p vectorcraft-engine tests_drape`), and a
probe test printing the result after 1, 10, 30, 60 and 120 steps.

| Run | Change | What it showed |
| --- | --- | --- |
| 1 | none (probe) | At step 10 the top is already 150 mm down: free fall (½gt² = 136 mm in a sixth of a second). At step 1 the worst link is stretched 309%. |
| 2 | each row wrapped round its own girth (not one circle as wide as the hems) | step 1 stretch 92%, still free fall. |
| 3 | friction on contact, four substeps | no free fall, but it slides down and stretches more and more (109%). |
| 4 | the test skirt checked against the body: 880 round at the hip line against 960 hips | **the test garment didn't fit** (it could only go on stretched). Made 800 at the hem: 1013 at the hip line. Still slid off. |
| 5 | probe of the stretch right after placing | 61% on a diagonal at the slanted side: rows' widths taken from the particles' staircase, which jumps a step at a time. Taken from the outline's crossing at the row's height: 20%. Still slid. |
| 6 | hypothesis: ten iterations can't hold a ring of 3000 particles at its length, so the body (projected last) stretches it over the hips. Test: 40 iterations, nothing else | **settles 18 mm below the waist and stays** (the same at 60 and 120 steps). |

Also found on the way, in `pf_core::cloth`'s collider:

- it pushed cloth out sideways only, which gives no hold where the body widens downward; now along
  the surface's normal (Newton steps on the ellipse function, the slope taken across 1 mm);
- Newton's steps on the squared ellipse function overshoot from inside (a point at 63 of a 127
  waist went to 161); on the distance as a share of the way out (its square root) they don't.

**Root cause.** Too few constraint iterations for a ring of cloth against a body that widens
below it; the collision projection, last in each round, wins and stretches the ring. Fixed with
40 iterations a substep (and friction, the normal push and the row placing, each a real fault of
its own). Left: the seams pull particles a grid step inside a slanted side together (12 mm gap,
41% on one link at a seam); particles on the outline itself would fix it.

**Ruled out.** The avatar's rings (heights and girths checked in `pf-core`'s avatar tests), the
seam pairing's direction (the probe shows the seams closed).

**Follow-up (the seam gap).** The particles at the grid's edge are now moved onto the outline
(within a step, a quarter step apart): the skirt's widest seam gap went from 12 mm to 2 mm and its
worst stretch from 41% to 7%. It now hangs 652 mm from its top (the cloth reaches its hem line,
and lengthens a few per cent under its weight), where it hung about 600 with its edges a step in.

**Follow-up (the drape view's seam strip, PF-10.4).** The first drape view showed a speckled
strip of red, green and beige along the side seam at precision 15.

| Run | Change / probe | What it showed |
| --- | --- | --- |
| 1 | hypothesis: the seams pair particles at different heights; pair the edge particles by their place along the run | worse (stretch 48% to 76%): reverted. |
| 2 | precision 20 with 240 steps (one change: the precision, not the steps) | seam 2 mm, stretch 7%: the precision, not the time. |
| 3 | hypothesis: particles moved onto the outline near corners fold triangles; put such moves back | no change (seam 8.8 mm): reverted. Two guesses wrong: follow the data. |
| 4 | probe: the widest seam pair and the worst link, with their places on the pattern | the worst link was 5 mm long (two edge particles moved close together): 3 mm on it read as 56%; the red speckles were slivers. The 8.8 mm gap was one pair at the hem's corner. |
| 5 | precision 10 and 25 | 25 holds, **10 slides off** (7000 particles): the iterations a step must grow as the mesh gets finer. |
| 6 | the beige in the strip | grid cells cut by a slanted side had no triangle: a sawtooth of gaps. Filled with the triangle of their three corners inside: gone. |

Fixed: the stretch and tightness ignore the short links and slivers at the edge, cut cells get
their triangle, the iterations scale with (20 / precision)², the default precision is 20 mm and
the finest 15. Left: at 15 mm a seam corner can be off by a step (an 8.8 mm gap at the hem's
corner).
