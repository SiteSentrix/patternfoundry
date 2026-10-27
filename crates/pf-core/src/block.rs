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
//! The basic trouser block follows the metric steps published at
//! <https://www.theshapesoffabric.com/2020/08/16/learn-how-to-draft-the-basic-pants-pattern/>
//! (the owner's choice, 2026-10-20; the same author as the skirt): half the hip and 2 cm ease wide,
//! the crotch line 1.5 cm below the crotch depth, crotch extensions of an eighth of the half hip
//! (3 cm more at the back), the waists a quarter waist and 1.5 cm (front) and 0.5 cm with a
//! 2 cm × 10 cm dart (back), the hems 1 cm under and over half the hem, the knees from the line
//! between hem and fork. The curves the page leaves to its drawings are chosen here and marked.
//!
//! The men's trouser block (PF-4.12) follows the metric steps published at
//! <https://www.textileblog.com/mens-pants-pattern-drafting-from-measurement/> (the traditional
//! British men's draft: the front from a crease line, its fork a sixteenth of the seat and 5 mm out
//! from a twelfth and 1.5 cm, a quarter seat and 2 cm across the seat line, a quarter waist and
//! 2.5 cm; the back on the same lines, a quarter seat and 3 cm and a quarter waist and 4.5 cm with a
//! 2.5 cm × 12 cm dart, its fork lower and further out, its hem and knee 2 cm wider each side). The
//! page names but never places its knee points: they are on the line it draws half the inside leg
//! and 5 cm above the hem, 1 cm wider each side of the crease than the hem (chosen; a straight
//! leg's "knee and hem nearly equal", <https://sewingforaliving.com/how-to-draft-a-mens-pants-pattern-from-scratch/>).
//! Its inside leg is the waist to ankle less the crotch depth; its bottom width half the
//! trouser hem round. The curves it leaves to its drawings are chosen here and marked.
//!
//! The men's shirt block (PF-4.13) follows the metric steps published by Open Source Stitches
//! (<https://opensourcestitches.wordpress.com/2011/02/17/pattern-drafting-101-the-mens-shirt-block/>,
//! after Gedwood's BurdaStyle tutorial, itself "partly based on" Winifred Aldrich's *Metric Pattern
//! Cutting for Menswear*): a rectangle half the chest and 8 cm wide, the armhole line the armhole
//! depth and 2.5 cm down; the back's width half the half back and 2.5 cm, the side seam half way
//! between it and a third of the chest and 1.5 cm, less 5 mm; the back neck a fifth of the neck less
//! 5 mm across and 2 cm up, the front neck a fifth deep and a fifth less 1 cm across; the back
//! shoulder to a point a quarter of the armhole depth and 4 cm above half the armhole depth and
//! 3.5 cm out, the front shoulder 5 mm longer, 2 cm lower. Straight sides, no darts. The curves it
//! leaves to its drawings are chosen here (the armhole within the page's distances of its
//! corners) and marked.
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
    /// The basic trouser block: front and back.
    Trousers,
    /// The men's trouser block (PF-4.12): front and back.
    MensTrousers,
    /// The men's shirt block (PF-4.13): back and front.
    MensShirt,
}

/// What a drafted path is: which part of which block, and where the block's origin is in the
/// document (points), so the path can be drawn again when the measurements change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockRef {
    pub kind: BlockKind,
    /// The part (for the bodice: 0 back, 1 front).
    pub part: usize,
    pub origin: Pt,
    /// The block's options set (PF-4.5): its ease and darts; the rest are the block's defaults.
    #[serde(default, skip_serializing_if = "Options::is_empty")]
    pub options: Options,
    /// The design steps made on the part since it was drafted (PF-5.20), made again, in order, when
    /// it is drawn again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<crate::design::DesignStep>,
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

/// The measurements the trouser block is drafted from (all in mm).
pub const TROUSER_MEASUREMENTS: &[(&str, &str)] = &[
    ("waist", "round the natural waist"),
    ("hip", "round the fullest part of the hips"),
    ("waist_to_hip", "down the side from the waist to the fullest part of the hips"),
    ("crotch_depth", "sitting on a flat chair, down the side from the waist to the seat"),
    ("waist_to_knee", "down the side from the waist to the middle of the knee"),
    ("waist_to_ankle", "down the side from the waist to the ankle bone"),
    ("trouser_hem", "round the trouser leg at the hem (a choice, not a body measurement: 46 cm in the source's example)"),
];

/// The measurements the men's trouser block is drafted from (all in mm).
pub const MENS_TROUSER_MEASUREMENTS: &[(&str, &str)] = &[
    ("waist", "round the natural waist"),
    ("hip", "round the seat, its fullest part"),
    ("crotch_depth", "sitting on a flat chair, down the side from the waist to the seat (the body rise)"),
    ("waist_to_ankle", "down the side from the waist to the ankle bone"),
    ("trouser_hem", "round the trouser leg at the hem (a choice, not a body measurement: 50 cm in the source's example)"),
];

/// The measurements the men's shirt block is drafted from (all in mm).
pub const MENS_SHIRT_MEASUREMENTS: &[(&str, &str)] = &[
    ("bust", "round the chest, its fullest part (for a man the chest)"),
    ("neck", "round the base of the neck"),
    ("back_width", "across the back between the armpits' creases"),
    ("armscye_depth", "from the nape down to the level of the armpits"),
    ("nape_to_waist", "down the spine from the nape to the waist"),
];

/// The measurements a block is drafted from.
pub fn measurements(kind: BlockKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        BlockKind::Bodice => BODICE_MEASUREMENTS,
        BlockKind::Skirt => SKIRT_MEASUREMENTS,
        BlockKind::Sleeve => SLEEVE_MEASUREMENTS,
        BlockKind::Trousers => TROUSER_MEASUREMENTS,
        BlockKind::MensTrousers => MENS_TROUSER_MEASUREMENTS,
        BlockKind::MensShirt => MENS_SHIRT_MEASUREMENTS,
    }
}

/// A block's options (PF-4.5) by name, in mm: an option left out is the block's default.
pub type Options = std::collections::BTreeMap<String, f64>;

/// One option of a block: its ease or a dart's size, which the source gives one value and a
/// pattern maker changes for the fit or the style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockOption {
    pub name: &'static str,
    pub what: &'static str,
    /// The source's value (mm); none when it is worked out from the measurements.
    pub default: Option<f64>,
    pub min: f64,
    pub max: f64,
}

const fn opt(name: &'static str, what: &'static str, default: Option<f64>, min: f64, max: f64) -> BlockOption {
    BlockOption { name, what, default, min, max }
}

/// The bodice block's options.
pub const BODICE_OPTIONS: &[BlockOption] = &[
    opt("ease", "ease across the half bust, at the chest line", Some(50.0), 0.0, 300.0),
    opt("waist_ease", "ease at the half waist, before the darts take the rest in", Some(30.0), 0.0, 200.0),
    opt("bust_dart", "the bust dart's width at the shoulder (by default 7 cm at an 88 cm bust, 0.6 cm more each 4 cm more)", None, 0.0, 200.0),
    opt("shoulder_dart", "the back shoulder dart's width", Some(10.0), 0.0, 50.0),
];

/// The skirt block's options.
pub const SKIRT_OPTIONS: &[BlockOption] = &[
    opt("ease", "ease at the half hip", Some(10.0), 0.0, 300.0),
    opt("back_dart_length", "the back darts' length (never past 3 cm above the hip line)", Some(140.0), 10.0, 400.0),
    opt("front_dart_length", "the front darts' length (never past 3 cm above the hip line)", Some(100.0), 10.0, 400.0),
];

/// The sleeve block's options.
pub const SLEEVE_OPTIONS: &[BlockOption] = &[
    opt("ease", "ease round the top arm", Some(50.0), 0.0, 300.0),
    opt(
        "crown_ease",
        "ease the crown takes in over the bodice's armhole, when the document has a bodice block (by default the middle of a classic sleeve's 1½ to 2 in)",
        Some(CLASSIC_CROWN_EASE),
        0.0,
        100.0,
    ),
];

/// The ease a classic sleeve's crown takes in by default: the middle of 1½ to 2 in (University of
/// Kentucky, FCS2-807), mm.
pub const CLASSIC_CROWN_EASE: f64 = 44.45;

/// The trouser block's options.
pub const MENS_SHIRT_OPTIONS: &[BlockOption] = &[opt(
    "length",
    "the shirt's length down the centre back from the nape (the source leaves it to the maker; chosen: 78 cm)",
    Some(780.0),
    300.0,
    1300.0,
)];

pub const MENS_TROUSER_OPTIONS: &[BlockOption] = &[
    opt("waistband", "the waistband's depth: the block's top is its lower edge", Some(40.0), 0.0, 100.0),
    opt("easy_fit", "added along the back crotch for an easy fit (the source: 3.5 cm)", Some(0.0), 0.0, 60.0),
    opt("back_dart", "the back waist dart's width", Some(25.0), 0.0, 60.0),
    opt("back_dart_length", "the back waist dart's length", Some(120.0), 10.0, 250.0),
];

pub const TROUSER_OPTIONS: &[BlockOption] = &[
    opt("ease", "ease at the half hip", Some(20.0), 0.0, 300.0),
    opt(
        "front_dart",
        "a front waist dart's width, added at the front side seam (none by default; the source's author suggests about 1 cm)",
        Some(0.0),
        0.0,
        60.0,
    ),
    opt("front_dart_length", "the front waist dart's length", Some(80.0), 10.0, 200.0),
    opt("back_dart", "the back waist dart's width", Some(20.0), 0.0, 100.0),
    opt("back_dart_length", "the back waist dart's length", Some(100.0), 10.0, 300.0),
];

/// A block's options.
pub fn options(kind: BlockKind) -> &'static [BlockOption] {
    match kind {
        BlockKind::Bodice => BODICE_OPTIONS,
        BlockKind::Skirt => SKIRT_OPTIONS,
        BlockKind::Sleeve => SLEEVE_OPTIONS,
        BlockKind::Trousers => TROUSER_OPTIONS,
        BlockKind::MensTrousers => MENS_TROUSER_OPTIONS,
        BlockKind::MensShirt => MENS_SHIRT_OPTIONS,
    }
}

/// Whether `o` are options of `kind` in their ranges.
pub fn check_options(kind: BlockKind, o: &Options) -> Result<(), BlockError> {
    for (name, v) in o {
        let Some(spec) = options(kind).iter().find(|s| s.name == name) else {
            return Err(BlockError::Option(format!("the block has no option `{name}`")));
        };
        if !(v.is_finite() && (spec.min..=spec.max).contains(v)) {
            return Err(BlockError::Option(format!("{name} {v} mm: give {} to {} mm", spec.min, spec.max)));
        }
    }
    Ok(())
}

/// Option `name`'s value in `o`, else `default`.
fn value(o: &Options, name: &str, default: f64) -> f64 {
    o.get(name).copied().unwrap_or(default)
}

/// Why a block can't be drafted.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockError {
    /// Measurements the set doesn't have (or that don't work out).
    Missing(Vec<String>),
    /// Measurements that don't make the block (a shoulder too short to reach the armhole line…).
    Impossible(&'static str),
    /// An option the block doesn't have, or out of its range.
    Option(String),
}

impl std::fmt::Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockError::Missing(m) => write!(f, "the block needs these measurements: {}", m.join(", ")),
            BlockError::Impossible(why) => write!(f, "these measurements don't make the block: {why}"),
            BlockError::Option(why) => f.write_str(why),
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
pub fn bodice(env: &mut Env, opts: &Options) -> Result<Vec<Part>, BlockError> {
    let v = need(env, BODICE_MEASUREMENTS)?;
    let &[bust, waist, nape_to_waist, back_width, chest, shoulder, neck, depth] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    let half_bust = bust / 2.0;
    let (ease, waist_ease) = (value(opts, "ease", 50.0), value(opts, "waist_ease", 30.0));
    // The construction frame.
    let o = Pt::new(0.0, 0.0);
    let a = Pt::new(0.0, 15.0);
    let b = Pt::new(0.0, a.y + depth + 5.0);
    let c = Pt::new(half_bust + ease, b.y);
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
    // The back shoulder dart: 1 cm wide at the shoulder (the `shoulder_dart` option), its point
    // 5 cm below and 1 cm toward the centre back from the shoulder's middle.
    let j = mid(g, i);
    let k = Pt::new(j.x - 10.0, j.y + 50.0);
    let along = (i - g).unit().unwrap_or(Pt::new(1.0, 0.0));
    let sd = value(opts, "shoulder_dart", 10.0) / 2.0;
    let (dart_in, dart_out) = (j - along * sd, j + along * sd);
    // Back width line and armhole points.
    let l = Pt::new(back_width / 2.0 + 5.0, b.y);
    let m = Pt::new(l.x, h.y);
    // The back armhole touches the back width line half way between the shoulder line and L.
    let n = mid(l, m);
    let p = mid(b, l);
    // Front neck.
    let r = Pt::new(d.x - (neck / 5.0 + 7.0), d.y);
    let s = Pt::new(d.x, d.y + neck / 5.0 - 2.0);
    // The bust dart: 7 cm at an 88 cm bust, 0.6 cm more for every 4 cm more bust (unless the
    // `bust_dart` option sets it).
    let dart = value(opts, "bust_dart", 70.0 + 6.0 * (bust - 880.0) / 40.0);
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
    let take = (half_bust + ease) - (waist / 2.0 + waist_ease);
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
pub fn skirt(env: &mut Env, o: &Options) -> Result<Vec<Part>, BlockError> {
    let v = need(env, SKIRT_MEASUREMENTS)?;
    let &[waist, hip, waist_to_hip, length] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    if waist_to_hip >= length {
        return Err(BlockError::Impossible("the skirt is shorter than the waist to hip depth"));
    }
    let width = hip / 2.0 + value(o, "ease", 10.0);
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
    let (back_len, front_len) =
        (value(o, "back_dart_length", 140.0).min(waist_to_hip - 30.0), value(o, "front_dart_length", 100.0).min(waist_to_hip - 30.0));
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
            // The back's hip curve reversed (its handles from the side seam's full height, the rise
            // included), so the side seams are as long and sew together.
            Seg::Cubic(
                Pt::new(side, hip_y - (hip_y + rise) * 0.45),
                Pt::new(front_side_top.x, front_side_top.y + (hip_y + rise) * 0.5),
                front_side_top,
            ),
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
fn sleeve_part(top_arm: f64, length: f64, depth: f64, ease: f64) -> Part {
    // 5 cm ease round the top arm (the `ease` option), half each side of the centre line; the back
    // and front lines a quarter of the width in from the underarms.
    let half = (top_arm + ease) / 2.0;
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
pub fn sleeve(env: &mut Env, o: &Options) -> Result<Vec<Part>, BlockError> {
    sleeve_for(env, o, None)
}

/// [`sleeve`] set into a bodice whose armhole (back and front, mm) is `armhole` (PF-4.9, the
/// owner's decision, 2026-10-23): without a `sleeve_crown` measurement the crown is that armhole
/// plus the `crown_ease` option (the middle of a classic sleeve's ease by default), instead of the
/// depth from the top arm table, which leaves a crown about as long as the armhole, without ease.
pub fn sleeve_for(env: &mut Env, o: &Options, armhole: Option<f64>) -> Result<Vec<Part>, BlockError> {
    check_options(BlockKind::Sleeve, o)?;
    let ease = value(o, "ease", 50.0);
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
        armhole.filter(|a| a.is_finite() && *a > 0.0).map(|a| a + value(o, "crown_ease", CLASSIC_CROWN_EASE))
    };
    let depth = match target {
        None => crown_depth(top_arm),
        Some(t) => {
            // The crown grows with its depth: halve the interval until it is the length asked.
            let len = |d: f64| sleeve_crown(&sleeve_part(top_arm, length, d, ease));
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
    Ok(vec![sleeve_part(top_arm, length, depth, ease)])
}

/// The front and back of the basic trouser block (parts 0 and 1), side by side in one frame: the
/// top of the frame (the side seams' waist line) at y = 0, the front's half on the left (its
/// centre front at x = 0), the back's on the right (its centre back at the frame's right edge),
/// the side seams on the frame's middle line down to the crotch line.
pub fn trousers(env: &mut Env, o: &Options) -> Result<Vec<Part>, BlockError> {
    let v = need(env, TROUSER_MEASUREMENTS)?;
    let &[waist, hip, waist_to_hip, crotch_depth, waist_to_knee, waist_to_ankle, hem] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    let (half_hip, half_waist) = (hip / 2.0, waist / 2.0);
    // The frame: half the hip and 2 cm ease wide, waist to ankle long; the crotch line 1.5 cm
    // below the crotch depth.
    let width = half_hip + value(o, "ease", 20.0);
    let side = width / 2.0;
    let (hip_y, crotch_y, knee_y, hem_y) = (waist_to_hip, crotch_depth + 15.0, waist_to_knee, waist_to_ankle);
    if !(hip_y < crotch_y && crotch_y < knee_y && knee_y < hem_y) {
        return Err(BlockError::Impossible("the waist to hip, crotch depth, waist to knee and waist to ankle must come one below the other"));
    }
    // The crotch extensions, out from the centre front and the centre back along the crotch
    // line (the source says only "extended left" and "right": out from the frame's edges, as the
    // centre lines it draws reach the hip line there).
    let front_ext = half_hip / 8.0;
    let back_ext = front_ext + 30.0;
    let front_fork = Pt::new(-front_ext, crotch_y);
    let back_fork = Pt::new(width + back_ext, crotch_y);
    // Centre front: 0.5 cm in from the frame's top-left corner and 1 cm down, straight to the
    // hip line at the frame's edge. Centre back: 4 cm in from the top-right corner, straight
    // from the hip line through it and 2.5 cm on, above the frame.
    let cf_top = Pt::new(5.0, 10.0);
    let cf_hip = Pt::new(0.0, hip_y);
    let cb_hip = Pt::new(width, hip_y);
    let cb_up = (Pt::new(width - 40.0, 0.0) - cb_hip).unit().unwrap_or(Pt::new(0.0, -1.0));
    let cb_top = Pt::new(width - 40.0, 0.0) + cb_up * 25.0;
    // The waists: a quarter waist and 1.5 cm at the front, and 0.5 cm at the back (with its
    // 2 cm dart), each straight from the centre line to the frame's top line.
    let to_top = |from: Pt, len: f64, dir: f64| -> Option<Pt> {
        let d2 = len * len - from.y * from.y;
        (d2 > 0.0).then(|| Pt::new(from.x + dir * d2.sqrt(), 0.0))
    };
    // A front dart (the `front_dart` option, none by default) takes its width out again: the
    // tutorial's author adds "that extra width to the side seam of the front piece" for one.
    let front_dart = value(o, "front_dart", 0.0);
    let front_len = half_waist / 2.0 + 15.0 + front_dart;
    let back_len = half_waist / 2.0 + 5.0;
    let front_side_top = to_top(cf_top, front_len, 1.0).ok_or(BlockError::Impossible("the waist is too small"))?;
    let back_side_top = to_top(cb_top, back_len, -1.0).ok_or(BlockError::Impossible("the waist is too small"))?;
    if front_side_top.x >= side || back_side_top.x <= side {
        return Err(BlockError::Impossible("the waist is wider than the hips with ease"));
    }
    let side_hip = Pt::new(side, hip_y);
    let side_crotch = Pt::new(side, crotch_y);
    // The crease line (grain) half way between the front fork and the side seam on the crotch
    // line; the back's as far from the side seam.
    let front_grain = (front_fork.x + side) / 2.0;
    let back_grain = side + (side - front_grain);
    // The hems: half the hem round, 1 cm less at the front and 1 cm more at the back, centred
    // on the crease lines.
    let (front_hem, back_hem) = ((hem / 2.0 - 10.0) / 2.0, (hem / 2.0 + 10.0) / 2.0);
    if front_hem <= 0.0 {
        return Err(BlockError::Impossible("the hem is too small"));
    }
    // The front knee: the straight line from the hem to the fork, 1 cm in at knee level; as far
    // from the crease on the side seam. The back knee 1 cm wider each side.
    let fhi = Pt::new(front_grain - front_hem, hem_y);
    let t = (knee_y - crotch_y) / (hem_y - crotch_y);
    let on_line = front_fork + (fhi - front_fork) * t;
    let front_knee = front_grain - (on_line.x + 10.0);
    if front_knee <= 0.0 {
        return Err(BlockError::Impossible("the leg is too narrow at the knee"));
    }
    let back_knee = front_knee + 10.0;
    let fki = Pt::new(front_grain - front_knee, knee_y);
    let fks = Pt::new(front_grain + front_knee, knee_y);
    let fhs = Pt::new(front_grain + front_hem, hem_y);
    let bki = Pt::new(back_grain + back_knee, knee_y);
    let bks = Pt::new(back_grain - back_knee, knee_y);
    let bhi = Pt::new(back_grain + back_hem, hem_y);
    let bhs = Pt::new(back_grain - back_hem, hem_y);
    // Curves the source leaves to its drawings, chosen here: each leaves its straight part in
    // the same direction (no corner at the knee), the inseams meet the forks square to the
    // crotch line (more curved at the back), the side seams reach the side line square at the
    // crotch line, the front crotch is gentle and the back deeper (leaving the hip line along the
    // slanted centre back, without a corner), the waists meet the centre lines square.
    let rise = crotch_y - hip_y;
    let up = |from: Pt, below: Pt, len: f64| from + (from - below).unit().unwrap_or(Pt::new(0.0, -1.0)) * len;
    let thigh = knee_y - crotch_y;
    let cf_dir = cf_hip - cf_top;
    let cb_dir = cb_hip - cb_top;
    // Front: from the centre front at the waist, down the centre front and the crotch, down the
    // inseam, across the hem, up the side seam, the waist.
    let mut front = Part {
        name: "Front Trouser Block",
        start: cf_top,
        segs: vec![
            Seg::Line(cf_hip),
            Seg::Cubic(Pt::new(0.0, hip_y + rise * 0.55), Pt::new(front_fork.x * 0.5, crotch_y), front_fork),
            Seg::Cubic(Pt::new(front_fork.x, crotch_y + thigh * 0.3), up(fki, fhi, thigh * 0.35), fki),
            Seg::Line(fhi),
            Seg::Line(fhs),
            Seg::Line(fks),
            Seg::Cubic(up(fks, fhs, thigh * 0.35), Pt::new(side, crotch_y + thigh * 0.35), side_crotch),
            Seg::Line(side_hip),
            Seg::Cubic(Pt::new(side, hip_y * 0.6), front_side_top + (side_hip - front_side_top) * 0.3, front_side_top),
        ],
    };
    let cf_square = Pt::new(cf_dir.y, -cf_dir.x).unit().unwrap_or(Pt::new(1.0, 0.0));
    if front_dart > 0.0 {
        // The dart in the middle of the waist, square to it, `front_dart_length` long (8 cm unless
        // set: the length is left open, chosen here).
        let along = (cf_top - front_side_top).unit().unwrap_or(Pt::new(-1.0, 0.0));
        let mid_waist = (front_side_top + cf_top) * 0.5;
        let (leg_side, leg_cf) = (mid_waist - along * (front_dart / 2.0), mid_waist + along * (front_dart / 2.0));
        let down = Pt::new(-along.y, along.x);
        let down = if down.y < 0.0 { down * -1.0 } else { down };
        let point = mid_waist + down * value(o, "front_dart_length", 80.0);
        let rest = (cf_top - leg_cf).len();
        front.segs.extend([
            Seg::Line(leg_side),
            Seg::Line(point),
            Seg::Line(leg_cf),
            Seg::Cubic(leg_cf + (cf_top - leg_cf) * 0.33, cf_top + cf_square * (rest * 0.33), cf_top),
        ]);
    } else {
        front.segs.push(Seg::Cubic(front_side_top + (cf_top - front_side_top) * 0.33, cf_top + cf_square * (front_len * 0.33), cf_top));
    }
    // Back: from the centre back at the waist, across the waist with its dart (2 cm wide, 10 cm
    // long, square to the waist at its middle), down the side seam, across the hem, up the
    // inseam, the crotch and the centre back.
    let along = (back_side_top - cb_top).unit().unwrap_or(Pt::new(-1.0, 0.0));
    let mid_waist = (cb_top + back_side_top) * 0.5;
    let half_dart = value(o, "back_dart", 20.0) / 2.0;
    let (dart_cb, dart_side) = (mid_waist - along * half_dart, mid_waist + along * half_dart);
    let down = Pt::new(-along.y, along.x);
    let down = if down.y < 0.0 { down * -1.0 } else { down };
    let dart_point = mid_waist + down * value(o, "back_dart_length", 100.0);
    let cb_square = Pt::new(-cb_dir.y, cb_dir.x).unit().unwrap_or(Pt::new(-1.0, 0.0));
    let cb_square = if cb_square.x > 0.0 { cb_square * -1.0 } else { cb_square };
    let back = Part {
        name: "Back Trouser Block",
        start: cb_top,
        segs: vec![
            Seg::Cubic(cb_top + cb_square * (back_len * 0.15), dart_cb - along * (back_len * 0.1), dart_cb),
            Seg::Line(dart_point),
            Seg::Line(dart_side),
            Seg::Line(back_side_top),
            Seg::Cubic(back_side_top + (side_hip - back_side_top) * 0.3, Pt::new(side, hip_y * 0.6), side_hip),
            Seg::Line(side_crotch),
            Seg::Cubic(Pt::new(side, crotch_y + thigh * 0.35), up(bks, bhs, thigh * 0.35), bks),
            Seg::Line(bhs),
            Seg::Line(bhi),
            Seg::Line(bki),
            Seg::Cubic(up(bki, bhi, thigh * 0.35), Pt::new(back_fork.x, crotch_y + thigh * 0.45), back_fork),
            Seg::Cubic(Pt::new(width + back_ext * 0.35, crotch_y), cb_hip + cb_dir.unit().unwrap_or(Pt::new(0.0, 1.0)) * (rise * 0.75), cb_hip),
            Seg::Line(cb_top),
        ],
    };
    Ok(vec![front, back])
}

/// The men's shirt block (PF-4.13, see the module docs): the back (centre back on the left) and
/// the front (centre front on the right), half bodies, set apart. The points are the source's,
/// lettered as it letters them, from A at the centre back's neck; y down.
pub fn mens_shirt(env: &mut Env, o: &Options) -> Result<Vec<Part>, BlockError> {
    let v = need(env, MENS_SHIRT_MEASUREMENTS)?;
    let &[chest, neck, back_width, scye, nape_to_waist] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    let length = value(o, "length", 780.0);
    if length <= nape_to_waist + 20.0 || scye + 25.0 >= nape_to_waist {
        return Err(BlockError::Impossible("the shirt must reach below the waist, the waist below the armholes"));
    }
    let half_back = back_width / 2.0;
    // The rectangle: the armhole line (B–C) the armhole depth and 2.5 cm down, half the chest and
    // 8 cm wide; the hem (G–H) the length and 1 cm down.
    let y_b = scye + 25.0;
    let x_c = chest / 2.0 + 80.0;
    let y_g = length + 10.0;
    // I the back's width on the armhole line, K a third of the chest and 1.5 cm, L 3.5 cm above K;
    // the side seam (M) half way between I and K less 5 mm; P half the armhole depth and 1 cm down,
    // Q level with it above I.
    let x_i = half_back + 25.0;
    let x_k = chest / 3.0 + 15.0;
    if x_k <= x_i || x_k >= x_c {
        return Err(BlockError::Impossible("the half back is too wide for the chest"));
    }
    let x_m = x_k - ((x_k - x_i) / 2.0 - 5.0);
    let l = Pt::new(x_k, y_b - 35.0);
    let q = Pt::new(x_i, scye / 2.0 + 10.0);
    // The back neck: R a fifth of the neck less 5 mm along the top, S 2 cm above it; the back
    // shoulder to U, 3.5 cm out from T, a quarter of the armhole depth and 4 cm above Q.
    let s = Pt::new(neck / 5.0 - 5.0, -20.0);
    let u = Pt::new(x_i + 35.0, q.y - (scye / 4.0 + 40.0));
    // The front neck: V a fifth of the neck down the centre front, W a fifth less 1 cm in along the
    // top; the shoulder from W to Y, 5 mm longer than the back's, level with X 2 cm below U.
    let v_ = Pt::new(x_c, neck / 5.0);
    let w = Pt::new(x_c - (neck / 5.0 - 10.0), 0.0);
    let y_x = u.y + 20.0;
    let shoulder = (u - s).len() + 5.0;
    let run = shoulder * shoulder - y_x * y_x;
    if run <= 0.0 {
        return Err(BlockError::Impossible("the front shoulder can't reach the shoulder line"));
    }
    let y = Pt::new(w.x - run.sqrt(), y_x);
    let (m, o_) = (Pt::new(x_m, y_b), Pt::new(x_m, y_g));
    // Curves the page leaves to its drawings, chosen: the necks square to the centre lines; the
    // back armhole from U down to Q, then round the corner at I (within 3 cm of it, as the page
    // asks) to M on the armhole line; the front's from M round the corner at K (within 1.75 cm) up
    // through L, then (within 1.25 cm of the line L–Y) to Y.
    let back = Part {
        name: "Back Men's Shirt Block",
        start: Pt::new(0.0, 0.0),
        segs: vec![
            Seg::Cubic(Pt::new(s.x * 0.55, 0.0), Pt::new(s.x * 0.92, s.y * 0.4), s),
            Seg::Line(u),
            Seg::Cubic(Pt::new(u.x - 6.0, u.y + (q.y - u.y) * 0.45), Pt::new(x_i, q.y - (q.y - u.y) * 0.3), q),
            Seg::Cubic(Pt::new(x_i, q.y + (y_b - q.y) * 0.8), Pt::new(x_i + (x_m - x_i) * 0.1, y_b), m),
            Seg::Line(o_),
            Seg::Line(Pt::new(0.0, y_g)),
            Seg::Line(Pt::new(0.0, 0.0)),
        ],
    };
    let front = Part {
        name: "Front Men's Shirt Block",
        start: w,
        segs: vec![
            Seg::Cubic(Pt::new(w.x, v_.y * 0.55), Pt::new(x_c - (x_c - w.x) * 0.55, v_.y), v_),
            Seg::Line(Pt::new(x_c, y_g)),
            Seg::Line(o_),
            Seg::Line(m),
            Seg::Cubic(Pt::new(m.x + (x_k - m.x) * 0.6, y_b), Pt::new(x_k, y_b - 12.0), l),
            Seg::Cubic(Pt::new(l.x, l.y - (l.y - y.y) * 0.35), y + (l - y) * 0.3 + Pt::new(4.0, 0.0), y),
            Seg::Line(w),
        ],
    };
    // Side by side: the front moved 6 cm right of the back.
    let shift = |p: Pt| Pt::new(p.x + 60.0, p.y);
    let front = moved(&front, shift);
    Ok(vec![back, front])
}

/// The men's trouser block (PF-4.12, see the module docs): front then back, each a half pair (one
/// leg), both with the fork on the left and the side seam on the right (the back drawn over the
/// front's lines as tailors draw it, then set beside it; cut as a mirror-image pair each, so the
/// pieces pair up leg by leg). The points are the
/// source's, numbered as it numbers them; x from the crease line (0, 1, 2 straight down it),
/// y down from the block's top (the waistband's lower edge).
pub fn mens_trousers(env: &mut Env, o: &Options) -> Result<Vec<Part>, BlockError> {
    let v = need(env, MENS_TROUSER_MEASUREMENTS)?;
    let &[waist, seat, rise, waist_to_ankle, hem] = v.as_slice() else {
        return Err(BlockError::Missing(vec![]));
    };
    let band = value(o, "waistband", 40.0);
    let inside_leg = waist_to_ankle - rise;
    let bottom = hem / 2.0;
    if inside_leg <= 100.0 || rise <= band {
        return Err(BlockError::Impossible("the waist to ankle must be longer than the crotch depth, the crotch depth than the waistband"));
    }
    // The lines: the crotch (1) the body rise and 1 cm less the waistband down, the hem (2) the
    // inside leg below it, the knee (3) half the inside leg and 5 cm above the hem, the seat (4) a
    // quarter of the body rise above the crotch.
    let y1 = rise + 10.0 - band;
    let y2 = y1 + inside_leg;
    let y3 = y2 - (inside_leg / 2.0 + 50.0);
    let y4 = y1 - rise / 4.0;
    // The front: 5 a twelfth of the seat and 1.5 cm from the crease on the crotch line, the fork
    // (9) a sixteenth and 5 mm beyond it; 6 and 7 above 5 on the seat line and the top; the side
    // at the seat (8) a quarter seat and 2 cm from 6; the centre front's top (10) 1 cm in from 7,
    // the side's (11) a quarter waist and 2.5 cm from it; the hem (12, 13) half the bottom width
    // each side of the crease, the knee (14, 15) 1 cm wider.
    let x5 = -(seat / 12.0 + 15.0);
    let x9 = x5 - (seat / 16.0 + 5.0);
    let p6 = Pt::new(x5, y4);
    let p8 = Pt::new(x5 + seat / 4.0 + 20.0, y4);
    let p9 = Pt::new(x9, y1);
    let p10 = Pt::new(x5 + 10.0, 0.0);
    let p11 = Pt::new(p10.x + waist / 4.0 + 25.0, 0.0);
    let half_knee = bottom / 2.0 + 10.0;
    let (p12, p13) = (Pt::new(bottom / 2.0, y2), Pt::new(-bottom / 2.0, y2));
    let (p14, p15) = (Pt::new(half_knee, y3), Pt::new(-half_knee, y3));
    if p11.x <= p10.x || p8.x <= p14.x - 1.0 || p9.x >= p15.x {
        return Err(BlockError::Impossible("the seat is too small for the trouser hem"));
    }
    // Curves the source leaves to its drawings, chosen: the centre front straight down to the seat
    // line and round the fork to it, square to the crotch line there; the side seam 5 mm out between
    // the waist and the seat and 5 mm in below it (as the source says), on straight to the knee and
    // hem; the inside leg 1 cm in below the fork (as the source says).
    let thigh = y3 - y1;
    let front = Part {
        name: "Front Men's Trouser Block",
        start: p10,
        segs: vec![
            Seg::Line(p6),
            Seg::Cubic(Pt::new(x5, y4 + (y1 - y4) * 0.6), Pt::new(x5 + (x9 - x5) * 0.5, y1), p9),
            Seg::Cubic(Pt::new(x9 + 10.0, y1 + thigh * 0.35), Pt::new(p15.x - 5.0, y3 - thigh * 0.3), p15),
            Seg::Line(p13),
            Seg::Line(p12),
            Seg::Line(p14),
            Seg::Cubic(Pt::new(p14.x + 2.0, y3 - thigh * 0.4), Pt::new(p8.x - 5.0, y1 - (y1 - y4) * 0.2), p8),
            Seg::Cubic(Pt::new(p8.x + 3.0, y4 * 0.6), p11 + (p8 - p11) * 0.3 + Pt::new(5.0, 0.0), p11),
            Seg::Line(p10),
        ],
    };
    // The back on the same lines: 16 a quarter of 1–5 in from 5, 17 and 18 above it on the seat line
    // and the top, 19 half way between; the centre back's top (21) 2 cm in from 18 and 1 cm up; the
    // fork (23) half of 5–9 and 5 mm beyond 9, 5 mm down, and (an easy fit) the option further out;
    // the side at the seat (26) a quarter seat and 3 cm from 17, at the top (24) a quarter waist and
    // 4.5 cm from 21, the dart in the middle; the hem and knee 2 cm wider each side than the front's.
    let x16 = x5 + (-x5) / 4.0;
    let p17 = Pt::new(x16, y4);
    let p19 = Pt::new(x16, y4 / 2.0);
    let p21 = Pt::new(x16 + 20.0, -10.0);
    let easy = value(o, "easy_fit", 0.0);
    let p23 = Pt::new(x9 - ((x5 - x9) / 2.0 + 5.0) - easy, y1 + 5.0);
    let p26 = Pt::new(p17.x + seat / 4.0 + 30.0, y4);
    let w = waist / 4.0 + 45.0;
    let dy = 10.0f64; // 21 is 1 cm above the top line: 24 on it, `w` from 21
    let p24 = Pt::new(p21.x + (w * w - dy * dy).max(1.0).sqrt(), 0.0);
    let (p27, p28, p29, p30) = (p12 + Pt::new(20.0, 0.0), p13 - Pt::new(20.0, 0.0), p14 + Pt::new(20.0, 0.0), p15 - Pt::new(20.0, 0.0));
    let along = (p24 - p21).unit().unwrap_or(Pt::new(1.0, 0.0));
    let mid = (p21 + p24) * 0.5;
    let half_dart = value(o, "back_dart", 25.0) / 2.0;
    let down = Pt::new(-along.y, along.x);
    let down = if down.y < 0.0 { down * -1.0 } else { down };
    let (dart_cb, dart_side) = (mid - along * half_dart, mid + along * half_dart);
    let dart_point = mid + down * value(o, "back_dart_length", 120.0);
    let back = Part {
        name: "Back Men's Trouser Block",
        start: p21,
        segs: vec![
            Seg::Line(dart_cb),
            Seg::Line(dart_point),
            Seg::Line(dart_side),
            Seg::Line(p24),
            Seg::Cubic(p24 + (p26 - p24) * 0.3 + Pt::new(6.0, 0.0), Pt::new(p26.x + 3.0, y4 - (y4 * 0.2)), p26),
            Seg::Cubic(Pt::new(p26.x - 5.0, y1 - (y1 - y4) * 0.2), Pt::new(p29.x + 2.0, y3 - thigh * 0.4), p29),
            Seg::Line(p27),
            Seg::Line(p28),
            Seg::Line(p30),
            Seg::Cubic(Pt::new(p30.x - 5.0, y3 - thigh * 0.3), Pt::new(p23.x + 15.0, y1 + thigh * 0.3), p23),
            Seg::Cubic(Pt::new(p23.x + (x16 - p23.x) * 0.55, y1 + 2.0), Pt::new(x16, y4 + (y1 - y4) * 0.35), p17),
            Seg::Line(p19),
            Seg::Line(p21),
        ],
    };
    // Beside the front, to its right, turned over (its fork on the right, its side seam on the left,
    // as the women's trouser block lays its back out): drawn on the front's lines a back has its fork
    // on the same side as the front's, so its copy as drawn is the other leg's, and the pieces' first
    // copies, sewn to each other, would join one leg's front to the other leg's back.
    let right = p8.x.max(p14.x);
    let gap = right + 60.0 + (p26.x.max(p29.x) - p23.x.min(p28.x).min(x9));
    let flip = |p: Pt| Pt::new(gap - (p.x - p23.x.min(p28.x).min(x9)), p.y);
    Ok(vec![front, moved(&back, flip)])
}

/// `part` with every point moved by `f` (to set a part beside another, or turn it over: a mirror
/// image runs the other way round on the page, as a block's back and front do; the women's trouser
/// block's both run the same way, its back drawn so).
fn moved(part: &Part, f: impl Fn(Pt) -> Pt) -> Part {
    let flip = f;
    let segs = part
        .segs
        .iter()
        .map(|s| match *s {
            Seg::Line(a) => Seg::Line(flip(a)),
            Seg::Cubic(a, b, c) => Seg::Cubic(flip(a), flip(b), flip(c)),
        })
        .collect();
    Part { name: part.name, start: flip(part.start), segs }
}

/// The length along `part`'s outline from anchor `from` forward to anchor `to` (mm): anchor 0 is
/// the start, anchor `i` the end of segment `i - 1`; going past the last anchor comes round.
pub fn run_length(part: &Part, from: usize, to: usize) -> f64 {
    let n = part.segs.len();
    if n == 0 {
        return 0.0;
    }
    let corners = part.corners();
    let mut total = 0.0;
    let mut i = from % n;
    while i != to % n {
        let at = corners[i];
        total += match part.segs[i] {
            Seg::Line(e) => (e - at).len(),
            Seg::Cubic(c1, c2, e) => cubic_len(at, c1, c2, e),
        };
        i = (i + 1) % n;
    }
    total
}

/// The bodice block's armhole as drafted with `o` (mm): the back's and the front's, shoulder point
/// to side seam.
pub fn bodice_armhole(env: &mut Env, o: &Options) -> Result<f64, BlockError> {
    let parts = draft(BlockKind::Bodice, env, o)?;
    let mut total = 0.0;
    for (i, part) in parts.iter().enumerate() {
        let (from, to, _) = armhole(BlockKind::Bodice, i).ok_or(BlockError::Impossible("the bodice has no armhole"))?;
        total += run_length(part, from, to);
    }
    Ok(total)
}

/// The parts of a block.
pub fn draft(kind: BlockKind, env: &mut Env, o: &Options) -> Result<Vec<Part>, BlockError> {
    check_options(kind, o)?;
    match kind {
        BlockKind::Bodice => bodice(env, o),
        BlockKind::Skirt => skirt(env, o),
        BlockKind::Sleeve => sleeve(env, o),
        BlockKind::Trousers => trousers(env, o),
        BlockKind::MensTrousers => mens_trousers(env, o),
        BlockKind::MensShirt => mens_shirt(env, o),
    }
}

/// Where a block part's armhole (a bodice part's) or crown (the sleeve's, underarm to underarm
/// over the shoulder point) runs: from one anchor forward along the outline to another, and how
/// many anchors the outline has as drafted. A part changed since (a dart moved into the armhole)
/// has other anchors: its seams are named by hand.
pub fn armhole(kind: BlockKind, part: usize) -> Option<(usize, usize, usize)> {
    match (kind, part) {
        // Back: the shoulder point (I) round to the side seam's top (Z).
        (BlockKind::Bodice, 0) => Some((6, 9, 14)),
        // Front: the side seam's top (Z) round to the shoulder point (X).
        (BlockKind::Bodice, 1) => Some((6, 9, 13)),
        // The crown: from the back underarm over the shoulder point to the front underarm.
        (BlockKind::Sleeve, 0) => Some((10, 4, 14)),
        _ => None,
    }
}

/// A run along a block part's outline: from one anchor forward to another.
pub type Run = (usize, usize);

/// The trouser block's seams that sew front to back, walked in two runs each as The Shapes of
/// Fabric's tutorial checks them ("first walk the inseams from the kneeline to the hemline", then
/// from the kneeline to the crotch point, and the side seams from the kneeline to the hemline and
/// to the waistline): the run's name, then each part's run (from one anchor forward to another).
pub const TROUSER_SEAMS: &[(&str, Run, Run)] = &[
    ("inseam, knee to hem", (3, 4), (9, 10)),
    ("inseam, crotch to knee", (2, 3), (10, 11)),
    ("side seam, knee to hem", (5, 6), (7, 8)),
    ("side seam, waist to knee", (6, 9), (4, 7)),
];

/// How many anchors the trouser block's front and back have as drafted (the front 3 more with a
/// front dart, which comes after the side seam's waist end).
pub const TROUSER_ANCHORS: (usize, usize) = (10, 13);

/// The trouser front's anchors with a front dart.
pub const TROUSER_FRONT_DART_ANCHORS: usize = 13;

/// How a set-in sleeve is to sit, for the ease its crown takes in: the University of Kentucky
/// extension's figures, measured notch to notch (FCS2-807, "Set-in Sleeves",
/// https://publications.ca.uky.edu/sites/publications.ca.uky.edu/files/fcs2807.pdf). Knits take
/// less.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleeveStyle {
    /// The regulation or classic sleeve, the highest cap: 1½ to 2 in.
    Classic,
    /// The shorter cap of a slightly dropped shoulder: ¾ to 1 in.
    Casual,
    /// The flat shirt sleeve: ½ to ¾ in.
    Shirt,
}

impl SleeveStyle {
    /// The ease the crown takes in, least and most, mm.
    pub fn ease(self) -> (f64, f64) {
        // The inches, exactly.
        match self {
            SleeveStyle::Classic => (38.1, 50.8),
            SleeveStyle::Casual => (19.05, 25.4),
            SleeveStyle::Shirt => (12.7, 19.05),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "classic" => SleeveStyle::Classic,
            "casual" => SleeveStyle::Casual,
            "shirt" => SleeveStyle::Shirt,
            _ => return None,
        })
    }
}

impl Part {
    /// How many anchors the outline has: its segment ends, the last left out when it closes on
    /// the start.
    pub fn anchors(&self) -> usize {
        let c = self.corners();
        if c.len() > 1 && c.last() == c.first() { c.len() - 1 } else { c.len() }
    }

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
        let parts = bodice(&mut me().env(), &Options::new()).unwrap();
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
        let p = bodice(&mut me().env(), &Options::new()).unwrap();
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
        let front = bodice(&mut big.env(), &Options::new()).unwrap()[1].corners();
        assert!(((front[12] - front[10]).len() - 82.0).abs() < 1e-9);
    }

    #[test]
    fn the_skirt_block_is_half_the_hip_with_ease_and_takes_the_waist_in() {
        let mut m = Measurements::default();
        for (k, v) in [("waist", 720.0), ("hip", 960.0), ("waist_to_hip", 200.0), ("skirt_length", 580.0)] {
            m.set_value(k, Some(v)).unwrap();
        }
        let p = skirt(&mut m.env(), &Options::new()).unwrap();
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
        assert!(matches!(skirt(&mut m2.env(), &Options::new()), Err(BlockError::Impossible(_))));
        m2.set_value("hip", None).unwrap();
        assert!(matches!(skirt(&mut m2.env(), &Options::new()), Err(BlockError::Missing(_))));
    }

    #[test]
    fn missing_or_impossible_measurements_are_named() {
        let mut m = me();
        m.set_value("neck", None).unwrap();
        m.set_value("shoulder", None).unwrap();
        assert_eq!(bodice(&mut m.env(), &Options::new()), Err(BlockError::Missing(vec!["shoulder".into(), "neck".into()])));
        // The armhole between back and front is open: the chest leaves room past the back width.
        let p = bodice(&mut me().env(), &Options::new()).unwrap();
        let side = p[0].corners()[9];
        assert!(side.x > 345.0 / 2.0 + 5.0 + 40.0, "{side:?}");
        let mut m = me();
        m.set_value("nape_to_waist", Some(100.0)).unwrap();
        assert!(matches!(bodice(&mut m.env(), &Options::new()), Err(BlockError::Impossible(_))));
        let mut m = me();
        m.set_value("shoulder", Some(5.0)).unwrap();
        assert!(matches!(bodice(&mut m.env(), &Options::new()), Err(BlockError::Impossible(_))));
        // A formula can stand for a measurement.
        let mut m = me();
        m.set_formula("waist", Some("bust - 200")).unwrap();
        assert!(bodice(&mut m.env(), &Options::new()).is_ok());
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
        let p = sleeve(&mut m.env(), &Options::new()).unwrap();
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
        let fitted = sleeve(&mut m.env(), &Options::new()).unwrap();
        assert!((sleeve_crown(&fitted[0]) - (table + 30.0)).abs() < 1e-6);
        // A deeper crown, the width kept.
        let c = fitted[0].corners();
        assert!(c[4].x == 215.0 && c[10] == Pt::new(-215.0, c[4].y) && c[4].y > 150.0, "{:?} {:?}", c[4], c[10]);
        assert!(bounds(&fitted[0]).3 == b);
        m.set_formula("sleeve_crown", Some("armhole + 30")).unwrap();
        assert_eq!(sleeve(&mut m.env(), &Options::new()), Err(BlockError::Missing(vec!["sleeve_crown".into()])));
        m.set_formula("sleeve_crown", Some("2000")).unwrap();
        assert!(matches!(sleeve(&mut m.env(), &Options::new()), Err(BlockError::Impossible(_))));
        m.set_formula("sleeve_crown", None).unwrap();
        m.set_value("sleeve_length", Some(180.0)).unwrap();
        assert!(matches!(sleeve(&mut m.env(), &Options::new()), Err(BlockError::Impossible(_))));
        m.set_value("top_arm", None).unwrap();
        assert!(matches!(sleeve(&mut m.env(), &Options::new()), Err(BlockError::Missing(_))));
    }

    /// The men's shirt block (PF-4.13) by the source's steps: a shirt's ease, the necks, the
    /// shoulders, the armhole near its corners, straight sides.
    #[test]
    fn the_mens_shirt_block_follows_the_sources_steps() {
        let mut m = Measurements::default();
        for (k, v) in [("bust", 1000.0), ("neck", 400.0), ("back_width", 400.0), ("armscye_depth", 240.0), ("nape_to_waist", 450.0)] {
            m.set_value(k, Some(v)).unwrap();
        }
        let p = mens_shirt(&mut m.env(), &Options::new()).unwrap();
        assert_eq!((p[0].name, p[1].name), ("Back Men's Shirt Block", "Front Men's Shirt Block"));
        let (b, f) = (p[0].corners(), p[1].corners());
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        // The back: its neck 7.5 cm across and 2 cm up, the shoulder point 3.5 cm out from the back's
        // width (half the half back and 2.5 cm: 22.5 cm), the side seam half way to a third of the
        // chest and 1.5 cm less 5 mm, the hem 79 cm down.
        assert!(near(b[1].x, 75.0) && near(b[1].y, -20.0));
        assert!(near(b[2].x, 260.0) && near(b[3].x, 225.0) && near(b[3].y, 130.0));
        let side = 1000.0 / 3.0 + 15.0 - ((1000.0 / 3.0 + 15.0 - 225.0) / 2.0 - 5.0);
        assert!(near(b[4].x, side) && near(b[4].y, 265.0) && near(b[5].x, side) && near(b[5].y, 790.0));
        // Twice the back and front across the armhole line: the chest and 16 cm.
        let front_width = (f[1].x - 60.0) - side;
        assert!(near(2.0 * (side + front_width), 1160.0));
        // The front neck a fifth of the neck deep and a fifth less 1 cm across; its shoulder 5 mm
        // longer than the back's, 2 cm lower.
        assert!(near(f[1].y, 80.0) && near(f[1].x - f[0].x, 70.0) && near(f[0].y, 0.0));
        let (back_sh, front_sh) = ((b[2] - b[1]).len(), (f[6] - f[0]).len());
        assert!(near(front_sh - back_sh, 5.0) && near(f[6].y, b[2].y + 20.0));
        // The armhole: the back's within 3 cm of its corner I, the front's within 1.75 cm of K.
        let dist = |part: &Part, k: usize, q: Pt| {
            let c = part.corners();
            let Seg::Cubic(c1, c2, e) = part.segs[k] else { panic!("a curve") };
            let a = c[k];
            (0..=100)
                .map(|i| {
                    let t = i as f64 / 100.0;
                    let u = 1.0 - t;
                    let pt = a * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + e * (t * t * t);
                    (pt - q).len()
                })
                .fold(f64::INFINITY, f64::min)
        };
        assert!(dist(&p[0], 3, Pt::new(225.0, 265.0)) < 30.0);
        assert!(dist(&p[1], 4, Pt::new(1000.0 / 3.0 + 15.0 + 60.0, 265.0)) < 17.5);
        // A shirt no longer than the nape to waist is refused.
        let mut o = Options::new();
        o.insert("length".into(), 400.0);
        assert!(matches!(mens_shirt(&mut m.env(), &o), Err(BlockError::Impossible(_))));
    }

    /// The men's trouser block (PF-4.12) by the source's numbers, its example measurements (seat
    /// 102, waist 82, body rise 28, inside leg 78, bottom width 25, waistband 4 cm).
    #[test]
    fn the_mens_trouser_block_follows_the_sources_steps() {
        let mut m = Measurements::default();
        for (k, v) in [("waist", 820.0), ("hip", 1020.0), ("crotch_depth", 280.0), ("waist_to_ankle", 1060.0), ("trouser_hem", 500.0)] {
            m.set_value(k, Some(v)).unwrap();
        }
        let p = mens_trousers(&mut m.env(), &Options::new()).unwrap();
        assert_eq!((p[0].name, p[1].name), ("Front Men's Trouser Block", "Back Men's Trouser Block"));
        let (f, b) = (p[0].corners(), p[1].corners());
        // The front: the crotch line 25 cm down (28 + 1 − 4), the fork 10 + 6.875 cm from the crease,
        // the seat line 7 cm above, a quarter seat and 2 cm (27.5 cm) across it from 6, the hem
        // 78 cm below the crotch line, 12.5 cm each side of the crease.
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(near(f[2].y, 250.0) && near(f[2].x, -(102.0 / 12.0 * 10.0 + 15.0) - (1020.0 / 16.0 + 5.0)), "{:?}", f[2]);
        assert!(near(f[1].y, 180.0) && near(f[7].x - f[1].x, 275.0));
        assert!(near(f[4].y, 1030.0) && near(f[4].x, -125.0) && near(f[5].x, 125.0));
        // Its waist: from 1 cm in from the centre line, a quarter waist and 2.5 cm (23 cm).
        assert!(near(f[8].x - f[0].x, 230.0) && near(f[0].y, 0.0));
        // The back: the hem 2 cm wider each side, a quarter seat and 3 cm across the seat line, its
        // centre back 1 cm above the top line, the dart 2.5 cm wide and 12 cm long.
        assert!(near((b[7].x - b[8].x).abs(), 290.0));
        assert!(near((b[5].x - b[11].x).abs(), 285.0) && near(b[5].y, b[11].y));
        assert!(near(b[0].y, -10.0));
        assert!(near((b[1] - b[3]).len(), 25.0) && near((b[2] - (b[1] + b[3]) * 0.5).len(), 120.0));
        // Turned over beside the front: its fork right of its side seam, all of it right of the front.
        assert!(b[10].x > b[5].x && b.iter().map(|q| q.x).fold(f64::INFINITY, f64::min) > f.iter().map(|q| q.x).fold(f64::NEG_INFINITY, f64::max));
        // The outlines run the same way round, as the women's block's do.
        let area = |c: &[Pt]| (0..c.len()).map(|i| c[i].x * c[(i + 1) % c.len()].y - c[(i + 1) % c.len()].x * c[i].y).sum::<f64>();
        assert_eq!(area(&f).signum(), area(&b).signum());
        // Refused: a crotch depth no deeper than the waistband; missing measurements named.
        m.set_value("crotch_depth", Some(30.0)).unwrap();
        assert!(matches!(mens_trousers(&mut m.env(), &Options::new()), Err(BlockError::Impossible(_))));
        m.set_value("trouser_hem", None).unwrap();
        assert_eq!(mens_trousers(&mut m.env(), &Options::new()), Err(BlockError::Missing(vec!["trouser_hem".into()])));
    }

    #[test]
    fn the_trouser_block_is_half_the_hip_with_ease_and_its_waists_make_the_waist() {
        // The source's example: half hip 48, half waist 37, waist to hip 20, crotch depth 26,
        // waist to knee 60, waist to ankle 104, hem 46 (cm).
        let mut m = Measurements::default();
        for (k, v) in [
            ("waist", 740.0),
            ("hip", 960.0),
            ("waist_to_hip", 200.0),
            ("crotch_depth", 260.0),
            ("waist_to_knee", 600.0),
            ("waist_to_ankle", 1040.0),
            ("trouser_hem", 460.0),
        ] {
            m.set_value(k, Some(v)).unwrap();
        }
        let p = trousers(&mut m.env(), &Options::new()).unwrap();
        let (f, b) = (p[0].corners(), p[1].corners());
        // The forks: 6 cm out from the centre front, 9 cm from the centre back, on the crotch
        // line 27.5 cm down.
        assert_eq!((f[2], b[11]), (Pt::new(-60.0, 275.0), Pt::new(590.0, 275.0)));
        // The front's crease line is 9.5 cm in; its hem 22 cm and the back's 24 cm.
        assert_eq!((f[4], f[5]), (Pt::new(-15.0, 1040.0), Pt::new(205.0, 1040.0)));
        assert_eq!((b[8].x - b[9].x).abs(), 240.0);
        // The waists, centre line to side, are 20 cm and 19 cm: with the 2 cm dart closed, half
        // the waist.
        let (cf_top, fs_top) = (f[0], f[9]);
        assert!(((fs_top - cf_top).len() - 200.0).abs() < 1e-9 && fs_top.y == 0.0);
        let (cb_top, bs_top) = (b[0], b[4]);
        assert!(((bs_top - cb_top).len() - 190.0).abs() < 1e-9 && bs_top.y == 0.0);
        assert!(((b[1] - b[3]).len() - 20.0).abs() < 1e-9 && ((b[2] - (b[1] + b[3]) * 0.5).len() - 100.0).abs() < 1e-9);
        // The back's centre is 2.5 cm above the frame, and the back's knee 1 cm wider each side.
        assert!(cb_top.y < -24.0);
        assert!((((b[10].x - b[7].x) - (f[3].x - f[6].x).abs()) - 20.0).abs() < 1e-9);
        // Both sides meet the side line at the hip and crotch lines.
        assert!(f.contains(&Pt::new(250.0, 200.0)) && b.contains(&Pt::new(250.0, 275.0)));
        m.set_value("waist_to_knee", Some(250.0)).unwrap();
        assert!(matches!(trousers(&mut m.env(), &Options::new()), Err(BlockError::Impossible(_))));
        m.set_value("crotch_depth", None).unwrap();
        assert_eq!(trousers(&mut m.env(), &Options::new()), Err(BlockError::Missing(vec!["crotch_depth".into()])));
    }

    #[test]
    fn the_armholes_and_the_crown_are_where_armhole_says() {
        let p = bodice(&mut me().env(), &Options::new()).unwrap();
        let (back, front) = (p[0].corners(), p[1].corners());
        let ((bf, bt, bn), (ff, ft, fn_)) = (armhole(BlockKind::Bodice, 0).unwrap(), armhole(BlockKind::Bodice, 1).unwrap());
        assert_eq!((p[0].anchors(), p[1].anchors()), (bn, fn_));
        // The back runs from its shoulder point to the side seam's top, where the front's starts.
        assert!(((back[bf] - back[2]).len() - 132.0).abs() < 1e-9, "the back shoulder point");
        assert_eq!(back[bt], front[ff]);
        assert!(((front[ft] - front[10]).len() - 122.0).abs() < 1e-9, "the front shoulder point");
        // The sleeve's crown, from underarm to underarm, is what sleeve_crown measures.
        let mut m = me();
        m.set_value("top_arm", Some(280.0)).unwrap();
        m.set_value("sleeve_length", Some(590.0)).unwrap();
        let sl = &sleeve(&mut m.env(), &Options::new()).unwrap()[0];
        let (from, to, n) = armhole(BlockKind::Sleeve, 0).unwrap();
        assert_eq!(sl.anchors(), n);
        let c = sl.corners();
        assert!((c[from].y - c[to].y).abs() < 1e-9 && (c[from].x + c[to].x).abs() < 1e-9 && c[from].x < 0.0, "the underarms");
        let mut at = sl.start;
        let mut crown = 0.0;
        for (i, seg) in sl.segs.iter().enumerate() {
            let end = match *seg {
                Seg::Line(e) | Seg::Cubic(_, _, e) => e,
            };
            if i >= from || i < to {
                crown += match *seg {
                    Seg::Line(e) => (e - at).len(),
                    Seg::Cubic(c1, c2, e) => cubic_len(at, c1, c2, e),
                };
            }
            at = end;
        }
        assert!((crown - sleeve_crown(sl)).abs() < 1e-9);
        assert_eq!(SleeveStyle::parse("classic").map(SleeveStyle::ease), Some((38.1, 50.8)));
        assert_eq!(SleeveStyle::parse("dolman"), None);
    }

    #[test]
    fn options_change_the_ease_and_the_darts_and_bad_ones_are_refused() {
        let opts = |pairs: &[(&str, f64)]| pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect::<Options>();
        // The bodice: 2 cm more ease puts the centre front 2 cm further out; a wider bust dart.
        let plain = bodice(&mut me().env(), &Options::new()).unwrap();
        let roomy = draft(BlockKind::Bodice, &mut me().env(), &opts(&[("ease", 70.0), ("bust_dart", 90.0)])).unwrap();
        assert!((bounds(&roomy[1]).2 - bounds(&plain[1]).2 - 20.0).abs() < 1e-9);
        assert_ne!(roomy[1], plain[1]);
        // The skirt: 2 cm more ease at the half hip is 2 cm wider.
        let mut m = Measurements::default();
        for (k, v) in [("waist", 720.0), ("hip", 960.0), ("waist_to_hip", 200.0), ("skirt_length", 580.0)] {
            m.set_value(k, Some(v)).unwrap();
        }
        let a = skirt(&mut m.env(), &Options::new()).unwrap();
        let b = draft(BlockKind::Skirt, &mut m.env(), &opts(&[("ease", 30.0)])).unwrap();
        assert!((bounds(&b[1]).2 - bounds(&a[1]).2 - 20.0).abs() < 1e-9);
        // The sleeve: 3 cm ease round the top arm is 2 cm narrower than the source's 5.
        let mut m = Measurements::default();
        for (k, v) in [("top_arm", 380.0), ("sleeve_length", 640.0)] {
            m.set_value(k, Some(v)).unwrap();
        }
        let a = sleeve(&mut m.env(), &Options::new()).unwrap();
        let b = draft(BlockKind::Sleeve, &mut m.env(), &opts(&[("ease", 30.0)])).unwrap();
        let w = |p: &Part| bounds(p).2 - bounds(p).0;
        assert!((w(&a[0]) - w(&b[0]) - 20.0).abs() < 1e-9, "{} {}", w(&a[0]), w(&b[0]));
        // Unknown options and values out of range are refused.
        assert!(matches!(draft(BlockKind::Sleeve, &mut m.env(), &opts(&[("bust_dart", 10.0)])), Err(BlockError::Option(_))));
        assert!(matches!(draft(BlockKind::Sleeve, &mut m.env(), &opts(&[("ease", -1.0)])), Err(BlockError::Option(_))));
        assert!(options(BlockKind::Trousers).iter().any(|o| o.name == "back_dart" && o.default == Some(20.0)));
    }

    #[test]
    fn the_trouser_seams_run_between_the_knee_hem_crotch_and_waist_points() {
        let mut m = Measurements::default();
        for (k, v) in [
            ("waist", 740.0),
            ("hip", 960.0),
            ("waist_to_hip", 200.0),
            ("crotch_depth", 260.0),
            ("waist_to_knee", 600.0),
            ("waist_to_ankle", 1040.0),
            ("trouser_hem", 460.0),
        ] {
            m.set_value(k, Some(v)).unwrap();
        }
        let p = trousers(&mut m.env(), &Options::new()).unwrap();
        // The outlines come back to their starts: the last corner is the first.
        let (front, back) = (p[0].corners(), p[1].corners());
        assert_eq!((front.len() - 1, back.len() - 1), TROUSER_ANCHORS);
        let y = |pts: &[Pt], i: usize| pts[i].y;
        // Knee 600, hem 1040, crotch line 275, waist line about 0.
        for (_, (f0, f1), (b0, b1)) in TROUSER_SEAMS {
            for (pts, a, b) in [(&front, *f0, *f1), (&back, *b0, *b1)] {
                let ends = [y(pts, a).round(), y(pts, b).round()];
                assert!(ends.contains(&600.0), "{ends:?}: every run starts or ends at the knee");
                assert!(ends.iter().any(|e| [1040.0, 275.0].contains(e) || *e < 20.0), "{ends:?}");
            }
        }
    }

    #[test]
    fn a_front_dart_widens_the_front_waist_by_its_width_and_takes_it_in_again() {
        let mut m = Measurements::default();
        for (k, v) in [
            ("waist", 740.0),
            ("hip", 960.0),
            ("waist_to_hip", 200.0),
            ("crotch_depth", 260.0),
            ("waist_to_knee", 600.0),
            ("waist_to_ankle", 1040.0),
            ("trouser_hem", 460.0),
        ] {
            m.set_value(k, Some(v)).unwrap();
        }
        let opts = [("front_dart".to_string(), 10.0)].into_iter().collect::<Options>();
        let plain = trousers(&mut m.env(), &Options::new()).unwrap();
        let darted = trousers(&mut m.env(), &opts).unwrap();
        let (a, b) = (plain[0].corners(), darted[0].corners());
        assert_eq!(b.len() - 1, TROUSER_FRONT_DART_ANCHORS);
        // The side seam's waist end (anchor 9) moves out along the waist; the seams before it stay.
        assert_eq!(a[..9], b[..9]);
        let dist = |p: Pt, q: Pt| (p - q).len();
        assert!((dist(b[9], b[0]) - dist(a[9], a[0]) - 10.0).abs() < 0.5, "{} {}", dist(b[9], b[0]), dist(a[9], a[0]));
        // The dart: legs 1 cm apart at the waist, its point 8 cm below their middle.
        assert!((dist(b[10], b[12]) - 10.0).abs() < 1e-6);
        assert!((dist(b[11], (b[10] + b[12]) * 0.5) - 80.0).abs() < 1e-6);
        assert_eq!(plain[1], darted[1], "the back is as it was");
    }

    #[test]
    fn with_a_bodice_the_sleeve_takes_in_classic_ease_over_its_armhole() {
        let mut m = me();
        m.set_value("top_arm", Some(280.0)).unwrap();
        m.set_value("sleeve_length", Some(590.0)).unwrap();
        let armhole = bodice_armhole(&mut m.env(), &Options::new()).unwrap();
        assert!(armhole > 300.0 && armhole < 600.0, "{armhole}");
        // From the table alone the crown is about the armhole: no ease.
        let table = sleeve_crown(&sleeve(&mut m.env(), &Options::new()).unwrap()[0]);
        assert!((table - armhole).abs() < 20.0, "{table} {armhole}");
        // Set into that armhole: the armhole and 44.45 mm (1¾ in); the option changes it.
        let set = sleeve_crown(&sleeve_for(&mut m.env(), &Options::new(), Some(armhole)).unwrap()[0]);
        assert!((set - armhole - CLASSIC_CROWN_EASE).abs() < 1e-3, "{set} {armhole}");
        let o = [("crown_ease".to_string(), 30.0)].into_iter().collect::<Options>();
        let less = sleeve_crown(&sleeve_for(&mut m.env(), &o, Some(armhole)).unwrap()[0]);
        assert!((less - armhole - 30.0).abs() < 1e-3);
        // A sleeve_crown measurement still wins.
        m.set_formula("sleeve_crown", Some("500")).unwrap();
        let measured = sleeve_crown(&sleeve_for(&mut m.env(), &Options::new(), Some(armhole)).unwrap()[0]);
        assert!((measured - 500.0).abs() < 1e-3);
    }
}
