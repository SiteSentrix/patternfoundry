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
