//! An avatar from body measurements (PF-10.1): the body the 3D fit preview drapes a pattern on.
//!
//! The body is lofted through horizontal cross-sections (rings) at the heights of the body's
//! landmarks: the base of the neck, the bust, the waist, the hips and the crotch on the torso, and
//! the thigh, knee, calf and ankle on each leg. Each ring is an ellipse as round as the body is
//! there (its depth over its breadth) and exactly as long round as the girth measured, so a
//! garment drafted from the same measurements meets the body where the tape went.
//!
//! Proportions the measurements don't give come from the U.S. Army's ANSUR II survey of 1,986
//! women (2012, public data:
//! <https://ph.health.mil/topics/workplacehealth/ergo/Pages/Anthropometric-Database.aspx>), its
//! means: the chest 0.918 as deep as it is broad, the waist 0.709, the hips (buttock depth over
//! hip breadth) 0.658; the heights of the bust (chest height), waist (omphalion), knee and ankle
//! above the floor, the cervicale's above the waist and the crotch's below the buttocks; the thigh 0.603 of the hips round,
//! the knee (lower thigh) 0.392, the calf 0.366, the ankle 0.212, the base of the neck 0.394 of the
//! chest. All lengths are in mm, x across the body (to its left), y to its front, z up from the
//! floor.

use std::f64::consts::PI;

/// Points round each ring.
pub const RING_POINTS: usize = 32;

/// One horizontal cross-section of the body: an ellipse at height `z`, centred at `x` across.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    /// What it is: "neck", "bust", "waist", "hip", "crotch", "thigh", "knee", "calf", "ankle".
    pub name: &'static str,
    /// Height above the floor.
    pub z: f64,
    /// Its middle across the body (0 on the torso; a leg's to one side).
    pub x: f64,
    /// Round it.
    pub girth: f64,
    /// Its depth (front to back) over its breadth (side to side).
    pub depth_ratio: f64,
}

impl Ring {
    /// Half its breadth and half its depth: the ellipse `girth` round (Ramanujan's second
    /// approximation of an ellipse's perimeter, within a few parts in a million for a body's).
    pub fn semi_axes(&self) -> (f64, f64) {
        let r = self.depth_ratio;
        let h = ((1.0 - r) / (1.0 + r)).powi(2);
        let per_a = PI * (1.0 + r) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()));
        let a = self.girth / per_a;
        (a, a * r)
    }

    /// Its points, [`RING_POINTS`] of them, starting at the body's left side and going round
    /// through the front.
    pub fn points(&self) -> Vec<[f64; 3]> {
        let (a, b) = self.semi_axes();
        (0..RING_POINTS)
            .map(|i| {
                let t = i as f64 / RING_POINTS as f64 * 2.0 * PI;
                [self.x + a * t.cos(), b * t.sin(), self.z]
            })
            .collect()
    }
}

/// The measurements an avatar is made from, in mm. Those that are `None` come from the others by
/// ANSUR II's proportions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Body {
    pub bust: f64,
    pub waist: f64,
    pub hip: f64,
    pub nape_to_waist: f64,
    pub waist_to_hip: f64,
    pub neck: Option<f64>,
    pub crotch_depth: Option<f64>,
    pub waist_to_knee: Option<f64>,
    pub waist_to_ankle: Option<f64>,
}

/// The measurements an avatar needs: (name, what it is).
pub const MEASUREMENTS: &[(&str, &str)] = &[
    ("bust", "round the fullest part of the bust"),
    ("waist", "round the natural waist"),
    ("hip", "round the fullest part of the hips"),
    ("nape_to_waist", "down the spine from the nape to the waist"),
    ("waist_to_hip", "down the side from the waist to the fullest part of the hips"),
];

/// Why an avatar can't be made.
#[derive(Clone, Debug, PartialEq)]
pub enum AvatarError {
    /// A measurement that is missing, not more than 0, or out of a body's range.
    Measurement(&'static str),
    /// The heights don't go down the body in order (the hips below the crotch, the knee below the
    /// ankle…).
    Order(&'static str),
}

impl std::fmt::Display for AvatarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvatarError::Measurement(m) => write!(f, "the avatar needs {m} (mm, measure.set)"),
            AvatarError::Order(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for AvatarError {}

/// A triangle mesh: its points and its triangles (each three points' indices, wound so the
/// outside is anticlockwise seen from outside).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// An avatar: its rings (the torso's from the neck down, then each leg's from the thigh down) and
/// the mesh lofted through them.
#[derive(Clone, Debug, PartialEq)]
pub struct Avatar {
    pub torso: Vec<Ring>,
    pub legs: [Vec<Ring>; 2],
    pub mesh: Mesh,
    /// Its height to the base of the neck.
    pub neck_height: f64,
}

// ANSUR II female means (mm), the proportions in the module docs.
const WAIST_HEIGHT: f64 = 980.0;
const CHEST_HEIGHT: f64 = 1172.0;
const CERVICALE_HEIGHT: f64 = 1396.0;
const CROTCH_HEIGHT: f64 = 782.0;
const BUTTOCK_HEIGHT: f64 = 834.0;
const KNEE_HEIGHT: f64 = 449.0;
const ANKLE_HEIGHT: f64 = 63.0;

/// Make the avatar for `body`.
pub fn avatar(body: &Body) -> Result<Avatar, AvatarError> {
    let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
    for (name, v, lo, hi) in [
        ("bust", body.bust, 400.0, 2500.0),
        ("waist", body.waist, 300.0, 2500.0),
        ("hip", body.hip, 400.0, 2500.0),
        ("nape_to_waist", body.nape_to_waist, 200.0, 800.0),
        ("waist_to_hip", body.waist_to_hip, 50.0, 500.0),
    ] {
        if !ok(v, lo, hi) {
            return Err(AvatarError::Measurement(name));
        }
    }
    // The body's scale along its length: the nape to the waist against the survey's.
    let s = body.nape_to_waist / (CERVICALE_HEIGHT - WAIST_HEIGHT);
    let waist_z = match body.waist_to_ankle {
        Some(w) => w + ANKLE_HEIGHT * s,
        None => WAIST_HEIGHT * s,
    };
    // Without a crotch depth, the crotch the survey's way below the fullest part of the hips (its
    // waist is the navel's, lower than a dressmaker's natural waist, so not from the waist).
    let crotch_z = match body.crotch_depth {
        Some(c) => waist_z - c,
        None => waist_z - body.waist_to_hip - (BUTTOCK_HEIGHT - CROTCH_HEIGHT) * s,
    };
    let knee_z = match body.waist_to_knee {
        Some(k) => waist_z - k,
        None => KNEE_HEIGHT * (crotch_z / CROTCH_HEIGHT),
    };
    let ankle_z = ANKLE_HEIGHT * s;
    let hip_z = waist_z - body.waist_to_hip;
    let neck_z = waist_z + body.nape_to_waist;
    let bust_z = waist_z + (CHEST_HEIGHT - WAIST_HEIGHT) * s;
    if hip_z <= crotch_z + 10.0 {
        return Err(AvatarError::Order("the hips are at or below the crotch: waist_to_hip is less than crotch_depth"));
    }
    if crotch_z <= knee_z + 50.0 || knee_z <= ankle_z + 50.0 {
        return Err(AvatarError::Order("the crotch, knee and ankle don't go down the leg in order"));
    }
    let ring = |name, z, x, girth, depth_ratio| Ring { name, z, x, girth, depth_ratio };
    let neck = body.neck.unwrap_or(body.bust * 0.394);
    // The torso: the hips' girth carried on to the crotch, a little in, where the legs part.
    let torso = vec![
        ring("neck", neck_z, 0.0, neck, 1.0),
        ring("bust", bust_z, 0.0, body.bust, 0.918),
        ring("waist", waist_z, 0.0, body.waist, 0.709),
        ring("hip", hip_z, 0.0, body.hip, 0.658),
        ring("crotch", crotch_z, 0.0, body.hip * 0.96, 0.658),
    ];
    // Each leg: its thigh just below the crotch, its outside in line with the side of the hips
    // (the thighs meet at the crotch, as they do on many bodies).
    let (hip_a, _) = torso[3].semi_axes();
    let thigh = ring("thigh", 0.0, 0.0, body.hip * 0.603, 0.9).semi_axes().0;
    let leg = |side: f64| {
        let x = side * (hip_a - thigh).max(thigh * 0.5);
        let calf_z = knee_z - (knee_z - ankle_z) * 0.3;
        vec![
            ring("thigh", crotch_z - 10.0, x, body.hip * 0.603, 0.9),
            ring("knee", knee_z, x, body.hip * 0.392, 0.95),
            ring("calf", calf_z, x, body.hip * 0.366, 0.95),
            ring("ankle", ankle_z, x, body.hip * 0.212, 0.9),
        ]
    };
    let legs = [leg(1.0), leg(-1.0)];
    let mut mesh = Mesh::default();
    for rings in [&torso, &legs[0], &legs[1]] {
        loft(&mut mesh, rings);
    }
    Ok(Avatar { torso, legs, mesh, neck_height: neck_z })
}

/// A tube through `rings` (top to bottom) added to `mesh`, its ends closed with a fan.
fn loft(mesh: &mut Mesh, rings: &[Ring]) {
    let n = RING_POINTS as u32;
    let start = mesh.vertices.len() as u32;
    for r in rings {
        mesh.vertices.extend(r.points());
    }
    for k in 0..rings.len().saturating_sub(1) as u32 {
        let (up, down) = (start + k * n, start + (k + 1) * n);
        for i in 0..n {
            let j = (i + 1) % n;
            // Seen from outside the body, a ring runs anticlockwise (left side, front, right).
            mesh.triangles.push([up + i, down + i, down + j]);
            mesh.triangles.push([up + i, down + j, up + j]);
        }
    }
    // The ends: the top seen from above, the bottom from below.
    for (k, top) in [(0usize, true), (rings.len().saturating_sub(1), false)] {
        let Some(r) = rings.get(k) else { continue };
        let c = mesh.vertices.len() as u32;
        mesh.vertices.push([r.x, 0.0, r.z]);
        let base = start + k as u32 * n;
        for i in 0..n {
            let j = (i + 1) % n;
            mesh.triangles.push(if top { [c, base + i, base + j] } else { [c, base + j, base + i] });
        }
    }
}

/// The mesh as a Wavefront OBJ file, in metres (as glTF and most 3D software take them), y up
/// and z to the front, as OBJ files are usually read.
pub fn to_obj(mesh: &Mesh, name: &str) -> String {
    let mut out = String::with_capacity(mesh.vertices.len() * 40 + mesh.triangles.len() * 24);
    out.push_str("# PatternFoundry avatar, metres, y up\n");
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    out.push_str(&format!("o {}\n", if safe.is_empty() { "avatar".into() } else { safe }));
    for v in &mesh.vertices {
        out.push_str(&format!("v {:.5} {:.5} {:.5}\n", v[0] / 1000.0, v[2] / 1000.0, v[1] / 1000.0));
    }
    for t in &mesh.triangles {
        // OBJ counts from 1; swapping y and z mirrors the mesh, so each triangle is turned back.
        out.push_str(&format!("f {} {} {}\n", t[0] + 1, t[2] + 1, t[1] + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> Body {
        Body { bust: 880.0, waist: 680.0, hip: 960.0, nape_to_waist: 410.0, waist_to_hip: 200.0, ..Default::default() }
    }

    /// The perimeter of ring `r`'s polygon of many points: close to the ellipse's.
    fn round_it(r: &Ring) -> f64 {
        let (a, b) = r.semi_axes();
        let n = 20_000;
        (0..n)
            .map(|i| {
                let (t0, t1) = (i as f64 / n as f64 * 2.0 * PI, (i + 1) as f64 / n as f64 * 2.0 * PI);
                ((a * t1.cos() - a * t0.cos()).powi(2) + (b * t1.sin() - b * t0.sin()).powi(2)).sqrt()
            })
            .sum()
    }

    #[test]
    fn each_ring_is_as_long_round_as_its_girth_and_as_deep_as_the_body() {
        let a = avatar(&body()).unwrap();
        for r in a.torso.iter().chain(a.legs.iter().flatten()) {
            assert!((round_it(r) - r.girth).abs() / r.girth < 1e-5, "{}: {} against {}", r.name, round_it(r), r.girth);
            let (ax, by) = r.semi_axes();
            assert!((by / ax - r.depth_ratio).abs() < 1e-12);
        }
        let by = |n: &str| a.torso.iter().find(|r| r.name == n).copied().unwrap();
        assert_eq!((by("bust").girth, by("waist").girth, by("hip").girth), (880.0, 680.0, 960.0));
        // The heights: the nape 410 above the waist, the hips 200 below it, in order down the leg.
        assert!((by("neck").z - by("waist").z - 410.0).abs() < 1e-9);
        assert!((by("waist").z - by("hip").z - 200.0).abs() < 1e-9);
        let zs: Vec<f64> = a.torso.iter().chain(&a.legs[0]).map(|r| r.z).collect();
        assert!(zs.windows(2).all(|w| w[0] > w[1]), "{zs:?}");
        // The survey's woman: a nape to waist of 416 puts her waist at 980.
        let mean = avatar(&Body { nape_to_waist: 416.0, ..body() }).unwrap();
        assert!((mean.torso[2].z - 980.0).abs() < 1e-9);
    }

    #[test]
    fn given_lengths_are_used_and_bad_measurements_refused() {
        let a = avatar(&Body { waist_to_ankle: Some(900.0), waist_to_knee: Some(560.0), crotch_depth: Some(260.0), ..body() }).unwrap();
        let waist = a.torso[2].z;
        let ankle = a.legs[0][3].z;
        assert!((waist - ankle - 900.0).abs() < 1e-9 && (waist - a.legs[0][1].z - 560.0).abs() < 1e-9);
        assert!((waist - a.torso[4].z - 260.0).abs() < 1e-9);
        // The legs apart, either side of the middle.
        assert!(a.legs[0][0].x > 0.0 && a.legs[1][0].x < 0.0);
        assert_eq!(avatar(&Body { bust: 0.0, ..body() }), Err(AvatarError::Measurement("bust")));
        assert!(matches!(avatar(&Body { crotch_depth: Some(150.0), ..body() }), Err(AvatarError::Order(_))));
    }

    #[test]
    fn the_mesh_is_closed_and_its_triangles_face_out() {
        let a = avatar(&body()).unwrap();
        let m = &a.mesh;
        // Three tubes, each its rings' points and two end points.
        assert_eq!(m.vertices.len(), (5 + 4 + 4) * RING_POINTS + 6);
        // Closed: every edge is shared by exactly two triangles, once each way.
        let mut edges = std::collections::HashMap::new();
        for t in &m.triangles {
            for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edges.entry((p, q)).or_insert(0) += 1;
            }
        }
        assert!(edges.iter().all(|(&(p, q), &c)| c == 1 && edges.get(&(q, p)) == Some(&1)));
        // Facing out: the volume by the divergence theorem is positive.
        let v: f64 = m
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| m.vertices[i as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
            })
            .sum();
        assert!(v > 0.0, "{v}");
        let obj = to_obj(m, "My body");
        assert!(obj.contains("\no My_body\n"));
        assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), m.vertices.len());
        assert_eq!(obj.lines().filter(|l| l.starts_with("f ")).count(), m.triangles.len());
    }
}
