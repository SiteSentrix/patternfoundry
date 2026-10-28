//! Fabric photos made ready to repeat (PF-10.17): a photo taken at home is lit unevenly (brighter
//! nearer the window or the lamp) and its edges don't meet when it repeats. Evening the light
//! divides each pixel's brightness by the photo's brightness round it, so only the light's slow
//! change across the photo goes, not the print; making it seamless cross-fades the photo with
//! itself moved half its width and height, so each edge meets the next copy's other edge through
//! the photo's own middle. Both are the usual ways of preparing a texture from a photo: flat-field
//! correction (<https://en.wikipedia.org/wiki/Flat-field_correction>) and the offset-and-blend
//! repeat (<https://en.wikipedia.org/wiki/Texture_synthesis>, the tiling it is for).

use crate::view3d::Swatch;

/// A pixel's brightness (Rec. 601 luma, 0 to 255).
fn luma(p: &[u8]) -> f64 {
    0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64
}

impl Swatch {
    fn pixels_ok(&self) -> bool {
        self.width > 0 && self.height > 0 && self.rgb.len() == self.width as usize * self.height as usize * 3
    }

    /// Even out the light across the photo: each pixel's colour scaled by the photo's mean
    /// brightness over its brightness averaged across a quarter of the photo round the pixel (a
    /// box, the photo's edge pixels repeated beyond it), at most 4 times brighter or darker.
    pub fn even_lighting(&mut self) {
        if !self.pixels_ok() {
            return;
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let l: Vec<f64> = self.rgb.chunks(3).map(luma).collect();
        // A summed-area table, so each box's mean is four lookups.
        let mut sum = vec![0.0; (w + 1) * (h + 1)];
        for y in 0..h {
            for x in 0..w {
                sum[(y + 1) * (w + 1) + x + 1] = l[y * w + x] + sum[y * (w + 1) + x + 1] + sum[(y + 1) * (w + 1) + x] - sum[y * (w + 1) + x];
            }
        }
        let mean = l.iter().sum::<f64>() / l.len() as f64;
        let r = (w.min(h) / 4).max(1);
        for y in 0..h {
            let (y0, y1) = (y.saturating_sub(r), (y + r + 1).min(h));
            for x in 0..w {
                let (x0, x1) = (x.saturating_sub(r), (x + r + 1).min(w));
                let total = sum[y1 * (w + 1) + x1] - sum[y0 * (w + 1) + x1] - sum[y1 * (w + 1) + x0] + sum[y0 * (w + 1) + x0];
                let around = total / ((x1 - x0) * (y1 - y0)) as f64;
                let k = (mean / around.max(1.0)).clamp(0.25, 4.0);
                for c in &mut self.rgb[(y * w + x) * 3..(y * w + x) * 3 + 3] {
                    *c = (*c as f64 * k).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }

    /// Make the photo repeat without a seam: each pixel the photo's own, cross-faded toward the
    /// edges into the photo moved half its width and height (whose edges are the photo's middle),
    /// by how near the pixel is to the middle across and down.
    pub fn make_seamless(&mut self) {
        if !self.pixels_ok() || self.width < 2 || self.height < 2 {
            return;
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let old = self.rgb.clone();
        // 1 in the middle, 0 at the edges.
        let tent = |i: usize, n: usize| 1.0 - (2.0 * (i as f64 + 0.5) / n as f64 - 1.0).abs();
        for y in 0..h {
            for x in 0..w {
                let a = tent(x, w) * tent(y, h);
                let (mx, my) = ((x + w / 2) % w, (y + h / 2) % h);
                for c in 0..3 {
                    let own = old[(y * w + x) * 3 + c] as f64;
                    let moved = old[(my * w + mx) * 3 + c] as f64;
                    self.rgb[(y * w + x) * 3 + c] = (a * own + (1.0 - a) * moved).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(w: u32, h: u32, f: impl Fn(u32, u32) -> f64) -> Swatch {
        let rgb =
            (0..h).flat_map(|y| (0..w).flat_map(move |x| [0, 1, 2].map(|_| (x, y)))).map(|(x, y)| f(x, y).round().clamp(0.0, 255.0) as u8).collect();
        Swatch { width: w, height: h, rgb, mm: 1.0 }
    }

    fn column_means(s: &Swatch) -> Vec<f64> {
        (0..s.width as usize)
            .map(|x| (0..s.height as usize).map(|y| s.rgb[(y * s.width as usize + x) * 3] as f64).sum::<f64>() / s.height as f64)
            .collect()
    }

    /// A small check print (8 px squares, 100 and 160) lit from the left, twice as bright there as
    /// at the right: evened, the light's change across goes and the print stays.
    #[test]
    fn evening_the_light_takes_out_a_gradient_and_keeps_the_print() {
        let print = |x: u32, y: u32| if (x / 8 + y / 8).is_multiple_of(2) { 100.0 } else { 160.0 };
        let light = |x: u32| 1.33 - 0.66 * x as f64 / 199.0;
        let mut s = grey(200, 120, |x, y| print(x, y) * light(x));
        let spread = |s: &Swatch| {
            // Each 16-px stretch's mean (a light square and a dark one): the light, not the print.
            let m = column_means(s);
            let blocks: Vec<f64> = m.chunks(16).filter(|c| c.len() == 16).map(|c| c.iter().sum::<f64>() / 16.0).collect();
            blocks.iter().copied().fold(f64::NEG_INFINITY, f64::max) - blocks.iter().copied().fold(f64::INFINITY, f64::min)
        };
        let before = spread(&s);
        s.even_lighting();
        let after = spread(&s);
        assert!(before > 60.0 && after < before / 3.0, "{before} → {after}");
        // The print still shows: a light square stays lighter than the dark one beside it.
        let at = |s: &Swatch, x: usize, y: usize| s.rgb[(y * 200 + x) * 3];
        assert!(at(&s, 100, 4) as i32 - at(&s, 108, 4) as i32 > 30 || at(&s, 108, 4) as i32 - at(&s, 100, 4) as i32 > 30);
    }

    /// A ramp from black to white across: repeated it jumps from white back to black; made
    /// seamless, its right edge runs into its left as gently as it runs inside.
    #[test]
    fn a_seamless_photo_meets_itself_at_its_edges() {
        let mut s = grey(64, 32, |x, _| x as f64 * 4.0);
        let jump = |s: &Swatch, y: usize| (s.rgb[(y * 64 + 63) * 3] as i32 - s.rgb[y * 64 * 3] as i32).abs();
        assert!(jump(&s, 16) > 200);
        s.make_seamless();
        for y in [0, 8, 16, 31] {
            let inside = (1..64).map(|x| (s.rgb[(y * 64 + x) * 3] as i32 - s.rgb[(y * 64 + x - 1) * 3] as i32).abs()).max().unwrap();
            assert!(jump(&s, y) <= inside.max(4), "row {y}: {} at the edge, {inside} inside", jump(&s, y));
        }
        // Down too: the first row meets the last.
        let col = |s: &Swatch, y: usize| s.rgb[(y * 64 + 10) * 3] as i32;
        assert!((col(&s, 0) - col(&s, 31)).abs() <= 8);
    }

    #[test]
    fn a_broken_swatch_is_left_alone() {
        let mut s = Swatch { width: 4, height: 4, rgb: vec![1, 2, 3], mm: 1.0 };
        s.even_lighting();
        s.make_seamless();
        assert_eq!(s.rgb, vec![1, 2, 3]);
    }
}
