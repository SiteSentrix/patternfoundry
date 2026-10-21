//! Blocks (PF-4): basic pattern shapes drafted from the measurements, the starting point every
//! design is made from.
//!
//! The close-fitting bodice block is drafted in the British metric method (the family of
//! Winifred Aldrich's *Metric Pattern Cutting for Women's Wear*), from the steps published at
//! <https://www.fashioncapital.co.uk/tools/how-to-construct-a-basic-bodice-block/>: the
//! construction rules (which point is how far from which, from which measurement) are
//! implemented here; no text or drawing is copied. Where that page leaves a value open it is
//! chosen here and said so in the code. To be checked against the book.
//!
//! The basic straight skirt block follows the metric steps published at
//! <https://www.theshapesoffabric.com/2019/02/23/how-to-draft-the-basic-skirt-pattern/> (half hip
//! plus 1 cm ease; two thirds of the waist taken in at the side seams, the rest in darts, more at
//! the back), with the values that page leaves to its drawings chosen here and marked.
//!
//! The one-piece straight sleeve block follows Natalie Bray's method as published at
//! <https://www.thecreativecurator.com/drafting-a-sleeve-block/> (the owner's choice, 2026-10-20:
//! no public description of Aldrich's sleeve was found): the crown depth from the top arm by the
//! page's table, 5 cm ease round the top arm, the crown through marks on the back and front lines
//! and the page's midpoint offsets, straight sides, the wrist curved down at the back and up at
//! the front. The page says to check the crown against the bodice's armhole and adjust: with a
//! `sleeve_crown` measurement (the armhole plus the ease to set the sleeve in) the crown depth is
//! solved so the crown is that long.
//!
//! Every length is in mm, y down, from the draft's origin `O` (the back neck's top-left
//! construction corner). The back is drawn on the left (centre back at x = 0), the front on the
//! right (centre front at the right edge), both half-bodies. The sleeve's origin is its shoulder
//! point, the back half on the left.

use serde::{Deserialize, Serialize};

use crate::Pt;
use crate::expr::Env;

/// Which block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    /// The close-fitting bodice block: back and front.
    Bodice,
    /// The basic straight skirt block: back and front.
    Skirt,
    /// The one-piece straight sleeve block.
    Sleeve,
}

/// What a drafted path is: which part of which block, and where the block's origin is in the
/// document (points), so the path can be drawn again when the measurements change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockRef {
    pub kind: BlockKind,
    /// The part (for the bodice: 0 back, 1 front).
    pub part: usize,
    pub origin: Pt,
}

impl BlockRef {
    /// Move the origin by `f`, as the host transforms the path.
    pub fn transform(&mut self, f: impl Fn(Pt) -> Pt) {
        self.origin = f(self.origin);
    }
}

/// One segment of an outline: a straight line or a cubic Bézier to its end point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line(Pt),
    Cubic(Pt, Pt, Pt),
}

/// A drafted part: its name and its closed outline (from `start`, through `segs`, back to
/// `start`).
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub name: &'static str,
    pub start: Pt,
    pub segs: Vec<Seg>,
}

/// The measurements the bodice block is drafted from (all in mm).
pub const BODICE_MEASUREMENTS: &[(&str, &str)] = &[
    ("bust", "round the fullest part of the bust"),
    ("waist", "round the natural waist"),
    ("nape_to_waist", "down the spine from the nape to the waist"),
    ("back_width", "across the back between the armpits' creases"),
    ("chest", "across the front between the armpits' creases, above the bust"),
    ("shoulder", "from the side of the neck to the shoulder point"),
    ("neck", "round the base of the neck"),
    ("armscye_depth", "from the nape down to the level of the armpits"),
];

/// The measurements the skirt block is drafted from (all in mm).
pub const SKIRT_MEASUREMENTS: &[(&str, &str)] = &[
    ("waist", "round the natural waist"),
    ("hip", "round the fullest part of the hips"),
    ("waist_to_hip", "down the side from the waist to the fullest part of the hips"),
    ("skirt_length", "down the side from the waist to the hem (the knee for a block)"),
];

/// The measurements the sleeve block is drafted from (all in mm).
pub const SLEEVE_MEASUREMENTS: &[(&str, &str)] = &[
    ("top_arm", "round the fullest part of the upper arm"),
    ("sleeve_length", "from the shoulder point down the outside of the arm to the wrist, the arm a little bent"),
];

/// The measurements a block is drafted from.
pub fn measurements(kind: BlockKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        BlockKind::Bodice => BODICE_MEASUREMENTS,
        BlockKind::Skirt => SKIRT_MEASUREMENTS,
        BlockKind::Sleeve => SLEEVE_MEASUREMENTS,
    }
}

/// Why a block can't be drafted.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockError {
    /// Measurements the set doesn't have (or that don't work out).
    Missing(Vec<String>),
    /// Measurements that don't make a bodice (a shoulder too short to reach the armhole line…).
    Impossible(&'static str),
}

impl std::fmt::Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockError::Missing(m) => write!(f, "the block needs these measurements: {}", m.join(", ")),
            BlockError::Impossible(why) => write!(f, "these measurements don't make a bodice: {why}"),
        }
    }
}

impl std::error::Error for BlockError {}

fn need(env: &mut Env, names: &[(&str, &str)]) -> Result<Vec<f64>, BlockError> {
    let mut out = Vec::with_capacity(names.len());
    let mut missing = Vec::new();
    for (n, _) in names {
        match env.value(n) {
            Ok(v) if v > 0.0 => out.push(v),
            _ => missing.push((*n).to_string()),
        }
    }
    if missing.is_empty() { Ok(out) } else { Err(BlockError::Missing(missing)) }
}

fn mid(a: Pt, b: Pt) -> Pt {
    (a + b) * 0.5
}

/// The point on the horizontal line `y` at distance `len` from `from`, toward `dir` (+1 right,
/// -1 left).
fn reach(from: Pt, y: f64, len: f64, dir: f64) -> Option<Pt> {
    let dy = y - from.y;
    let d2 = len * len - dy * dy;
    (d2 >= 0.0).then(|| Pt::new(from.x + dir * d2.sqrt(), y))
}

/// The back and front of the close-fitting bodice block (parts 0 and 1).
pub fn bodice(env: &mut Env) -> Result<Vec<Part>, BlockError> {
    let v = need(env, BODICE_MEASUREMENTS)?;
    let &[bust, waist, nape_to_waist, back_width, chest, shoulder, neck, depth] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    let half_bust = bust / 2.0;
    // The construction frame.
    let o = Pt::new(0.0, 0.0);
    let a = Pt::new(0.0, 15.0);
    let b = Pt::new(0.0, a.y + depth + 5.0);
    let c = Pt::new(half_bust + 50.0, b.y);
    // The front neck rises above the back's top line for larger busts: an eighth of the bust over
    // 92 cm.
    let d = Pt::new(c.x, o.y - ((bust - 920.0) / 8.0).max(0.0));
    let e = Pt::new(0.0, nape_to_waist);
    if e.y <= b.y {
        return Err(BlockError::Impossible("the nape to waist length is shorter than the armscye depth"));
    }
    // Back neck and shoulder.
    let g = Pt::new(neck / 5.0 - 2.0, o.y);
    let h = Pt::new(0.0, a.y + depth / 5.0 - 7.0);
    let i = reach(g, h.y, shoulder + 10.0, 1.0).ok_or(BlockError::Impossible("the shoulder is too short to reach the armhole line"))?;
    // The back shoulder dart: 1 cm wide at the shoulder, its point 5 cm below and 1 cm toward the
    // centre back from the shoulder's middle.
    let j = mid(g, i);
    let k = Pt::new(j.x - 10.0, j.y + 50.0);
    let along = (i - g).unit().unwrap_or(Pt::new(1.0, 0.0));
    let (dart_in, dart_out) = (j - along * 5.0, j + along * 5.0);
    // Back width line and armhole points.
    let l = Pt::new(back_width / 2.0 + 5.0, b.y);
    let m = Pt::new(l.x, h.y);
    // The back armhole touches the back width line half way between the shoulder line and L.
    let n = mid(l, m);
    let p = mid(b, l);
    // Front neck.
    let r = Pt::new(d.x - (neck / 5.0 + 7.0), d.y);
    let s = Pt::new(d.x, d.y + neck / 5.0 - 2.0);
    // The bust dart: 7 cm at an 88 cm bust, 0.6 cm more for every 4 cm more bust.
    let dart = 70.0 + 6.0 * (bust - 880.0) / 40.0;
    if dart <= 0.0 {
        return Err(BlockError::Impossible("the bust is too small for the bust dart"));
    }
    // The source works the chest out as the bust less a back measurement taken round the back;
    // with the across-back width used here, the chest is measured across the front instead (as
    // the book's own measurement list has it).
    let t = Pt::new(c.x - (chest / 2.0 + dart / 2.0), b.y);
    if t.x <= l.x {
        return Err(BlockError::Impossible("the back width and the chest overlap at the side"));
    }
    let u = mid(c, t);
    let bust_point = Pt::new(u.x, u.y + 25.0);
    let w = Pt::new(r.x - dart, r.y);
    let x = reach(w, h.y + 15.0, shoulder, -1.0).ok_or(BlockError::Impossible("the front shoulder is too short to reach the armhole line"))?;
    let y = Pt::new(t.x, b.y - depth / 3.0);
    let z = mid(l, t);
    // How far the armhole curve sits inside the corners at L and T (by bust size).
    let (l_in, t_in) = match bust {
        v if v <= 820.0 => (22.5, 17.5),
        v if v <= 940.0 => (25.0, 20.0),
        v if v <= 1070.0 => (30.0, 25.0),
        _ => (35.0, 30.0),
    };
    let diag = std::f64::consts::FRAC_1_SQRT_2;
    let l_curve = Pt::new(l.x + l_in * diag, l.y - l_in * diag);
    let t_curve = Pt::new(t.x - t_in * diag, t.y - t_in * diag);
    // The front waist drops at the centre front for the bust's fullness. The source gives 0.5,
    // 1 and 1.5 cm for small, middling and large busts without saying where they change: here
    // up to 88 cm, below 100 cm, and above.
    let drop = if bust <= 880.0 {
        5.0
    } else if bust < 1000.0 {
        10.0
    } else {
        15.0
    };
    // Waist darts: what the waist takes in, half a body, shared back 2.5 : side 3 : front 3.5 (the
    // source's widths, scaled to what there is to take in).
    let take = (half_bust + 50.0) - (waist / 2.0 + 30.0);
    let scale = (take / 90.0).max(0.0);
    let (back_dart, side_dart, front_dart) = (25.0 * scale, 30.0 * scale, 35.0 * scale);
    let q = Pt::new(p.x, e.y);
    let aa = Pt::new(z.x, e.y);
    let v_pt = Pt::new(u.x, e.y);
    let f_waist = Pt::new(c.x, e.y + drop);
    // The front waist line, from the side to the centre front, sloping down by `drop`.
    let front_waist_y = |px: f64| e.y + drop * ((px - aa.x) / (c.x - aa.x)).clamp(0.0, 1.0);

    // Back: up the centre back, the neck, the shoulder with its dart, the armhole, down the side
    // seam, the waist with its dart.
    let back = Part {
        name: "Back Bodice Block",
        start: e,
        segs: vec![
            Seg::Line(a),
            // A shallow curve from the centre back to the neck point.
            Seg::Cubic(Pt::new(a.x + g.x * 0.55, a.y), Pt::new(g.x - 1.0, g.y + (a.y - g.y) * 0.35), g),
            Seg::Line(dart_in),
            Seg::Line(k),
            Seg::Line(dart_out),
            Seg::Line(i),
            // The back armhole: from the shoulder point down to N on the back width line, and in
            // round the corner at L.
            Seg::Cubic(Pt::new(i.x - (i.x - n.x) * 0.3, i.y + (n.y - i.y) * 0.45), Pt::new(n.x, n.y - (n.y - i.y) * 0.35), n),
            Seg::Cubic(Pt::new(n.x, n.y + (l_curve.y - n.y) * 0.6), Pt::new(l_curve.x - 8.0, l_curve.y - 8.0), l_curve),
            Seg::Cubic(Pt::new(l_curve.x + 8.0, l_curve.y + 8.0), Pt::new(z.x - (z.x - l.x) * 0.5, z.y), z),
            Seg::Line(Pt::new(aa.x - side_dart / 2.0, e.y)),
            Seg::Line(Pt::new(q.x + back_dart / 2.0, e.y)),
            Seg::Line(p),
            Seg::Line(Pt::new(q.x - back_dart / 2.0, e.y)),
        ],
    };
    // Front: down the centre front, the waist with its dart, up the side seam, the armhole, the
    // shoulder, the bust dart, the neck.
    let fd = |dx: f64| {
        let px = v_pt.x + dx;
        Pt::new(px, front_waist_y(px))
    };
    let front = Part {
        name: "Front Bodice Block",
        start: s,
        segs: vec![
            Seg::Line(f_waist),
            Seg::Line(fd(front_dart / 2.0)),
            Seg::Line(bust_point),
            Seg::Line(fd(-front_dart / 2.0)),
            Seg::Line(Pt::new(aa.x + side_dart / 2.0, e.y)),
            Seg::Line(z),
            // The front armhole: round the corner at T, up past Y to the shoulder point.
            Seg::Cubic(Pt::new(z.x + (t.x - z.x) * 0.5, z.y), Pt::new(t_curve.x - 7.0, t_curve.y + 7.0), t_curve),
            Seg::Cubic(Pt::new(t_curve.x + 7.0, t_curve.y - 7.0), Pt::new(y.x, y.y + 10.0), y),
            Seg::Cubic(Pt::new(y.x, y.y - (y.y - x.y) * 0.4), Pt::new(x.x + 3.0, x.y + (y.y - x.y) * 0.4), x),
            Seg::Line(w),
            Seg::Line(bust_point),
            Seg::Line(r),
            // A deep curve from the neck point down to the centre front.
            Seg::Cubic(Pt::new(r.x, r.y + (s.y - r.y) * 0.55), Pt::new(s.x - (s.x - r.x) * 0.55, s.y), s),
        ],
    };
    Ok(vec![back, front])
}

/// The back and front of the basic straight skirt block (parts 0 and 1): centre back at x = 0,
/// centre front at half the hip plus ease, the waist at y = 0.
pub fn skirt(env: &mut Env) -> Result<Vec<Part>, BlockError> {
    let v = need(env, SKIRT_MEASUREMENTS)?;
    let &[waist, hip, waist_to_hip, length] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    if waist_to_hip >= length {
        return Err(BlockError::Impossible("the skirt is shorter than the waist to hip depth"));
    }
    let width = hip / 2.0 + 10.0;
    let side = width / 2.0;
    let take = width - waist / 2.0;
    if take < 0.0 {
        return Err(BlockError::Impossible("the waist is wider than the hips with ease"));
    }
    // Two thirds at the side seams (half each side), the rest in one dart each side: the source
    // gives the back the larger dart; here three fifths of it.
    let at_side = take * 2.0 / 3.0 / 2.0;
    let darts = take - 2.0 * at_side;
    let (back_dart, front_dart) = (darts * 0.6, darts * 0.4);
    // Dart lengths are left to the drawings: here 14 cm at the back and 10 cm at the front, and
    // always 3 cm short of the hip line.
    let (back_len, front_len) = (140.0_f64.min(waist_to_hip - 30.0), 100.0_f64.min(waist_to_hip - 30.0));
    if back_len <= 0.0 {
        return Err(BlockError::Impossible("the waist to hip depth is too short for darts"));
    }
    // The side seam rises 1.5 cm above the waist line; the hip curve reaches out to the hip line.
    let rise = 15.0;
    let hip_y = waist_to_hip;
    let curve = |from: Pt, to: Pt| Seg::Cubic(Pt::new(from.x, from.y + (to.y - from.y) * 0.5), Pt::new(to.x, to.y - (to.y - from.y) * 0.45), to);
    let dart = |centre: f64, width: f64, len: f64, at_y: f64| {
        [Pt::new(centre - width / 2.0, at_y), Pt::new(centre, at_y + len), Pt::new(centre + width / 2.0, at_y)]
    };
    // Back: from centre back at the waist, across the waist with its dart, down the side seam,
    // the hem, up the centre back.
    let back_side_top = Pt::new(side - at_side, -rise);
    let bd = dart(side / 2.0, back_dart, back_len, -rise * 0.5);
    let back = Part {
        name: "Back Skirt Block",
        start: Pt::new(0.0, 0.0),
        segs: vec![
            Seg::Line(bd[0]),
            Seg::Line(bd[1]),
            Seg::Line(bd[2]),
            Seg::Line(back_side_top),
            curve(back_side_top, Pt::new(side, hip_y)),
            Seg::Line(Pt::new(side, length)),
            Seg::Line(Pt::new(0.0, length)),
        ],
    };
    let front_side_top = Pt::new(side + at_side, -rise);
    let fd = dart(side + side / 2.0, front_dart, front_len, -rise * 0.5);
    let front = Part {
        name: "Front Skirt Block",
        start: Pt::new(width, 0.0),
        segs: vec![
            Seg::Line(Pt::new(width, length)),
            Seg::Line(Pt::new(side, length)),
            Seg::Line(Pt::new(side, hip_y)),
            Seg::Cubic(Pt::new(side, hip_y - hip_y * 0.45), Pt::new(front_side_top.x, front_side_top.y + hip_y * 0.5), front_side_top),
            Seg::Line(fd[0]),
            Seg::Line(fd[1]),
            Seg::Line(fd[2]),
        ],
    };
    Ok(vec![back, front])
}

/// Cubic segments through `pts` (a Catmull-Rom spline: each point's handles parallel to the line
/// between its neighbours, a sixth of it long; the ends' to the next point), from `pts[0]`.
fn through(pts: &[Pt]) -> Vec<Seg> {
    let n = pts.len();
    let at = |i: isize| pts[i.clamp(0, n as isize - 1) as usize];
    (0..n.saturating_sub(1))
        .map(|i| {
            let i = i as isize;
            let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
            Seg::Cubic(p1 + (p2 - p0) * (1.0 / 6.0), p2 - (p3 - p1) * (1.0 / 6.0), p2)
        })
        .collect()
}

/// The point half way along `a`–`b`, moved `by` square to it, to its left going from `a` to `b`
/// (y down: up and out of the sleeve when going round the crown from back to front).
fn bowed(a: Pt, b: Pt, by: f64) -> Pt {
    let d = (b - a).unit().unwrap_or(Pt::new(1.0, 0.0));
    mid(a, b) + Pt::new(d.y, -d.x) * by
}

/// The crown depth, from the shoulder point down to the crown line, for a top arm (mm): the
/// source's table, 12 cm at a 26–27 cm top arm and 5 mm more for every 2 cm more (to 18 cm at
/// 50–51 cm); beyond the table the same rule goes on.
pub fn crown_depth(top_arm: f64) -> f64 {
    120.0 + 5.0 * ((top_arm - 260.0) / 20.0).floor()
}

/// The length of a cubic from `a`: 64 straight pieces, sharpened by Richardson's rule (four
/// times the 64-piece length less the 32-piece one, over three), well under a micron here.
fn cubic_len(a: Pt, c1: Pt, c2: Pt, b: Pt) -> f64 {
    let at = |t: f64| {
        let m = 1.0 - t;
        a * (m * m * m) + c1 * (3.0 * m * m * t) + c2 * (3.0 * m * t * t) + b * (t * t * t)
    };
    let chords = |n: usize| (0..n).map(|k| (at((k + 1) as f64 / n as f64) - at(k as f64 / n as f64)).len()).sum::<f64>();
    (4.0 * chords(64) - chords(32)) / 3.0
}

/// How many segments the sleeve's crown has on each side (back and front).
const CROWN_SEGS: usize = 4;

/// The length of a sleeve block's crown, from the front underarm over the shoulder point to the
/// back underarm (mm): what is sewn into the armhole.
pub fn sleeve_crown(part: &Part) -> f64 {
    let n = part.segs.len();
    let mut at = part.start;
    let mut total = 0.0;
    for (i, seg) in part.segs.iter().enumerate() {
        let end = match *seg {
            Seg::Line(e) | Seg::Cubic(_, _, e) => e,
        };
        if i < CROWN_SEGS || i + CROWN_SEGS >= n {
            total += match *seg {
                Seg::Line(e) => (e - at).len(),
                Seg::Cubic(c1, c2, e) => cubic_len(at, c1, c2, e),
            };
        }
        at = end;
    }
    total
}

/// The sleeve's outline for a crown `depth`.
fn sleeve_part(top_arm: f64, length: f64, depth: f64) -> Part {
    // 5 cm ease round the top arm, half each side of the centre line; the back and front lines a
    // quarter of the width in from the underarms.
    let half = (top_arm + 50.0) / 2.0;
    let (back_under, front_under) = (Pt::new(-half, depth), Pt::new(half, depth));
    let (back_line, front_line) = (-half / 2.0, half / 2.0);
    let (back_mark, front_mark) = (Pt::new(back_line, depth - 55.0), Pt::new(front_line, depth - 60.0));
    let sp = Pt::new(0.0, 0.0);
    // The crown through the marks, each straight run's middle moved square to it: 1 cm in, 1 cm
    // out (back), 1.25 cm out, 2 cm in (front).
    let crown = [
        back_under,
        bowed(back_under, back_mark, -10.0),
        back_mark,
        bowed(back_mark, sp, 10.0),
        sp,
        bowed(sp, front_mark, 12.5),
        front_mark,
        bowed(front_mark, front_under, -20.0),
        front_under,
    ];
    // The wrist: down to the wrist line at the back line, up 2.5 cm at the front line, through
    // the centre and both side seams half way between.
    let level = length - 12.5;
    let hem = [Pt::new(half, level), Pt::new(front_line, length - 25.0), Pt::new(0.0, level), Pt::new(back_line, length), Pt::new(-half, level)];
    let mut segs = through(&crown[4..]);
    segs.push(Seg::Line(hem[0]));
    segs.extend(through(&hem));
    segs.push(Seg::Line(back_under));
    segs.extend(through(&crown[..5]));
    Part { name: "Sleeve Block", start: sp, segs }
}

/// The one-piece straight sleeve block (part 0): the shoulder point at the origin, the centre
/// line straight down (the grain), the back half on the left, the front on the right. The crown
/// depth comes from the top arm by the table; when the set has a `sleeve_crown` (the length the
/// crown is to be: the bodice's armhole plus the ease to set it in, `armhole + 30`), the depth is
/// found that makes the crown that long instead.
pub fn sleeve(env: &mut Env) -> Result<Vec<Part>, BlockError> {
    let v = need(env, SLEEVE_MEASUREMENTS)?;
    let &[top_arm, length] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    // Set but not working out (a formula over a measurement the set lacks): named as missing,
    // not passed over.
    let target = if env.contains("sleeve_crown") {
        match env.value("sleeve_crown") {
            Ok(t) if t > 0.0 => Some(t),
            _ => return Err(BlockError::Missing(vec!["sleeve_crown".into()])),
        }
    } else {
        None
    };
    let depth = match target {
        None => crown_depth(top_arm),
        Some(t) => {
            // The crown grows with its depth: halve the interval until it is the length asked.
            let len = |d: f64| sleeve_crown(&sleeve_part(top_arm, length, d));
            let (mut lo, mut hi) = (61.0, 400.0);
            if !(len(lo) <= t && t <= len(hi)) {
                return Err(BlockError::Impossible("no crown depth gives a crown that long for this top arm"));
            }
            for _ in 0..60 {
                let m = (lo + hi) / 2.0;
                if len(m) < t { lo = m } else { hi = m }
            }
            (lo + hi) / 2.0
        }
    };
    if depth <= 60.0 {
        return Err(BlockError::Impossible("the top arm is too small for the crown"));
    }
    // The hem curves 2.5 cm up at the front: the sleeve must reach past the crown line by more.
    if length <= depth + 50.0 {
        return Err(BlockError::Impossible("the sleeve is shorter than its crown"));
    }
    Ok(vec![sleeve_part(top_arm, length, depth)])
}

/// The parts of a block.
pub fn draft(kind: BlockKind, env: &mut Env) -> Result<Vec<Part>, BlockError> {
    match kind {
        BlockKind::Bodice => bodice(env),
        BlockKind::Skirt => skirt(env),
        BlockKind::Sleeve => sleeve(env),
    }
}

impl Part {
    /// Every point the outline passes through (segment ends), to measure it.
    pub fn corners(&self) -> Vec<Pt> {
        std::iter::once(self.start)
            .chain(self.segs.iter().map(|s| match s {
                Seg::Line(p) | Seg::Cubic(_, _, p) => *p,
            }))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::Measurements;

    /// A made-up set of measurements (not from any size chart), in mm.
    fn me() -> Measurements {
        let mut m = Measurements::default();
        for (k, v) in [
            ("bust", 880.0),
            ("waist", 680.0),
            ("nape_to_waist", 410.0),
            ("back_width", 345.0),
            ("chest", 325.0),
            ("shoulder", 122.0),
            ("neck", 370.0),
            ("armscye_depth", 210.0),
        ] {
            m.set_value(k, Some(v)).unwrap();
        }
        m
    }

    fn bounds(p: &Part) -> (f64, f64, f64, f64) {
        p.corners().iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(a, b, c, d), q| (a.min(q.x), b.min(q.y), c.max(q.x), d.max(q.y)))
    }

    #[test]
    fn the_bodice_block_is_half_the_bust_with_ease_across_and_waist_length_down() {
        let parts = bodice(&mut me().env()).unwrap();
        assert_eq!(parts.len(), 2);
        let (back, front) = (&parts[0], &parts[1]);
        let (bx0, _, bx1, by1) = bounds(back);
        let (_, fy0, fx1, fy1) = bounds(front);
        // Centre back at 0, centre front at half bust + 5 cm ease.
        assert!(bx0.abs() < 1e-9 && (fx1 - 490.0).abs() < 1e-9, "{bx0} {fx1}");
        // The back reaches the waist (nape to waist); the front dips 0.5 cm at the centre front.
        assert!((by1 - 410.0).abs() < 1e-9 && (fy1 - 415.0).abs() < 1e-9);
        // At an 88 cm bust the front neck is level with the back's top line.
        assert!(fy0.abs() < 1e-9);
        // The back and front meet at the side seam on the armhole line, between back width and chest.
        assert!(bx1 > 345.0 / 2.0 && bx1 < 490.0);
        assert!(back.corners().contains(&Pt::new(bx1, 15.0 + 210.0 + 5.0)) || back.corners().iter().any(|p| (p.y - 230.0).abs() < 1e-9));
    }

    #[test]
    fn the_shoulders_measure_the_shoulder_and_the_darts_follow_the_bust() {
        let p = bodice(&mut me().env()).unwrap();
        let back = p[0].corners();
        // Back neck point (G) to shoulder point (I): the shoulder plus 1 cm, the dart taken out.
        let (g, i) = (back[2], back[6]);
        assert!(((i - g).len() - 132.0).abs() < 1e-9, "{}", (i - g).len());
        // Front: from the far side of the bust dart (W) to the shoulder point (X), the shoulder.
        let front = p[1].corners();
        let (x, w) = (front[9], front[10]);
        assert!(((w - x).len() - 122.0).abs() < 1e-9);
        // The bust dart is 7 cm wide at an 88 cm bust, 8.2 cm at 96 cm.
        let (r, wd) = (front[12], front[10]);
        assert!(((r - wd).len() - 70.0).abs() < 1e-9);
        let mut big = me();
        big.set_value("bust", Some(960.0)).unwrap();
        let front = bodice(&mut big.env()).unwrap()[1].corners();
        assert!(((front[12] - front[10]).len() - 82.0).abs() < 1e-9);
    }

    #[test]
    fn the_skirt_block_is_half_the_hip_with_ease_and_takes_the_waist_in() {
        let mut m = Measurements::default();
        for (k, v) in [("waist", 720.0), ("hip", 960.0), ("waist_to_hip", 200.0), ("skirt_length", 580.0)] {
            m.set_value(k, Some(v)).unwrap();
        }
        let p = skirt(&mut m.env()).unwrap();
        let (back, front) = (bounds(&p[0]), bounds(&p[1]));
        assert!(back.0.abs() < 1e-9 && (front.2 - 490.0).abs() < 1e-9 && (back.3 - 580.0).abs() < 1e-9);
        // Both halves meet at the side seam at the hip line, half way across.
        assert!(p[0].corners().contains(&Pt::new(245.0, 200.0)) && p[1].corners().contains(&Pt::new(245.0, 200.0)));
        // The waist: what's left after the side seams and darts is half the waist.
        let (b, f) = (p[0].corners(), p[1].corners());
        let back_waist = (b[1].x - b[0].x) + (b[4].x - b[3].x);
        let front_waist = (f[5].x - f[4].x) + (f[0].x - f[7].x);
        assert!((back_waist + front_waist - 360.0).abs() < 1e-9, "{back_waist} + {front_waist}");
        // Back dart larger than the front dart.
        assert!(b[3].x - b[1].x > f[7].x - f[5].x);
        let mut m2 = m.clone();
        m2.set_value("skirt_length", Some(150.0)).unwrap();
        assert!(matches!(skirt(&mut m2.env()), Err(BlockError::Impossible(_))));
        m2.set_value("hip", None).unwrap();
        assert!(matches!(skirt(&mut m2.env()), Err(BlockError::Missing(_))));
    }

    #[test]
    fn missing_or_impossible_measurements_are_named() {
        let mut m = me();
        m.set_value("neck", None).unwrap();
        m.set_value("shoulder", None).unwrap();
        assert_eq!(bodice(&mut m.env()), Err(BlockError::Missing(vec!["shoulder".into(), "neck".into()])));
        // The armhole between back and front is open: the chest leaves room past the back width.
        let p = bodice(&mut me().env()).unwrap();
        let side = p[0].corners()[9];
        assert!(side.x > 345.0 / 2.0 + 5.0 + 40.0, "{side:?}");
        let mut m = me();
        m.set_value("nape_to_waist", Some(100.0)).unwrap();
        assert!(matches!(bodice(&mut m.env()), Err(BlockError::Impossible(_))));
        let mut m = me();
        m.set_value("shoulder", Some(5.0)).unwrap();
        assert!(matches!(bodice(&mut m.env()), Err(BlockError::Impossible(_))));
        // A formula can stand for a measurement.
        let mut m = me();
        m.set_formula("waist", Some("bust - 200")).unwrap();
        assert!(bodice(&mut m.env()).is_ok());
    }

    #[test]
    fn the_sleeve_block_has_its_crown_depth_from_the_table_and_ease_round_the_top_arm() {
        assert_eq!(crown_depth(260.0), 120.0);
        assert_eq!(crown_depth(279.0), 120.0);
        assert_eq!(crown_depth(380.0), 150.0);
        assert_eq!(crown_depth(510.0), 180.0);
        let mut m = Measurements::default();
        for (k, v) in [("top_arm", 380.0), ("sleeve_length", 640.0)] {
            m.set_value(k, Some(v)).unwrap();
        }
        let p = sleeve(&mut m.env()).unwrap();
        assert_eq!(p.len(), 1);
        let (l, t, r, b) = bounds(&p[0]);
        // 43 cm wide (38 + 5), the shoulder point at the top, 64 cm long at the back line.
        assert!((r - l - 430.0).abs() < 1e-9 && t.abs() < 1e-9 && (b - 640.0).abs() < 1.0, "{l} {t} {r} {b}");
        let c = p[0].corners();
        assert!(c.contains(&Pt::new(-215.0, 150.0)) && c.contains(&Pt::new(215.0, 150.0)));
        // The front crown scoops deeper than the back: the front mark is lower.
        assert!(c.contains(&Pt::new(107.5, 90.0)) && c.contains(&Pt::new(-107.5, 95.0)));
        // A crown fitted to an armhole: the depth that makes it that long.
        let table = sleeve_crown(&p[0]);
        m.set_formula("sleeve_crown", Some(&format!("{table} + 30"))).unwrap();
        let fitted = sleeve(&mut m.env()).unwrap();
        assert!((sleeve_crown(&fitted[0]) - (table + 30.0)).abs() < 1e-6);
        // A deeper crown, the width kept.
        let c = fitted[0].corners();
        assert!(c[4].x == 215.0 && c[10] == Pt::new(-215.0, c[4].y) && c[4].y > 150.0, "{:?} {:?}", c[4], c[10]);
        assert!(bounds(&fitted[0]).3 == b);
        m.set_formula("sleeve_crown", Some("armhole + 30")).unwrap();
        assert_eq!(sleeve(&mut m.env()), Err(BlockError::Missing(vec!["sleeve_crown".into()])));
        m.set_formula("sleeve_crown", Some("2000")).unwrap();
        assert!(matches!(sleeve(&mut m.env()), Err(BlockError::Impossible(_))));
        m.set_formula("sleeve_crown", None).unwrap();
        m.set_value("sleeve_length", Some(180.0)).unwrap();
        assert!(matches!(sleeve(&mut m.env()), Err(BlockError::Impossible(_))));
        m.set_value("top_arm", None).unwrap();
        assert!(matches!(sleeve(&mut m.env()), Err(BlockError::Missing(_))));
    }
}
