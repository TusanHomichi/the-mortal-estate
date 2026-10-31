//! Faceted meshes and the glTF that carries them: every triangle its own
//! three corners with the face's normal, one material per part, the buffer
//! inline. Numbers are written to a fixed precision, so the same model
//! always makes the same file.

use std::f32::consts::TAU;
use std::fmt::Write;

type V = [f32; 3];

/// One material's worth of a model.
pub struct Part {
    name: &'static str,
    colour: V,
    roughness: f32,
    glow: V,
    corners: Vec<V>,
}

impl Part {
    pub fn new(name: &'static str, colour: V, roughness: f32, glow: V) -> Part {
        Part {
            name,
            colour,
            roughness,
            glow,
            corners: Vec::new(),
        }
    }

    /// A face, its corners counter-clockwise as seen from the side it
    /// faces: the side that's drawn and lit.
    fn triangle(&mut self, a: V, b: V, c: V) {
        self.corners.extend([a, b, c]);
    }

    /// An ellipsoid around `centre` with these half-sizes, in `segments`
    /// around and `rings` from pole to pole.
    pub fn ellipsoid(&mut self, centre: V, half: V, segments: u32, rings: u32) {
        let point = |s: u32, r: u32| {
            let around = s as f32 / segments as f32 * TAU;
            let down = r as f32 / rings as f32 * TAU / 2.0;
            [
                centre[0] + half[0] * down.sin() * around.cos(),
                centre[1] + half[1] * down.cos(),
                centre[2] + half[2] * down.sin() * around.sin(),
            ]
        };
        for r in 0..rings {
            for s in 0..segments {
                let (a, b) = (point(s, r), point(s + 1, r));
                let (c, d) = (point(s, r + 1), point(s + 1, r + 1));
                if r > 0 {
                    self.triangle(a, b, c);
                }
                if r + 1 < rings {
                    self.triangle(b, d, c);
                }
            }
        }
    }

    /// A tapering spike from a base of `radius` at `from` to a point at `to`.
    pub fn spike(&mut self, from: V, to: V, radius: f32, sides: u32) {
        let ring = ring(from, to, radius, sides);
        for i in 0..sides as usize {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            self.triangle(a, b, to);
            self.triangle(b, a, from);
        }
    }

    /// A post of `radius` from `from` to `to`, capped at both ends.
    pub fn post(&mut self, from: V, to: V, radius: f32, sides: u32) {
        let low = ring(from, to, radius, sides);
        let high: Vec<V> = low.iter().map(|p| add(*p, sub(to, from))).collect();
        for i in 0..sides as usize {
            let j = (i + 1) % low.len();
            self.triangle(low[i], low[j], high[i]);
            self.triangle(low[j], high[j], high[i]);
            self.triangle(low[i], from, low[j]);
            self.triangle(high[j], to, high[i]);
        }
    }
}

/// Points round the axis from `from` to `to`, at `from`, counter-clockwise
/// as seen from `to`.
fn ring(from: V, to: V, radius: f32, sides: u32) -> Vec<V> {
    let axis = normal(sub(to, from));
    let other = if axis[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = normal(cross(axis, other));
    let v = cross(axis, u);
    (0..sides)
        .map(|i| {
            let a = i as f32 / sides as f32 * TAU;
            add(
                from,
                add(scale(u, a.cos() * radius), scale(v, a.sin() * radius)),
            )
        })
        .collect()
}

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: V, s: f32) -> V {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normal(a: V) -> V {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l == 0.0 {
        [0.0, 1.0, 0.0]
    } else {
        scale(a, 1.0 / l)
    }
}

/// A number as the file writes it: fixed precision, and never `-0`.
fn num(x: f32) -> String {
    let s = format!("{x:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn list(v: V) -> String {
    format!("[{},{},{}]", num(v[0]), num(v[1]), num(v[2]))
}

/// The parts as one glTF mesh, one primitive each.
pub fn write(parts: &[Part]) -> String {
    let mut bytes = Vec::new();
    let (mut views, mut accessors, mut primitives, mut materials) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (i, part) in parts.iter().enumerate() {
        // Rounded as the file writes numbers, so the bounds below hold.
        let corners: Vec<V> = part
            .corners
            .iter()
            .map(|c| c.map(|x| num(x).parse().expect("a number")))
            .collect();
        let normals: Vec<V> = corners
            .chunks(3)
            .flat_map(|t| {
                let n = normal(cross(sub(t[1], t[0]), sub(t[2], t[0])));
                [n, n, n]
            })
            .collect();
        let count = corners.len();
        for (data, target) in [(&corners, "positions"), (&normals, "normals")] {
            let offset = bytes.len();
            for v in data.iter() {
                for x in v {
                    bytes.extend(x.to_le_bytes());
                }
            }
            views.push(format!(
                r#"{{"buffer":0,"byteOffset":{offset},"byteLength":{},"target":34962}}"#,
                bytes.len() - offset
            ));
            let bounds = if target == "positions" {
                let lo = (0..3).map(|k| corners.iter().map(|c| c[k]).fold(f32::MAX, f32::min));
                let hi = (0..3).map(|k| corners.iter().map(|c| c[k]).fold(f32::MIN, f32::max));
                let lo: Vec<f32> = lo.collect();
                let hi: Vec<f32> = hi.collect();
                format!(
                    r#","min":{},"max":{}"#,
                    list([lo[0], lo[1], lo[2]]),
                    list([hi[0], hi[1], hi[2]])
                )
            } else {
                String::new()
            };
            accessors.push(format!(
                r#"{{"bufferView":{},"componentType":5126,"count":{count},"type":"VEC3"{bounds}}}"#,
                views.len() - 1
            ));
        }
        primitives.push(format!(
            r#"{{"attributes":{{"POSITION":{},"NORMAL":{}}},"material":{i}}}"#,
            accessors.len() - 2,
            accessors.len() - 1
        ));
        materials.push(format!(
            r#"{{"name":"{}","pbrMetallicRoughness":{{"baseColorFactor":[{},{},{},1],"metallicFactor":0,"roughnessFactor":{}}},"emissiveFactor":{}}}"#,
            part.name,
            num(part.colour[0]),
            num(part.colour[1]),
            num(part.colour[2]),
            num(part.roughness),
            list(part.glow)
        ));
    }
    let mut out = String::new();
    write!(
        out,
        r#"{{"asset":{{"version":"2.0","generator":"tme-models"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{}]}}],"materials":[{}],"accessors":[{}],"bufferViews":[{}],"buffers":[{{"byteLength":{},"uri":"data:application/octet-stream;base64,{}"}}]}}"#,
        primitives.join(","),
        materials.join(","),
        accessors.join(","),
        views.join(","),
        bytes.len(),
        base64(&bytes)
    )
    .expect("writing to a string");
    out.push('\n');
    out
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether every face of `part`, a solid that bulges everywhere, faces
    /// away from its middle.
    fn faces_out(part: &Part) -> bool {
        let n = part.corners.len() as f32;
        let middle = part
            .corners
            .iter()
            .fold([0.0; 3], |m, c| add(m, scale(*c, 1.0 / n)));
        part.corners.chunks(3).all(|t| {
            let facing = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let centre = scale(add(add(t[0], t[1]), t[2]), 1.0 / 3.0);
            let away = sub(centre, middle);
            facing[0] * away[0] + facing[1] * away[1] + facing[2] * away[2] > 0.0
        })
    }

    #[test]
    fn every_solid_faces_out() {
        let part = || Part::new("test", [1.0; 3], 1.0, [0.0; 3]);
        let mut egg = part();
        egg.ellipsoid([0.0, 0.5, 0.0], [0.4, 0.3, 0.6], 9, 5);
        assert!(faces_out(&egg), "an ellipsoid");
        // Up, and leaning over: the ring's axis is found two ways.
        for to in [[0.1, 1.0, 0.0], [1.0, 0.2, -0.3]] {
            let mut spike = part();
            spike.spike([0.0; 3], to, 0.1, 4);
            assert!(faces_out(&spike), "a spike to {to:?}");
            let mut post = part();
            post.post([0.0; 3], to, 0.1, 5);
            assert!(faces_out(&post), "a post to {to:?}");
        }
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
