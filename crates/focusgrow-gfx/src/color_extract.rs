use crate::gif_decoder::DecodedFrame;

/// Extracts a vivid accent `#RRGGBB` hex color from a decoded album cover frame.
///
/// Mirrors FocusGrow's JavaScript album color extractor:
/// - Samples up to a 32x32 grid, ignoring near-black / near-white pixels.
/// - Returns `#F1F5F9` for monochrome/desaturated covers.
/// - Optionally applies a 180-degree complementary hue shift when `use_complementary` is true.
/// - Falls back to `#80D8FF` if the resulting color luminance is too dark (`< 0.12`).
pub fn extract_album_accent(frame: &DecodedFrame, use_complementary: bool) -> String {
    if frame.width == 0 || frame.height == 0 || frame.pixels_pbgra.len() < 4 {
        return "#F1F5F9".to_string();
    }

    let step_x = (frame.width / 32).max(1);
    let step_y = (frame.height / 32).max(1);

    let mut total_sampled: usize = 0;
    let mut color_count: usize = 0;
    let mut sum_r: f64 = 0.0;
    let mut sum_g: f64 = 0.0;
    let mut sum_b: f64 = 0.0;

    let mut y = 0;
    while y < frame.height {
        let mut x = 0;
        while x < frame.width {
            let idx = ((y * frame.width + x) * 4) as usize;
            if idx + 3 < frame.pixels_pbgra.len() {
                let b_pre = frame.pixels_pbgra[idx];
                let g_pre = frame.pixels_pbgra[idx + 1];
                let r_pre = frame.pixels_pbgra[idx + 2];
                let a = frame.pixels_pbgra[idx + 3];

                if a >= 16 {
                    total_sampled += 1;
                    let (r, g, b) = unpremultiply(r_pre, g_pre, b_pre, a);
                    let max_c = r.max(g).max(b);
                    let min_c = r.min(g).min(b);

                    if max_c > 20 && max_c < 250 {
                        let sat = (max_c - min_c) as f64 / max_c as f64;
                        if sat > 0.22 {
                            sum_r += r as f64;
                            sum_g += g as f64;
                            sum_b += b as f64;
                            color_count += 1;
                        }
                    }
                }
            }
            x += step_x;
        }
        y += step_y;
    }

    let min_colored = (total_sampled / 10).max(12);
    if color_count < min_colored {
        return "#F1F5F9".to_string();
    }

    let avg_r = (sum_r / color_count as f64) / 255.0;
    let avg_g = (sum_g / color_count as f64) / 255.0;
    let avg_b = (sum_b / color_count as f64) / 255.0;

    let (mut h, mut s, mut l) = rgb_to_hsl(avg_r, avg_g, avg_b);
    if use_complementary && s > 0.08 {
        h = (h + 0.5) % 1.0;
        s = s.max(0.76);
        l = l.clamp(0.56, 0.72);
    } else {
        s = s.max(0.65);
        l = l.clamp(0.55, 0.75);
    }

    let (r, g, b) = hsl_to_rgb(h, s, l);
    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    if luminance < 0.12 {
        return "#80D8FF".to_string();
    }

    format!(
        "#{:02X}{:02X}{:02X}",
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8
    )
}

fn unpremultiply(r: u8, g: u8, b: u8, a: u8) -> (u8, u8, u8) {
    if a == 255 || a == 0 {
        return (r, g, b);
    }
    let scale = 255.0 / a as f32;
    (
        (r as f32 * scale).round().clamp(0.0, 255.0) as u8,
        (g as f32 * scale).round().clamp(0.0, 255.0) as u8,
        (b as f32 * scale).round().clamp(0.0, 255.0) as u8,
    )
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    let d = max - min;

    if d <= 1e-6 {
        return (0.0, 0.0, l);
    }

    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let mut h = if (max - r).abs() < 1e-6 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h /= 6.0;

    (h.rem_euclid(1.0), s.clamp(0.0, 1.0), l.clamp(0.0, 1.0))
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s <= 1e-6 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    (r, g, b)
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_frame(w: u32, h: u32, r: u8, g: u8, b: u8) -> DecodedFrame {
        let mut pixels_pbgra = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            pixels_pbgra.extend_from_slice(&[b, g, r, 255]);
        }
        DecodedFrame {
            width: w,
            height: h,
            delay_ms: 100,
            pixels_pbgra,
        }
    }

    #[test]
    fn monochrome_cover_falls_back_to_light_slate() {
        let frame = solid_frame(16, 16, 140, 140, 140);
        assert_eq!(extract_album_accent(&frame, false), "#F1F5F9");
        assert_eq!(extract_album_accent(&frame, true), "#F1F5F9");
    }

    #[test]
    fn dark_cover_falls_back_to_light_slate_or_cyan() {
        let frame = solid_frame(16, 16, 12, 12, 12);
        assert_eq!(extract_album_accent(&frame, false), "#F1F5F9");
    }

    #[test]
    fn vivid_red_cover_returns_red_or_complementary_cyan() {
        let frame = solid_frame(16, 16, 220, 30, 40);
        let normal = extract_album_accent(&frame, false);
        let comp = extract_album_accent(&frame, true);
        assert_ne!(normal, "#F1F5F9");
        assert_ne!(normal, comp);

        let nr = u8::from_str_radix(&normal[1..3], 16).unwrap();
        let nb = u8::from_str_radix(&normal[5..7], 16).unwrap();
        assert!(nr > nb, "normal accent should lean red: {normal}");

        let cr = u8::from_str_radix(&comp[1..3], 16).unwrap();
        let cg = u8::from_str_radix(&comp[3..5], 16).unwrap();
        assert!(cg > cr, "complementary accent should shift toward cyan: {comp}");
    }

    #[test]
    fn pastel_mixed_cover_rotates_complementary() {
        // Low-saturation pastel blue (avg sat ~0.26) must still shift ~180 deg to warm coral/amber
        let frame = solid_frame(16, 16, 120, 155, 195);
        let normal = extract_album_accent(&frame, false);
        let comp = extract_album_accent(&frame, true);
        assert_ne!(normal, comp);
        let cr = u8::from_str_radix(&comp[1..3], 16).unwrap();
        let cb = u8::from_str_radix(&comp[5..7], 16).unwrap();
        assert!(cr > cb, "complementary accent for blue cover should lean warm orange: {comp}");
    }
}

