//! An avatar from body measurements (PF-10.1): the body the 3D fit preview drapes a pattern on.
//!
//! The body is lofted through horizontal cross-sections (rings) at the heights of the body's
//! landmarks: the base of the neck, the shoulders, the bust, the waist, the hips and the crotch on
//! the torso; the thigh, knee, calf and ankle on each leg; and the top of the arm, the upper arm,
//! the elbow and the wrist on each arm, the arms hanging a little out from the body (an A pose, as
//! fitting avatars stand, so a sleeve and the side of a bodice can hang between arm and body). Each ring is an ellipse as round as the body is
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
//! chest; the shoulders (acromion) 60.6 mm and the armpit (axilla) 156.7 below the cervicale, the
//! shoulders 365.3 broad (biacromial breadth), 0.386 of the chest round, and as deep at the shoulders
//! as their 1028.2 round over the deltoids at 450.3 across makes them (0.476 of their breadth); the
//! arm 543.6 from the shoulder to the wrist (sleeve outseam), the elbow 311.2 down it
//! (acromion–radiale); the upper arm (biceps) 0.323 of the chest round, the forearm 0.864 of the
//! upper arm and the wrist 0.506 (the means of the survey's women, computed from its public data).
//! A male figure ([`Figure::Male`], PF-10.11) takes the same proportions from the survey's 4,082
//! men instead (in [`MEN`]: a chest 0.877 as deep as broad, hips 0.711, broader shoulders, longer
//! arms, all heights taller), and has no bust standing forward: its chest (the `bust`
//! measurement, taken round the chest) is one elliptic ring.
//! All lengths are in mm, x across the body (to its left), y to its front, z up from the
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
    /// Its middle toward the front (0 but at the bust, which stands forward of a flat back).
    pub y: f64,
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
                [self.x + a * t.cos(), self.y + b * t.sin(), self.z]
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
    /// Along the shoulder from the base of the neck to the shoulder point (a bodice's shoulder).
    pub shoulder: Option<f64>,
    /// Round the upper arm.
    pub top_arm: Option<f64>,
    /// From the shoulder point down the outside of the arm to the wrist.
    pub sleeve_length: Option<f64>,
    /// From the nape down to the level of the armpits.
    pub armscye_depth: Option<f64>,
    /// Round the ribcage just under the bust (a bra's band).
    pub underbust: Option<f64>,
    /// Whose proportions fill in the rest.
    pub figure: Figure,
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

/// An avatar: its rings (the torso's from the neck down, each leg's from the thigh down and each
/// arm's from the shoulder down) and the mesh lofted through them.
#[derive(Clone, Debug, PartialEq)]
pub struct Avatar {
    pub torso: Vec<Ring>,
    pub legs: [Vec<Ring>; 2],
    /// The body's left arm (to +x) and its right.
    pub arms: [Vec<Ring>; 2],
    pub mesh: Mesh,
    /// Its height to the base of the neck.
    pub neck_height: f64,
}

/// Whose proportions the avatar takes where the measurements don't say: ANSUR II's women's or
/// men's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Figure {
    #[default]
    Female,
    Male,
}

impl Figure {
    /// Its name in `measure.set` ("female", "male").
    pub fn name(self) -> &'static str {
        match self {
            Figure::Female => "female",
            Figure::Male => "male",
        }
    }

    pub fn parse(s: &str) -> Option<Figure> {
        match s {
            "female" => Some(Figure::Female),
            "male" => Some(Figure::Male),
            _ => None,
        }
    }
}

/// A figure's ANSUR II means (mm) and proportions: those in the module docs for the women; the
/// men's (4,082 of them) computed the same way from the same public data.
struct Survey {
    waist_height: f64,
    chest_height: f64,
    cervicale_height: f64,
    crotch_height: f64,
    buttock_height: f64,
    knee_height: f64,
    ankle_height: f64,
    acromial_height: f64,
    axilla_height: f64,
    sleeve_outseam: f64,
    shoulder_to_elbow: f64,
    /// Depth over breadth: the chest, the waist, the hips (buttock depth over hip breadth), the
    /// shoulders (from their girth over the deltoids, over the half breadth at the shoulder points).
    chest_depth: f64,
    waist_depth: f64,
    hip_depth: f64,
    shoulder_depth: f64,
    /// Of the hips round: the thigh, the knee (lower thigh), the calf, the ankle.
    thigh: f64,
    knee: f64,
    calf: f64,
    ankle: f64,
    /// Of the chest round: the base of the neck, the shoulders' breadth, the upper arm (biceps).
    neck: f64,
    shoulders: f64,
    upper_arm: f64,
    /// Of the upper arm round: the forearm, the wrist.
    forearm: f64,
    wrist: f64,
    /// The bust stands forward of the ribcage (the women's).
    bust: bool,
}

const WOMEN: Survey = Survey {
    waist_height: 980.0,
    chest_height: 1172.0,
    cervicale_height: 1396.0,
    crotch_height: 782.0,
    buttock_height: 834.0,
    knee_height: 449.0,
    ankle_height: 63.0,
    acromial_height: 1335.1,
    axilla_height: 1239.0,
    sleeve_outseam: 543.6,
    shoulder_to_elbow: 311.2,
    chest_depth: 0.918,
    waist_depth: 0.709,
    hip_depth: 0.658,
    shoulder_depth: 0.476,
    thigh: 0.603,
    knee: 0.392,
    calf: 0.366,
    ankle: 0.212,
    neck: 0.394,
    shoulders: 0.386,
    upper_arm: 0.323,
    forearm: 0.864,
    wrist: 0.506,
    bust: true,
};

const MEN: Survey = Survey {
    waist_height: 1056.5,
    chest_height: 1291.1,
    cervicale_height: 1517.3,
    crotch_height: 845.9,
    buttock_height: 887.9,
    knee_height: 488.4,
    ankle_height: 72.9,
    acromial_height: 1440.7,
    axilla_height: 1329.1,
    sleeve_outseam: 593.1,
    shoulder_to_elbow: 335.2,
    chest_depth: 0.877,
    waist_depth: 0.728,
    hip_depth: 0.711,
    shoulder_depth: 0.503,
    thigh: 0.613,
    knee: 0.401,
    calf: 0.385,
    ankle: 0.225,
    neck: 0.410,
    shoulders: 0.393,
    upper_arm: 0.338,
    forearm: 0.866,
    wrist: 0.491,
    bust: false,
};

/// The height from the waist to the cervicale over the length down the back between them
/// (cervicale height less waist height, 415.5, over the waist back length, 425.4; the men's
/// 460.8 over 477.6, the same to within 0.3%).
pub const BACK_RISE: f64 = 415.5 / 425.4;
/// How far out from straight down the arms hang (an A pose).
/// A B cup's bust over underbust (EN 13402-3: 14 to 16 cm), when the set has no underbust.
const B_CUP: f64 = 150.0;
/// How far below the bust point the underbust is (a bra's band; chosen: the survey doesn't
/// measure it), and how deep the ribcage is there over its breadth (chosen, between the chest's and
/// the waist's).
const UNDERBUST_DROP: f64 = 75.0;
const UNDERBUST_DEPTH: f64 = 0.8;
const ARM_ANGLE: f64 = 30.0 * PI / 180.0;

impl Avatar {
    /// The torso's ring called `name` ("neck", "shoulder", "bust", "waist", "hip", "crotch").
    pub fn ring(&self, name: &str) -> Option<&Ring> {
        self.torso.iter().find(|r| r.name == name)
    }

    /// Each chain of rings the body is lofted through: the torso, the legs, the arms.
    pub fn chains(&self) -> [&Vec<Ring>; 5] {
        [&self.torso, &self.legs[0], &self.legs[1], &self.arms[0], &self.arms[1]]
    }
}

/// The girth of an ellipse with semi-axes `a` and `b` (Ramanujan's second approximation).
fn ellipse_girth(a: f64, b: f64) -> f64 {
    let h = ((a - b) / (a + b)).powi(2);
    PI * (a + b) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()))
}

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
    // The nape to the waist is measured down the curve of the back: its height is a little less.
    let sv = match body.figure {
        Figure::Female => &WOMEN,
        Figure::Male => &MEN,
    };
    let rise = body.nape_to_waist * BACK_RISE;
    let s = rise / (sv.cervicale_height - sv.waist_height);
    let waist_z = match body.waist_to_ankle {
        Some(w) => w + sv.ankle_height * s,
        None => sv.waist_height * s,
    };
    // Without a crotch depth, the crotch the survey's way below the fullest part of the hips (its
    // waist is the navel's, lower than a dressmaker's natural waist, so not from the waist).
    let crotch_z = match body.crotch_depth {
        Some(c) => waist_z - c,
        None => waist_z - body.waist_to_hip - (sv.buttock_height - sv.crotch_height) * s,
    };
    let knee_z = match body.waist_to_knee {
        Some(k) => waist_z - k,
        None => sv.knee_height * (crotch_z / sv.crotch_height),
    };
    let ankle_z = sv.ankle_height * s;
    let hip_z = waist_z - body.waist_to_hip;
    let neck_z = waist_z + rise;
    let bust_z = waist_z + (sv.chest_height - sv.waist_height) * s;
    if hip_z <= crotch_z + 10.0 {
        return Err(AvatarError::Order("the hips are at or below the crotch: waist_to_hip is less than crotch_depth"));
    }
    if crotch_z <= knee_z + 50.0 || knee_z <= ankle_z + 50.0 {
        return Err(AvatarError::Order("the crotch, knee and ankle don't go down the leg in order"));
    }
    let ring = |name, z, x, girth, depth_ratio| Ring { name, z, x, y: 0.0, girth, depth_ratio };
    let neck = body.neck.unwrap_or(body.bust * sv.neck);
    // The shoulders: as broad as two shoulder lengths and the neck across, else the survey's share
    // of the chest; as deep as the survey's.
    let shoulder_z = neck_z - (sv.cervicale_height - sv.acromial_height) * s;
    // The armpits as deep below the nape as measured (a bodice's underarm is drafted there),
    // else the survey's.
    let axilla_z = match body.armscye_depth {
        Some(d) if d > 0.0 => neck_z - d * BACK_RISE,
        _ => neck_z - (sv.cervicale_height - sv.axilla_height) * s,
    };
    let breadth = match body.shoulder {
        Some(l) if l > 0.0 => 2.0 * l + neck / PI,
        _ => body.bust * sv.shoulders,
    };
    let shoulder_ring = ring("shoulder", shoulder_z, 0.0, ellipse_girth(breadth / 2.0, breadth / 2.0 * sv.shoulder_depth), sv.shoulder_depth);
    // The bust: the ribcage under it as round as the underbust (else a B cup's, EN 13402-3: the
    // bust 14 to 16 cm more round), the back flat from the underbust up to the bust, the bust's
    // girth more than the ribcage's standing forward.
    // A man's chest (the `bust` measurement, round the chest) has none: one elliptic ring.
    let underbust = body.underbust.filter(|u| *u > 0.0 && *u < body.bust).unwrap_or((body.bust - B_CUP).max(body.bust * 0.75));
    let under = ring("underbust", bust_z - UNDERBUST_DROP * s, 0.0, underbust, UNDERBUST_DEPTH);
    let mut bust_ring = ring("bust", bust_z, 0.0, body.bust, sv.chest_depth);
    if sv.bust {
        bust_ring.y = bust_ring.semi_axes().1 - under.semi_axes().1;
    }
    // The torso: the hips' girth carried on to the crotch, a little in, where the legs part.
    let mut torso = vec![ring("neck", neck_z, 0.0, neck, 1.0), shoulder_ring, bust_ring];
    if sv.bust {
        torso.push(under);
    }
    let hip_ring = ring("hip", hip_z, 0.0, body.hip, sv.hip_depth);
    torso.extend([ring("waist", waist_z, 0.0, body.waist, sv.waist_depth), hip_ring, ring("crotch", crotch_z, 0.0, body.hip * 0.96, sv.hip_depth)]);
    // Each leg: its thigh just below the crotch, its outside in line with the side of the hips
    // (the thighs meet at the crotch, as they do on many bodies).
    let (hip_a, _) = hip_ring.semi_axes();
    let thigh = ring("thigh", 0.0, 0.0, body.hip * sv.thigh, 0.9).semi_axes().0;
    let leg = |side: f64| {
        let x = side * (hip_a - thigh).max(thigh * 0.5);
        let calf_z = knee_z - (knee_z - ankle_z) * 0.3;
        vec![
            ring("thigh", crotch_z - 10.0, x, body.hip * sv.thigh, 0.9),
            ring("knee", knee_z, x, body.hip * sv.knee, 0.95),
            ring("calf", calf_z, x, body.hip * sv.calf, 0.95),
            ring("ankle", ankle_z, x, body.hip * sv.ankle, 0.9),
        ]
    };
    let legs = [leg(1.0), leg(-1.0)];
    let upper = body.top_arm.filter(|g| *g > 0.0).unwrap_or(body.bust * sv.upper_arm);
    let reach = body.sleeve_length.filter(|l| *l > 0.0).map_or(s, |l| l / sv.sleeve_outseam);
    // Each arm swings out from its shoulder joint (the head of the upper arm, under the shoulder
    // point), as an arm does: the top of the arm is the deltoid, round over the joint and on out
    // past the shoulder point (the survey's shoulders are 450.3 across the deltoids for 365.3
    // across the shoulder points); below it the arm hangs out at the A pose's angle, the armpit
    // opening under it. Seen across, a tilted arm's level cross-section is longer by 1/cos.
    let deltoid = upper * 1.12;
    let r_top = deltoid / (2.0 * PI);
    let angle = ARM_ANGLE;
    // The joint sits in from the shoulder point by a fifth of the deltoid's radius (further in,
    // or the arm at 20°, and the sleeves drag the bodice down the arms: PF-10.9's debug log).
    let joint = (breadth / 2.0 - r_top * 0.2, shoulder_z - r_top);
    // A point down the arm's line from the joint, `l` along it.
    let along = |l: f64| (joint.0 + l * angle.sin(), joint.1 - l * angle.cos());
    let at_z = |z: f64| joint.0 + (joint.1 - z) * angle.tan();
    let top_z = shoulder_z - r_top * 0.25;
    let elbow = along(sv.shoulder_to_elbow * reach - r_top);
    let wrist = along(sv.sleeve_outseam * reach - r_top);
    let tilted = |name, z: f64, x: f64, girth: f64, depth: f64| {
        let r = girth / (2.0 * PI);
        let (a, b) = (r / angle.cos(), r * depth);
        ring(name, z, x, ellipse_girth(a, b), b / a)
    };
    let arm = |side: f64| {
        // Over the deltoid a rounded top, so a sleeve's cap rests on it (the body is only where
        // its rings are: without it the arm's top was open to the cloth, and the cap slid off it).
        let cap_z = top_z + r_top * 0.55;
        vec![
            ring("armcap", cap_z, side * (joint.0 - (cap_z - joint.1) * angle.tan()), deltoid * 0.55, 0.9),
            ring("armtop", top_z, side * (joint.0 - (top_z - joint.1) * angle.tan()), deltoid, 0.9),
            tilted("upperarm", axilla_z, side * at_z(axilla_z), upper, 0.95),
            tilted("elbow", elbow.1, side * elbow.0, upper * sv.forearm, 0.95),
            tilted("wrist", wrist.1, side * wrist.0, upper * sv.wrist, 0.75),
        ]
    };
    let (elbow_z, wrist_z) = (elbow.1, wrist.1);
    let arms = [arm(1.0), arm(-1.0)];
    if !(top_z > axilla_z && axilla_z > elbow_z + 50.0 && elbow_z > wrist_z + 50.0) {
        return Err(AvatarError::Order("the shoulder, armpit, elbow and wrist don't go down the arm in order"));
    }
    let mut mesh = Mesh::default();
    for rings in [&torso, &legs[0], &legs[1], &arms[0], &arms[1]] {
        loft(&mut mesh, rings);
    }
    Ok(Avatar { torso, legs, arms, mesh, neck_height: neck_z })
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
        for r in a.torso.iter().chain(a.legs.iter().flatten()).chain(a.arms.iter().flatten()) {
            assert!((round_it(r) - r.girth).abs() / r.girth < 1e-5, "{}: {} against {}", r.name, round_it(r), r.girth);
            let (ax, by) = r.semi_axes();
            assert!((by / ax - r.depth_ratio).abs() < 1e-12);
        }
        let by = |n: &str| a.torso.iter().find(|r| r.name == n).copied().unwrap();
        assert_eq!((by("bust").girth, by("waist").girth, by("hip").girth), (880.0, 680.0, 960.0));
        // The heights: the nape 410 down the back above the waist (a little less straight up), the
        // hips 200 below it, in order down the leg.
        assert!((by("neck").z - by("waist").z - 410.0 * BACK_RISE).abs() < 1e-9);
        assert!((by("waist").z - by("hip").z - 200.0).abs() < 1e-9);
        let zs: Vec<f64> = a.torso.iter().chain(&a.legs[0]).map(|r| r.z).collect();
        assert!(zs.windows(2).all(|w| w[0] > w[1]), "{zs:?}");
        // The survey's woman: her cervicale 416 above her waist puts her waist at 980.
        let mean = avatar(&Body { nape_to_waist: 416.0 / BACK_RISE, ..body() }).unwrap();
        assert!((mean.ring("waist").unwrap().z - 980.0).abs() < 1e-9);
    }

    #[test]
    fn given_lengths_are_used_and_bad_measurements_refused() {
        let a = avatar(&Body { waist_to_ankle: Some(900.0), waist_to_knee: Some(560.0), crotch_depth: Some(260.0), ..body() }).unwrap();
        let waist = a.ring("waist").unwrap().z;
        let ankle = a.legs[0][3].z;
        assert!((waist - ankle - 900.0).abs() < 1e-9 && (waist - a.legs[0][1].z - 560.0).abs() < 1e-9);
        assert!((waist - a.ring("crotch").unwrap().z - 260.0).abs() < 1e-9);
        // The legs apart, either side of the middle.
        assert!(a.legs[0][0].x > 0.0 && a.legs[1][0].x < 0.0);
        assert_eq!(avatar(&Body { bust: 0.0, ..body() }), Err(AvatarError::Measurement("bust")));
        assert!(matches!(avatar(&Body { crotch_depth: Some(150.0), ..body() }), Err(AvatarError::Order(_))));
    }

    /// The shoulders as broad as two shoulder lengths and the neck, below the base of the neck by
    /// the survey's 60.6 mm; the arms down to the wrist as long as the sleeve length, hanging out
    /// clear of the chest and the hips, the upper arm as round as measured.
    #[test]
    fn the_shoulders_and_arms_follow_the_measurements() {
        let b = Body { shoulder: Some(121.0), neck: Some(370.0), top_arm: Some(280.0), sleeve_length: Some(590.0), ..body() };
        let a = avatar(&b).unwrap();
        let sh = *a.ring("shoulder").unwrap();
        assert!((sh.semi_axes().0 * 2.0 - (242.0 + 370.0 / PI)).abs() < 0.01, "{:?}", sh.semi_axes());
        let s = 410.0 * BACK_RISE / (WOMEN.cervicale_height - WOMEN.waist_height);
        assert!((a.neck_height - sh.z - (WOMEN.cervicale_height - WOMEN.acromial_height) * s).abs() < 1e-9);
        for (k, side) in [(0, 1.0), (1, -1.0)] {
            let arm = &a.arms[k];
            assert_eq!(arm.iter().map(|r| r.name).collect::<Vec<_>>(), ["armcap", "armtop", "upperarm", "elbow", "wrist"]);
            // The upper arm round as measured, square to the arm (level across the tilted arm it
            // is longer by 1/cos).
            let (u, d) = arm[2].semi_axes();
            let square = ellipse_girth(u * ARM_ANGLE.cos(), d);
            assert!((square - 280.0).abs() < 280.0 * 0.03, "{square}");
            assert!(arm[2].girth > square);
            // Down the outside of the arm, shoulder to wrist: the sleeve length.
            let along = ((sh.z - arm[4].z).powi(2) + (arm[4].x - arm[2].x).powi(2)).sqrt();
            assert!((along - 590.0).abs() < 25.0, "{along}");
            // Out of the side of the chest at the armpit, the top of the arm round over the
            // shoulder point; at the wrist clear of the hips.
            let bust = a.ring("bust").unwrap().semi_axes().0;
            assert!(side * arm[2].x > bust && side * arm[2].x - u < bust, "{:?} {bust}", arm[2]);
            assert!(side * arm[1].x + arm[1].semi_axes().0 > sh.semi_axes().0, "{:?}", arm[1]);
            assert!(arm[0].z > arm[1].z && arm[0].girth < arm[1].girth);
            let hip = a.ring("hip").unwrap().semi_axes().0;
            assert!(side * arm[4].x - arm[4].semi_axes().0 > hip, "{:?}", arm[4]);
            assert!(arm.windows(2).all(|w| w[0].z > w[1].z) && side * arm[4].x > side * arm[1].x);
        }
        // Without them, from the survey's proportions.
        let plain = avatar(&body()).unwrap();
        assert!((plain.ring("shoulder").unwrap().semi_axes().0 * 2.0 - 880.0 * 0.386).abs() < 1e-9);
        let (u, d) = plain.arms[0][2].semi_axes();
        assert!((ellipse_girth(u * ARM_ANGLE.cos(), d) - 880.0 * 0.323).abs() < 880.0 * 0.323 * 0.03);
    }

    /// A male figure (PF-10.11) takes the survey's men's proportions: no underbust and no bust
    /// standing forward, the chest as deep as the men's, the shoulders and arms from theirs.
    #[test]
    fn a_male_figure_takes_the_mens_proportions() {
        let b = Body { bust: 1000.0, waist: 860.0, hip: 980.0, nape_to_waist: 460.0, waist_to_hip: 200.0, ..body() };
        let man = avatar(&Body { figure: Figure::Male, ..b }).unwrap();
        let woman = avatar(&b).unwrap();
        assert!(man.ring("underbust").is_none() && woman.ring("underbust").is_some());
        let chest = *man.ring("bust").unwrap();
        assert_eq!((chest.y, chest.girth, chest.depth_ratio), (0.0, 1000.0, MEN.chest_depth));
        // Every ring still as round as measured.
        assert_eq!(man.ring("waist").unwrap().girth, 860.0);
        assert_eq!(man.ring("hip").unwrap().depth_ratio, MEN.hip_depth);
        // Broader shoulders and a thicker upper arm for the same chest, from the men's shares.
        let breadth = |a: &Avatar| a.ring("shoulder").unwrap().semi_axes().0;
        assert!((breadth(&man) * 2.0 - 1000.0 * MEN.shoulders).abs() < 1e-9);
        assert!(breadth(&man) > breadth(&woman));
        // The same mesh, less the underbust's ring.
        assert_eq!(man.mesh.vertices.len(), woman.mesh.vertices.len() - RING_POINTS);
        assert_eq!(Figure::parse("male"), Some(Figure::Male));
        assert_eq!(Figure::parse(Figure::Female.name()), Some(Figure::Female));
        assert_eq!(Figure::parse("other"), None);
    }

    /// The bust stands forward of a flat back: under it the ribcage as round as the underbust
    /// (else a B cup's 150 mm less than the bust), the bust's back in line with the ribcage's, its
    /// extra girth to the front.
    #[test]
    fn the_bust_stands_forward_of_a_flat_back() {
        for (under, expect) in [(None, 880.0 - 150.0), (Some(700.0), 700.0)] {
            let a = avatar(&Body { underbust: under, ..body() }).unwrap();
            let (bust, rib) = (*a.ring("bust").unwrap(), *a.ring("underbust").unwrap());
            assert_eq!(rib.girth, expect);
            assert!(rib.z < bust.z && rib.z > a.ring("waist").unwrap().z);
            let back = |r: &Ring| r.y - r.semi_axes().1;
            let front = |r: &Ring| r.y + r.semi_axes().1;
            assert!((back(&bust) - back(&rib)).abs() < 1e-9, "the back flat");
            assert!(front(&bust) > front(&rib) + 20.0, "the bust forward: {} {}", front(&bust), front(&rib));
            // The mesh's bust ring too.
            assert!(a.mesh.vertices.iter().any(|v| (v[2] - bust.z).abs() < 1e-9 && (v[1] - front(&bust)).abs() < 1e-6));
        }
    }

    #[test]
    fn the_mesh_is_closed_and_its_triangles_face_out() {
        let a = avatar(&body()).unwrap();
        let m = &a.mesh;
        // Five tubes, each its rings' points and two end points.
        assert_eq!(m.vertices.len(), (7 + 4 + 4 + 5 + 5) * RING_POINTS + 10);
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
