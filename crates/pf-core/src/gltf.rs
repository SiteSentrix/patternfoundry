//! glTF export (PF-10.5): the avatar and the draped garment as one binary glTF file (`.glb`), the
//! open 3D format game engines and 3D software read (Blender, Unity, Unreal, three.js, Windows 3D
//! Viewer, macOS Quick Look), written from the Khronos Group's published specification, glTF 2.0
//! (https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html):
//!
//! - the file is a 12-byte header (magic `glTF`, version 2, the whole length), a JSON chunk padded
//!   with spaces and a BIN chunk padded with zeros, each a multiple of four bytes (§4.4 GLB);
//! - glTF is in metres, y up, right-handed, the front of an asset facing +z (§3.4): our mm with
//!   z up and the front toward +y become `(x, z, y) / 1000`, a mirror, so each triangle is turned
//!   back to stay counter-clockwise seen from outside (§3.7.2.1, front faces counter-clockwise);
//! - each part is a mesh of one triangle primitive with POSITION (with its min and max, which
//!   the specification requires), NORMAL (unit length), COLOR_0 where it has colours (unsigned
//!   bytes, normalized, four to a vertex so each is four-byte aligned, §3.6.2.4) and UNSIGNED_INT
//!   indices, and a metallic-roughness material: not metallic, rough as cloth and skin are;
//! - a part printed with a fabric photo (PF-10.18) has TEXCOORD_0 (floats, §3.7.2.1; (0, 0) the
//!   image's top-left, u across and v down, §3.8.3) and its material a base colour texture
//!   (§3.9.2), the photo as a PNG in a buffer view of its own (no target, §3.6.1.2, §3.8.4), its
//!   sampler repeating both ways (REPEAT, 10497, §3.8.5), so the photo repeats across the
//!   cloth as the fabric does.

use serde_json::{Value, json};

/// One thing to export: points in mm (z up, the front toward +y) and triangles of them.
pub struct Part<'a> {
    /// The node's name in the file.
    pub name: &'a str,
    pub vertices: &'a [[f64; 3]],
    pub triangles: &'a [[u32; 3]],
    /// A colour for each vertex (RGB), multiplied by `base`; none: `base` alone.
    pub colours: Option<&'a [[u8; 3]]>,
    /// The material's base colour, RGBA 0 to 1 (linear, as glTF has it).
    pub base: [f64; 4],
    /// Seen from both sides (cloth has two); a closed body only from outside.
    pub double_sided: bool,
    /// Printed with a picture (PF-10.18).
    pub texture: Option<Texture<'a>>,
}

/// A picture printed on a part: each vertex's place on it (in pictures across and down, the
/// picture repeating beyond 0 to 1), and the picture as PNG (sRGB, as base colour textures are).
#[derive(Clone, Copy, Debug)]
pub struct Texture<'a> {
    pub uv: &'a [[f64; 2]],
    pub png: &'a [u8],
}

/// Why a part can't be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GltfError {
    /// A part has no triangles, or a triangle names a vertex it hasn't.
    Mesh(String),
    /// A part's colours aren't one to a vertex.
    Colours(String),
    /// A point isn't a finite number.
    NotFinite(String),
    /// A part's texture coordinates aren't one to a vertex, or aren't finite, or it has no picture.
    Texture(String),
}

impl std::fmt::Display for GltfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GltfError::Mesh(n) => write!(f, "{n} has no triangles or a triangle names a missing point"),
            GltfError::Colours(n) => write!(f, "{n}'s colours are not one to a point"),
            GltfError::NotFinite(n) => write!(f, "{n} has a point that is not a number"),
            GltfError::Texture(n) => write!(f, "{n}'s texture has no picture or its places are not one to a point"),
        }
    }
}

impl std::error::Error for GltfError {}

/// glTF's component types and buffer-view targets (§3.6.2.2, §3.6.1).
const FLOAT: u32 = 5126;
const UNSIGNED_BYTE: u32 = 5121;
const UNSIGNED_INT: u32 = 5125;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
/// A sampler's filters and wrapping (§3.8.5).
const LINEAR: u32 = 9729;
const LINEAR_MIPMAP_LINEAR: u32 = 9987;
const REPEAT: u32 = 10497;
/// The GLB magic, version and chunk types (§4.4.3, §4.4.4).
const MAGIC: u32 = 0x4654_6C67;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;

/// An sRGB channel (0 to 255, as colours are picked) as glTF's linear (0 to 1): glTF's colours,
/// the material's and the vertices', are linear (§3.9.2, §3.7.2.1).
pub fn linear(c: u8) -> f64 {
    let s = c as f64 / 255.0;
    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
}

/// An sRGB colour as linear bytes, for COLOR_0.
pub fn linear_rgb(c: [u8; 3]) -> [u8; 3] {
    c.map(|v| (linear(v) * 255.0).round() as u8)
}

/// A point in glTF's space: metres, y up, the front toward +z.
fn to_gltf(p: [f64; 3]) -> [f32; 3] {
    [(p[0] / 1000.0) as f32, (p[2] / 1000.0) as f32, (p[1] / 1000.0) as f32]
}

/// Each vertex's normal: the sum of its triangles' (as wound, area-weighted), made unit length;
/// a vertex of no triangle (or only flat ones) gets up.
fn normals(points: &[[f32; 3]], triangles: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut n = vec![[0.0f64; 3]; points.len()];
    for t in triangles {
        let [a, b, c] = t.map(|i| points[i as usize].map(f64::from));
        let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
        let f = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        for i in t {
            for k in 0..3 {
                n[*i as usize][k] += f[k];
            }
        }
    }
    n.iter()
        .map(|v| {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if len > 1e-18 { [(v[0] / len) as f32, (v[1] / len) as f32, (v[2] / len) as f32] } else { [0.0, 1.0, 0.0] }
        })
        .collect()
}

/// The parts as a binary glTF file, each a node of the one scene.
pub fn to_glb(parts: &[Part]) -> Result<Vec<u8>, GltfError> {
    let mut bin: Vec<u8> = Vec::new();
    let (mut views, mut accessors, mut meshes, mut materials, mut nodes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut images, mut textures) = (Vec::new(), Vec::new());
    // A buffer view of `bytes`, four-byte aligned, for vertex data or indices (`target`) or none
    // (an image) → its index.
    let mut view = |bin: &mut Vec<u8>, bytes: &[u8], target: Option<u32>| -> usize {
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let mut v = json!({"buffer": 0, "byteOffset": bin.len(), "byteLength": bytes.len()});
        if let Some(t) = target {
            v["target"] = json!(t);
        }
        views.push(v);
        bin.extend_from_slice(bytes);
        views.len() - 1
    };
    for part in parts {
        let name = part.name.to_string();
        let n = part.vertices.len();
        if part.triangles.is_empty() || part.triangles.iter().flatten().any(|i| *i as usize >= n) {
            return Err(GltfError::Mesh(name));
        }
        if part.vertices.iter().flatten().any(|v| !v.is_finite()) {
            return Err(GltfError::NotFinite(name));
        }
        if part.colours.is_some_and(|c| c.len() != n) {
            return Err(GltfError::Colours(name));
        }
        if part.texture.is_some_and(|t| t.uv.len() != n || t.png.is_empty() || t.uv.iter().flatten().any(|v| !v.is_finite())) {
            return Err(GltfError::Texture(name));
        }
        let points: Vec<[f32; 3]> = part.vertices.iter().map(|p| to_gltf(*p)).collect();
        // Mirrored by the swap of y and z: each triangle turned back.
        let triangles: Vec<[u32; 3]> = part.triangles.iter().map(|t| [t[0], t[2], t[1]]).collect();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for p in &points {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let floats = |v: &[[f32; 3]]| v.iter().flatten().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
        let pv = view(&mut bin, &floats(&points), Some(ARRAY_BUFFER));
        accessors.push(json!({"bufferView": pv, "componentType": FLOAT, "count": n, "type": "VEC3", "min": lo, "max": hi}));
        let mut attributes = json!({"POSITION": accessors.len() - 1});
        let nv = view(&mut bin, &floats(&normals(&points, &triangles)), Some(ARRAY_BUFFER));
        accessors.push(json!({"bufferView": nv, "componentType": FLOAT, "count": n, "type": "VEC3"}));
        attributes["NORMAL"] = json!(accessors.len() - 1);
        if let Some(colours) = part.colours {
            let bytes: Vec<u8> = colours.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect();
            let cv = view(&mut bin, &bytes, Some(ARRAY_BUFFER));
            accessors.push(json!({"bufferView": cv, "componentType": UNSIGNED_BYTE, "normalized": true, "count": n, "type": "VEC4"}));
            attributes["COLOR_0"] = json!(accessors.len() - 1);
        }
        let mut pbr = json!({"baseColorFactor": part.base, "metallicFactor": 0.0, "roughnessFactor": 0.9});
        if let Some(t) = part.texture {
            let uv: Vec<u8> = t.uv.iter().flatten().flat_map(|f| (*f as f32).to_le_bytes()).collect();
            let tv = view(&mut bin, &uv, Some(ARRAY_BUFFER));
            accessors.push(json!({"bufferView": tv, "componentType": FLOAT, "count": n, "type": "VEC2"}));
            attributes["TEXCOORD_0"] = json!(accessors.len() - 1);
            let image = view(&mut bin, t.png, None);
            images.push(json!({"name": name, "bufferView": image, "mimeType": "image/png"}));
            textures.push(json!({"sampler": 0, "source": images.len() - 1}));
            pbr["baseColorTexture"] = json!({"index": textures.len() - 1});
        }
        let iv = view(&mut bin, &triangles.iter().flatten().flat_map(|i| i.to_le_bytes()).collect::<Vec<u8>>(), Some(ELEMENT_ARRAY_BUFFER));
        accessors.push(json!({"bufferView": iv, "componentType": UNSIGNED_INT, "count": triangles.len() * 3, "type": "SCALAR"}));
        let indices = accessors.len() - 1;
        materials.push(json!({
            "name": name,
            "pbrMetallicRoughness": pbr,
            "doubleSided": part.double_sided,
        }));
        meshes
            .push(json!({"name": name, "primitives": [{"attributes": attributes, "indices": indices, "material": materials.len() - 1, "mode": 4}]}));
        nodes.push(json!({"name": name, "mesh": meshes.len() - 1}));
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let scene: Vec<usize> = (0..nodes.len()).collect();
    let mut doc = json!({
        "asset": {"version": "2.0", "generator": "PatternFoundry"},
        "scene": 0,
        "scenes": [{"nodes": scene}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
    });
    if !bin.is_empty() {
        doc["buffers"] = json!([{"byteLength": bin.len()}]);
    }
    if !images.is_empty() {
        doc["images"] = json!(images);
        doc["textures"] = json!(textures);
        doc["samplers"] = json!([{"magFilter": LINEAR, "minFilter": LINEAR_MIPMAP_LINEAR, "wrapS": REPEAT, "wrapT": REPEAT}]);
    }
    let mut text = serde_json::to_vec(&doc).unwrap_or_default();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + if bin.is_empty() { 0 } else { 8 + bin.len() };
    let mut out = Vec::with_capacity(total);
    for w in [MAGIC, 2, total as u32, text.len() as u32, JSON_CHUNK] {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out.extend_from_slice(&text);
    if !bin.is_empty() {
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(&BIN_CHUNK.to_le_bytes());
        out.extend_from_slice(&bin);
    }
    Ok(out)
}

/// A binary glTF file read back: its JSON and its BIN chunk (for tests and checks).
pub fn read_glb(bytes: &[u8]) -> Option<(Value, Vec<u8>)> {
    let word = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    if word(0)? != MAGIC || word(4)? != 2 || word(8)? as usize != bytes.len() || word(16)? != JSON_CHUNK {
        return None;
    }
    let jl = word(12)? as usize;
    let json: Value = serde_json::from_slice(bytes.get(20..20 + jl)?).ok()?;
    let at = 20 + jl;
    let bin = if at < bytes.len() {
        let bl = word(at)? as usize;
        if word(at + 4)? != BIN_CHUNK {
            return None;
        }
        bytes.get(at + 8..at + 8 + bl)?.to_vec()
    } else {
        Vec::new()
    };
    Some((json, bin))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tetrahedron wound counter-clockwise seen from outside in our space (z up).
    fn tetra() -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let v = vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 100.0]];
        (v, vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]])
    }

    /// The floats of accessor `i` of `(json, bin)`.
    fn floats(json: &Value, bin: &[u8], i: usize) -> Vec<f32> {
        let a = &json["accessors"][i];
        let v = &json["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
        let (off, len) = (v["byteOffset"].as_u64().unwrap() as usize, v["byteLength"].as_u64().unwrap() as usize);
        bin[off..off + len].chunks(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
    }

    fn u32s(json: &Value, bin: &[u8], i: usize) -> Vec<u32> {
        floats(json, bin, i).into_iter().map(f32::to_bits).collect()
    }

    #[test]
    fn a_glb_file_is_laid_out_as_the_specification_says() {
        let (v, t) = tetra();
        let colours = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [9, 9, 9]];
        let glb = to_glb(&[
            Part { name: "Body", vertices: &v, triangles: &t, colours: None, base: [0.8, 0.7, 0.6, 1.0], double_sided: false, texture: None },
            Part { name: "Cloth", vertices: &v, triangles: &t, colours: Some(&colours), base: [1.0; 4], double_sided: true, texture: None },
        ])
        .unwrap();
        assert_eq!(&glb[0..4], b"glTF");
        assert!(glb.len().is_multiple_of(4));
        let (json, bin) = read_glb(&glb).unwrap();
        assert_eq!(json["asset"]["version"], "2.0");
        assert_eq!(json["buffers"][0]["byteLength"].as_u64().unwrap() as usize, bin.len());
        assert_eq!(json["scenes"][0]["nodes"], json!([0, 1]));
        // Every buffer view four-byte aligned and inside the buffer.
        for v in json["bufferViews"].as_array().unwrap() {
            let (o, l) = (v["byteOffset"].as_u64().unwrap() as usize, v["byteLength"].as_u64().unwrap() as usize);
            assert!(o.is_multiple_of(4) && o + l <= bin.len());
        }
        // Metres, y up: the point 100 mm up is at y 0.1; the one 100 mm to the front at z 0.1.
        let prim = &json["meshes"][0]["primitives"][0];
        let pos = floats(&json, &bin, prim["attributes"]["POSITION"].as_u64().unwrap() as usize);
        assert_eq!(&pos[9..12], &[0.0, 0.1, 0.0]);
        assert_eq!(&pos[6..9], &[0.0, 0.0, 0.1]);
        let acc = &json["accessors"][prim["attributes"]["POSITION"].as_u64().unwrap() as usize];
        assert_eq!(acc["max"], json!([0.1f32, 0.1f32, 0.1f32]));
        // Turned back after the mirror: the solid is counter-clockwise seen from outside (a
        // positive volume), its normals pointing out.
        let idx = u32s(&json, &bin, prim["indices"].as_u64().unwrap() as usize);
        let p = |i: u32| [pos[i as usize * 3] as f64, pos[i as usize * 3 + 1] as f64, pos[i as usize * 3 + 2] as f64];
        let vol: f64 = idx
            .chunks(3)
            .map(|t| {
                let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
                a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])
            })
            .sum();
        assert!(vol > 0.0, "{vol}");
        let nor = floats(&json, &bin, prim["attributes"]["NORMAL"].as_u64().unwrap() as usize);
        // The far corner's normal points away from the solid's middle.
        assert!(nor[3] > 0.0 && nor[4] < 0.5, "{nor:?}");
        // The cloth's colours, normalized bytes, four to a vertex.
        let cloth = &json["meshes"][1]["primitives"][0]["attributes"];
        let ca = &json["accessors"][cloth["COLOR_0"].as_u64().unwrap() as usize];
        assert_eq!((ca["componentType"].as_u64(), ca["type"].as_str(), ca["normalized"].as_bool()), (Some(5121), Some("VEC4"), Some(true)));
        assert!(json["meshes"][0]["primitives"][0]["attributes"].get("COLOR_0").is_none());
        assert_eq!(json["materials"][1]["doubleSided"], true);
    }

    #[test]
    fn bad_parts_are_refused() {
        let (v, t) = tetra();
        let part =
            |t: &'static [[u32; 3]]| Part { name: "X", vertices: &v, triangles: t, colours: None, base: [1.0; 4], double_sided: true, texture: None };
        assert!(matches!(to_glb(&[part(&[])]), Err(GltfError::Mesh(_))));
        assert!(matches!(to_glb(&[part(&[[0, 1, 9]])]), Err(GltfError::Mesh(_))));
        let few = [[0u8; 3]; 2];
        assert!(matches!(
            to_glb(&[Part { name: "X", vertices: &v, triangles: &t, colours: Some(&few), base: [1.0; 4], double_sided: true, texture: None }]),
            Err(GltfError::Colours(_))
        ));
        let nan = [[f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        assert!(matches!(
            to_glb(&[Part { name: "X", vertices: &nan, triangles: &[[0, 1, 2]], colours: None, base: [1.0; 4], double_sided: true, texture: None }]),
            Err(GltfError::NotFinite(_))
        ));
        let short = [[0.0, 0.0]; 2];
        let png = [1u8, 2, 3];
        let tex = Texture { uv: &short, png: &png };
        assert!(matches!(
            to_glb(&[Part { name: "X", vertices: &v, triangles: &t, colours: None, base: [1.0; 4], double_sided: true, texture: Some(tex) }]),
            Err(GltfError::Texture(_))
        ));
        assert!(read_glb(b"nope").is_none());
        // sRGB's middle grey is about a fifth in linear light.
        assert!((linear(128) - 0.2158).abs() < 1e-3 && linear(0) == 0.0 && (linear(255) - 1.0).abs() < 1e-12);
    }

    /// A part printed with a picture (PF-10.18): TEXCOORD_0 as floats, one pair to a vertex, the
    /// picture's PNG in a buffer view without a target, a texture through a repeating sampler, the
    /// material's base colour texture; a part without one has none of these.
    #[test]
    fn a_printed_part_carries_its_picture_and_where_it_goes() {
        let (v, t) = tetra();
        let uv = [[0.0, 0.0], [2.5, 0.0], [0.0, 1.5], [-1.0, 3.0]];
        let png = [0x89, b'P', b'N', b'G', 13, 10, 26, 10, 7];
        let glb = to_glb(&[
            Part { name: "Body", vertices: &v, triangles: &t, colours: None, base: [1.0; 4], double_sided: false, texture: None },
            Part {
                name: "Cloth",
                vertices: &v,
                triangles: &t,
                colours: None,
                base: [1.0; 4],
                double_sided: true,
                texture: Some(Texture { uv: &uv, png: &png }),
            },
        ])
        .unwrap();
        let (json, bin) = read_glb(&glb).unwrap();
        assert!(json["meshes"][0]["primitives"][0]["attributes"].get("TEXCOORD_0").is_none());
        assert!(json["materials"][0]["pbrMetallicRoughness"].get("baseColorTexture").is_none());
        let at = json["meshes"][1]["primitives"][0]["attributes"]["TEXCOORD_0"].as_u64().unwrap() as usize;
        let acc = &json["accessors"][at];
        assert_eq!((acc["componentType"].as_u64(), acc["type"].as_str(), acc["count"].as_u64()), (Some(5126), Some("VEC2"), Some(4)));
        assert_eq!(floats(&json, &bin, at), vec![0.0, 0.0, 2.5, 0.0, 0.0, 1.5, -1.0, 3.0]);
        let tex = json["materials"][1]["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64().unwrap() as usize;
        let texture = &json["textures"][tex];
        let sampler = &json["samplers"][texture["sampler"].as_u64().unwrap() as usize];
        assert_eq!((sampler["wrapS"].as_u64(), sampler["wrapT"].as_u64()), (Some(10497), Some(10497)));
        let image = &json["images"][texture["source"].as_u64().unwrap() as usize];
        assert_eq!(image["mimeType"], "image/png");
        let view = &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
        assert!(view.get("target").is_none(), "{view}");
        let (o, l) = (view["byteOffset"].as_u64().unwrap() as usize, view["byteLength"].as_u64().unwrap() as usize);
        assert_eq!(&bin[o..o + l], &png);
    }
}
