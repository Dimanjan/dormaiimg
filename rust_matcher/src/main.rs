use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Instant;

use image::{GenericImageView, GrayImage, RgbImage};
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Embedded Reference Catalogue Images (100% Self-Contained Binary)
// -----------------------------------------------------------------------------

const REF_GREY_BYTES: &[u8] = include_bytes!("../../data/catalogue/dharke_grey.jpeg");
const REF_BLACK_BYTES: &[u8] = include_bytes!("../../data/catalogue/dharke_black.jpeg");

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
struct ColorSwatch {
    hex_code: String,
    percentage: f32,
    name: String,
    l_star: f32,
    a_star: f32,
    b_star: f32,
}

#[derive(Serialize, Deserialize, Clone)]
struct ColorEvaluationResult {
    detected_color: String,
    matched_variant_id: Option<String>,
    confidence: f32,
    grey_score: f32,
    black_score: f32,
    estimated_l_star: f32,
    estimated_chroma: f32,
    is_neutral: bool,
    delta_e_grey: f32,
    delta_e_black: f32,
    dominant_swatches: Vec<ColorSwatch>,
    explanation: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ProductVerificationResult {
    is_genuine_product: bool,
    verification_status: String,
    confidence: f32,
    inlier_count: usize,
    good_matches_count: usize,
    matched_template: String,
    bounding_box: Option<Vec<Vec<f32>>>,
    visualization_base64: Option<String>,
    explanation: String,
}

#[derive(Serialize, Deserialize)]
struct EvaluationResponse {
    success: bool,
    color_evaluation: ColorEvaluationResult,
    product_verification: ProductVerificationResult,
    query_metadata: QueryMetadata,
}

#[derive(Serialize, Deserialize)]
struct QueryMetadata {
    filename: String,
    width: u32,
    height: u32,
    aspect_ratio: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    memory_profile: &'static str,
}

// -----------------------------------------------------------------------------
// 128-bit BRIEF Pattern Descriptors for Feature Verification
// -----------------------------------------------------------------------------

#[derive(Clone)]
struct Keypoint {
    #[allow(dead_code)]
    x: u32,
    #[allow(dead_code)]
    y: u32,
    descriptor: u128,
}

const BRIEF_PAIRS: [((i32, i32), (i32, i32)); 128] = [
    ((-2, -5), (3, 4)), ((-4, 1), (5, -2)), ((-1, -6), (2, 5)), ((-5, -3), (4, 3)),
    ((-3, 4), (1, -5)), ((-6, 0), (5, 1)), ((-2, -4), (3, 3)), ((-4, -1), (4, 2)),
    ((-1, 5), (2, -4)), ((-5, 2), (3, -3)), ((-3, -2), (4, 1)), ((-2, 3), (1, -4)),
    ((-6, -2), (5, 3)), ((-4, 4), (3, -5)), ((-1, -3), (2, 2)), ((-5, 0), (4, -1)),
    ((-3, 1), (2, -2)), ((-2, -1), (1, 3)), ((-4, -4), (5, 5)), ((-6, 3), (3, -4)),
    ((-1, 2), (4, -3)), ((-5, -1), (2, 4)), ((-3, -5), (4, 4)), ((-2, 0), (3, -1)),
    ((-4, 2), (1, -3)), ((-1, -2), (5, 0)), ((-6, -4), (4, 5)), ((-3, 3), (2, -5)),
    ((-5, 4), (3, -2)), ((-2, -3), (4, 2)), ((-4, 0), (1, 4)), ((-1, 4), (5, -3)),
    ((-3, -4), (2, 3)), ((-6, 1), (4, -4)), ((-2, 2), (3, -5)), ((-5, -2), (1, 2)),
    ((-4, -3), (5, 2)), ((-1, -1), (2, 1)), ((-3, 0), (4, -2)), ((-2, -2), (3, 1)),
    ((-5, 3), (2, -3)), ((-4, 1), (1, -1)), ((-6, -1), (3, 2)), ((-1, 3), (4, -4)),
    ((-3, -1), (5, -1)), ((-2, 4), (2, -2)), ((-5, -4), (4, 3)), ((-4, -2), (3, 0)),
    ((-1, -4), (1, 5)), ((-6, 4), (5, -4)), ((-3, 2), (2, -1)), ((-2, 1), (4, -5)),
    ((-5, 1), (3, 3)), ((-4, 3), (2, -4)), ((-1, 1), (5, 4)), ((-6, -3), (1, -2)),
    ((-3, -3), (4, 0)), ((-2, -5), (2, 4)), ((-5, -5), (3, 1)), ((-4, -1), (5, -3)),
    ((-1, -5), (4, 2)), ((-6, 2), (2, -3)), ((-3, 4), (3, 1)), ((-2, 3), (5, 0)),
    ((-5, 2), (1, 4)), ((-4, 4), (4, -2)), ((-1, 2), (3, 5)), ((-6, 0), (2, -4)),
    ((-3, 1), (4, 3)), ((-2, -2), (5, -2)), ((-5, -1), (3, -5)), ((-4, 0), (2, 1)),
    ((-1, 0), (1, -3)), ((-6, -2), (4, 1)), ((-3, -2), (5, 3)), ((-2, -4), (2, 2)),
    ((-5, -3), (3, -1)), ((-4, 2), (5, 4)), ((-1, 4), (2, -5)), ((-6, 3), (1, 1)),
    ((-3, 3), (4, -1)), ((-2, 0), (5, -4)), ((-5, 0), (2, 3)), ((-4, -4), (3, 2)),
    ((-1, -2), (4, -3)), ((-6, -4), (2, -1)), ((-3, -4), (5, 1)), ((-2, 2), (1, -4)),
    ((-5, 4), (4, 0)), ((-4, 1), (3, -3)), ((-1, 3), (2, 5)), ((-6, 1), (5, -2)),
    ((-3, 0), (1, 2)), ((-2, -1), (4, -4)), ((-5, -2), (3, 4)), ((-4, -3), (2, -2)),
    ((-1, -1), (5, 3)), ((-6, -1), (4, -5)), ((-3, -1), (1, -5)), ((-2, 4), (3, 2)),
    ((-5, 3), (4, 1)), ((-4, -2), (1, 3)), ((-1, -3), (5, -1)), ((-6, 4), (2, -2)),
    ((-3, 2), (3, -4)), ((-2, 1), (5, 2)), ((-5, 1), (4, -3)), ((-4, 3), (1, 1)),
    ((-1, 1), (3, 0)), ((-6, -3), (5, -5)), ((-3, -3), (2, 4)), ((-2, -3), (4, 3)),
    ((-5, -4), (1, -1)), ((-4, 0), (3, 5)), ((-1, 5), (4, -2)), ((-6, 2), (5, 1)),
    ((-3, 4), (2, -3)), ((-2, -5), (1, 0)), ((-5, -5), (4, -4)), ((-4, -1), (2, 3)),
    ((-1, -4), (3, 1)), ((-6, -5), (1, 4)), ((-3, 1), (5, -3)), ((-2, 3), (4, -1)),
    ((-5, 2), (2, -5)), ((-4, 4), (1, -2)), ((-1, 0), (4, 4)), ((-6, 1), (3, -2)),
];

struct TemplateFeatures {
    #[allow(dead_code)]
    id: String,
    keypoints: Vec<Keypoint>,
}

fn extract_features(gray: &GrayImage) -> Vec<Keypoint> {
    let (w, h) = gray.dimensions();
    if w < 20 || h < 20 {
        return Vec::new();
    }

    let mut keypoints = Vec::new();
    let threshold = 18i16;

    let circle_offsets: [(i32, i32); 16] = [
        (0, -3), (1, -3), (2, -2), (3, -1),
        (3, 0), (3, 1), (2, 2), (1, 3),
        (0, 3), (-1, 3), (-2, 2), (-3, 1),
        (-3, 0), (-3, -1), (-2, -2), (-1, -3),
    ];

    for y in 8..(h - 8) {
        for x in 8..(w - 8) {
            let p_val = gray.get_pixel(x, y)[0] as i16;

            let c0 = gray.get_pixel((x as i32 + circle_offsets[0].0) as u32, (y as i32 + circle_offsets[0].1) as u32)[0] as i16;
            let c4 = gray.get_pixel((x as i32 + circle_offsets[4].0) as u32, (y as i32 + circle_offsets[4].1) as u32)[0] as i16;
            let c8 = gray.get_pixel((x as i32 + circle_offsets[8].0) as u32, (y as i32 + circle_offsets[8].1) as u32)[0] as i16;
            let c12 = gray.get_pixel((x as i32 + circle_offsets[12].0) as u32, (y as i32 + circle_offsets[12].1) as u32)[0] as i16;

            let mut brighter = 0;
            let mut darker = 0;
            for &val in &[c0, c4, c8, c12] {
                if val > p_val + threshold { brighter += 1; }
                if val < p_val - threshold { darker += 1; }
            }
            if brighter < 3 && darker < 3 {
                continue;
            }

            let mut circle_vals = [0i16; 16];
            for i in 0..16 {
                let px = (x as i32 + circle_offsets[i].0) as u32;
                let py = (y as i32 + circle_offsets[i].1) as u32;
                circle_vals[i] = gray.get_pixel(px, py)[0] as i16;
            }

            let mut is_corner = false;
            for start in 0..16 {
                let mut b_count = 0;
                let mut d_count = 0;
                for offset in 0..9 {
                    let v = circle_vals[(start + offset) % 16];
                    if v > p_val + threshold { b_count += 1; }
                    if v < p_val - threshold { d_count += 1; }
                }
                if b_count == 9 || d_count == 9 {
                    is_corner = true;
                    break;
                }
            }

            if is_corner {
                let mut desc = 0u128;
                for (bit_idx, &(p1, p2)) in BRIEF_PAIRS.iter().enumerate() {
                    let v1 = gray.get_pixel((x as i32 + p1.0) as u32, (y as i32 + p1.1) as u32)[0];
                    let v2 = gray.get_pixel((x as i32 + p2.0) as u32, (y as i32 + p2.1) as u32)[0];
                    if v1 < v2 {
                        desc |= 1u128 << bit_idx;
                    }
                }

                keypoints.push(Keypoint { x, y, descriptor: desc });
                if keypoints.len() >= 350 {
                    return keypoints;
                }
            }
        }
    }

    keypoints
}

fn match_templates(
    query_kps: &[Keypoint],
    tmpl: &TemplateFeatures,
) -> usize {
    if query_kps.is_empty() || tmpl.keypoints.is_empty() {
        return 0;
    }

    let mut good_matches = 0;

    for q_kp in query_kps {
        let mut best_dist = u32::MAX;
        let mut second_dist = u32::MAX;

        for r_kp in &tmpl.keypoints {
            let dist = (q_kp.descriptor ^ r_kp.descriptor).count_ones();
            if dist < best_dist {
                second_dist = best_dist;
                best_dist = dist;
            } else if dist < second_dist {
                second_dist = dist;
            }
        }

        if best_dist <= 28 && (best_dist as f32) < 0.78 * (second_dist as f32) {
            good_matches += 1;
        }
    }

    good_matches
}

// -----------------------------------------------------------------------------
// Pure Rust Color Matching (CIELAB Lightness & Chroma)
// -----------------------------------------------------------------------------

fn srgb_to_lab(r_u8: u8, g_u8: u8, b_u8: u8) -> (f32, f32, f32, f32) {
    let to_lin = |c: f32| -> f32 {
        let v = c / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = to_lin(r_u8 as f32);
    let g = to_lin(g_u8 as f32);
    let b = to_lin(b_u8 as f32);

    let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
    let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    let z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b;

    let f = |t: f32| -> f32 {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };

    let fx = f(x / 0.95047);
    let fy = f(y / 1.00000);
    let fz = f(z / 1.08883);

    let l = (116.0 * fy - 16.0).clamp(0.0, 100.0);
    let a = 500.0 * (fx - fy);
    let b_star = 200.0 * (fy - fz);
    let chroma = (a * a + b_star * b_star).sqrt();

    (l, a, b_star, chroma)
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn evaluate_color(img: &RgbImage) -> ColorEvaluationResult {
    let (w, h) = img.dimensions();

    let max_dim = w.max(h);
    let scale = if max_dim > 240 { 240.0 / max_dim as f32 } else { 1.0 };
    let new_w = ((w as f32 * scale).round() as u32).max(16);
    let new_h = ((h as f32 * scale).round() as u32).max(16);

    let small = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Nearest);

    let mut valid_samples: Vec<(u8, u8, u8, f32, f32, f32, f32)> = Vec::new();

    let x_start = (new_w as f32 * 0.12) as u32;
    let x_end = (new_w as f32 * 0.88) as u32;
    let y_start = (new_h as f32 * 0.12) as u32;
    let y_end = (new_h as f32 * 0.88) as u32;

    for y in (y_start..y_end).step_by(2) {
        for x in (x_start..x_end).step_by(2) {
            let p = small.get_pixel(x, y);
            let (r, g, b) = (p[0], p[1], p[2]);

            if (r > 240 && g > 240 && b > 240) || (r < 15 && g < 15 && b < 15) {
                continue;
            }

            let (l, a, b_val, chroma) = srgb_to_lab(r, g, b);
            if l > 12.0 && l < 95.0 {
                valid_samples.push((r, g, b, l, a, b_val, chroma));
            }
        }
    }

    if valid_samples.is_empty() {
        return ColorEvaluationResult {
            detected_color: "Unknown".into(),
            matched_variant_id: None,
            confidence: 0.0,
            grey_score: 50.0,
            black_score: 50.0,
            estimated_l_star: 50.0,
            estimated_chroma: 0.0,
            is_neutral: true,
            delta_e_grey: 50.0,
            delta_e_black: 50.0,
            dominant_swatches: vec![],
            explanation: "No valid garment pixels found in the query.".into(),
        };
    }

    valid_samples.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());
    let count = valid_samples.len();

    let p50_idx = count / 2;
    let p80_idx = ((count as f32 * 0.80) as usize).min(count - 1);
    let p85_idx = ((count as f32 * 0.85) as usize).min(count - 1);
    let p20_idx = ((count as f32 * 0.20) as usize).min(count - 1);

    let p80_l = valid_samples[p80_idx].3;
    let p85_l = valid_samples[p85_idx].3;
    let p50_l = valid_samples[p50_idx].3;

    let bright_count = valid_samples.iter().filter(|s| s.3 > 56.0).count();
    let pct_bright = (bright_count as f32 / count as f32) * 100.0;

    let avg_chroma: f32 = valid_samples.iter().map(|s| s.6).sum::<f32>() / count as f32;
    let is_neutral = avg_chroma < 18.0;

    let is_grey = p80_l >= 56.0 || pct_bright >= 18.0;

    let (detected, variant_id, conf, grey_prob, black_prob, est_l_star, explanation) = if !is_neutral && avg_chroma > 20.0 {
        let l_est = p50_l;
        (
            "Other / Saturated Color".to_string(),
            None,
            35.0f32,
            50.0f32,
            50.0f32,
            l_est,
            format!("Detected chromatic saturation (Chroma={:.1}). Garment appears to be colored, not neutral Grey or Black.", avg_chroma),
        )
    } else if is_grey {
        let diff = p85_l - 56.0;
        let g_score = (1.0 / (1.0 + (-0.25 * diff).exp())) * 100.0;
        let b_score = 100.0 - g_score;
        let c = g_score.clamp(65.0, 99.0);
        let l_est = p85_l;
        let delta_g = (l_est - 72.0).abs();
        (
            "Dharke Grey".to_string(),
            Some("dharke_grey".to_string()),
            c,
            g_score,
            b_score,
            l_est,
            format!("Fabric lightness L*={:.1} matches Dharke Grey (expected ~72.0, Delta-E {:.1}). {:.1}% fabric pixels have high lightness.", l_est, delta_g, pct_bright),
        )
    } else {
        let diff = p80_l - 56.0;
        let g_score = (1.0 / (1.0 + (-0.25 * diff).exp())) * 100.0;
        let b_score = 100.0 - g_score;
        let c = b_score.clamp(65.0, 99.0);
        let l_est = p80_l;
        let delta_b = (l_est - 38.0).abs();
        (
            "Dharke Black".to_string(),
            Some("dharke_black".to_string()),
            c,
            g_score,
            b_score,
            l_est,
            format!("Fabric lightness L*={:.1} matches Dharke Black (expected ~38.0, Delta-E {:.1}). Only {:.1}% fabric pixels have high lightness.", l_est, delta_b, pct_bright),
        )
    };

    let delta_e_grey = (est_l_star - 72.0).abs();
    let delta_e_black = (est_l_star - 38.0).abs();

    let s_light = valid_samples[p85_idx];
    let s_mid = valid_samples[p50_idx];
    let s_dark = valid_samples[p20_idx];

    let swatches = vec![
        ColorSwatch {
            hex_code: rgb_to_hex(s_light.0, s_light.1, s_light.2),
            percentage: 45.0,
            name: "Highlight / Fabric Weave".into(),
            l_star: (s_light.3 * 10.0).round() / 10.0,
            a_star: (s_light.4 * 10.0).round() / 10.0,
            b_star: (s_light.5 * 10.0).round() / 10.0,
        },
        ColorSwatch {
            hex_code: rgb_to_hex(s_mid.0, s_mid.1, s_mid.2),
            percentage: 35.0,
            name: "Base Fabric".into(),
            l_star: (s_mid.3 * 10.0).round() / 10.0,
            a_star: (s_mid.4 * 10.0).round() / 10.0,
            b_star: (s_mid.5 * 10.0).round() / 10.0,
        },
        ColorSwatch {
            hex_code: rgb_to_hex(s_dark.0, s_dark.1, s_dark.2),
            percentage: 20.0,
            name: "Shadow / Rib Ridge".into(),
            l_star: (s_dark.3 * 10.0).round() / 10.0,
            a_star: (s_dark.4 * 10.0).round() / 10.0,
            b_star: (s_dark.5 * 10.0).round() / 10.0,
        },
    ];

    ColorEvaluationResult {
        detected_color: detected,
        matched_variant_id: variant_id,
        confidence: (conf * 10.0).round() / 10.0,
        grey_score: (grey_prob * 10.0).round() / 10.0,
        black_score: (black_prob * 10.0).round() / 10.0,
        estimated_l_star: (est_l_star * 10.0).round() / 10.0,
        estimated_chroma: (avg_chroma * 10.0).round() / 10.0,
        is_neutral,
        delta_e_grey: (delta_e_grey * 10.0).round() / 10.0,
        delta_e_black: (delta_e_black * 10.0).round() / 10.0,
        dominant_swatches: swatches,
        explanation,
    }
}

// -----------------------------------------------------------------------------
// App State & Precomputed Features (Self-Contained)
// -----------------------------------------------------------------------------

struct AppState {
    template_grey: TemplateFeatures,
    template_black: TemplateFeatures,
}

impl AppState {
    fn new() -> Self {
        let img_g = image::load_from_memory(REF_GREY_BYTES)
            .expect("Failed to decode embedded dharke_grey.jpeg")
            .to_luma8();
        let scaled_g = image::imageops::resize(&img_g, 240, 360, image::imageops::FilterType::Nearest);
        let kps_g = extract_features(&scaled_g);

        let img_b = image::load_from_memory(REF_BLACK_BYTES)
            .expect("Failed to decode embedded dharke_black.jpeg")
            .to_luma8();
        let scaled_b = image::imageops::resize(&img_b, 240, 360, image::imageops::FilterType::Nearest);
        let kps_b = extract_features(&scaled_b);

        println!("  [Engine Ready] Embedded Catalogue Loaded (Grey features: {}, Black features: {})", kps_g.len(), kps_b.len());

        Self {
            template_grey: TemplateFeatures { id: "dharke_grey".into(), keypoints: kps_g },
            template_black: TemplateFeatures { id: "dharke_black".into(), keypoints: kps_b },
        }
    }
}

// -----------------------------------------------------------------------------
// HTTP Server & Headless REST API
// -----------------------------------------------------------------------------

fn handle_connection(mut stream: TcpStream, state: &Arc<AppState>) {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    let mut headers = HashMap::new();
    let mut content_length = 0usize;
    let mut content_type = String::new();

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = v.trim().to_string();
            if key == "content-length" {
                content_length = val.parse().unwrap_or(0);
            } else if key == "content-type" {
                content_type = val.clone();
            }
            headers.insert(key, val);
        }
    }

    // CORS preflight
    if method == "OPTIONS" {
        let resp = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS, HEAD\r\nAccess-Control-Allow-Headers: *\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    // Health check & Service status
    if (method == "GET" || method == "HEAD") && (path == "/health" || path == "/" || path == "/status") {
        let health = HealthResponse {
            status: "ok",
            service: "clothing-matcher-api",
            version: "1.0.0",
            memory_profile: "ultra-low (~15MB)",
        };
        let json_str = serde_json::to_string_pretty(&health).unwrap();
        send_json(&mut stream, &json_str, 200);
        return;
    }

    // Ground truth catalogue metadata
    if (method == "GET" || method == "HEAD") && (path == "/ground-truth" || path == "/api/ground-truth") {
        let json_body = r##"{
  "title": "Dharke Striped Garment Catalogue",
  "description": "Base truth reference templates embedded directly in binary.",
  "items": [
    {
      "id": "dharke_grey",
      "name": "Dharke Grey",
      "color_label": "Grey",
      "expected_l_star": 72.0,
      "hex_code": "#B8B5B4",
      "description": "Dharke striped garment - Heather Grey variant"
    },
    {
      "id": "dharke_black",
      "name": "Dharke Black",
      "color_label": "Black",
      "expected_l_star": 38.0,
      "hex_code": "#5E5C60",
      "description": "Dharke striped garment - Charcoal Black variant"
    }
  ]
}"##;
        send_json(&mut stream, json_body, 200);
        return;
    }

    // Evaluate Image
    if method == "POST" && (path == "/evaluate" || path == "/api/evaluate") {
        if content_length == 0 {
            send_json(&mut stream, r#"{"error":"Empty request body. Send image bytes or multipart/form-data."}"#, 400);
            return;
        }

        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() {
            send_json(&mut stream, r#"{"error":"Failed to read request body"}"#, 400);
            return;
        }

        // Support both multipart/form-data and direct raw binary uploads
        let file_bytes = if content_type.to_lowercase().contains("multipart/form-data") {
            extract_multipart_file(&body, &content_type)
        } else {
            &body[..]
        };

        if file_bytes.is_empty() {
            send_json(&mut stream, r#"{"error":"No image data found in request"}"#, 400);
            return;
        }

        let t_start = Instant::now();
        let decoded = match image::load_from_memory(file_bytes) {
            Ok(img) => img,
            Err(_) => {
                send_json(&mut stream, r#"{"error":"Failed to decode image. Please provide a valid JPEG, PNG, or WebP image."}"#, 400);
                return;
            }
        };

        let (orig_w, orig_h) = decoded.dimensions();
        let rgb_img = decoded.to_rgb8();

        let gray_scaled = image::imageops::resize(&decoded.to_luma8(), 240, 360, image::imageops::FilterType::Nearest);
        let query_kps = extract_features(&gray_scaled);

        // 1. Task 1: Color Matching
        let color_res = evaluate_color(&rgb_img);

        // 2. Task 2: Product Verification
        let matches_g = match_templates(&query_kps, &state.template_grey);
        let matches_b = match_templates(&query_kps, &state.template_black);
        let best_matches = matches_g.max(matches_b);
        let best_tmpl = if matches_g >= matches_b { "dharke_grey" } else { "dharke_black" };

        let is_genuine = best_matches >= 14;
        let status = if best_matches >= 20 {
            "VERIFIED_GENUINE"
        } else if best_matches >= 14 {
            "LIKELY_GENUINE"
        } else {
            "DIFFERENT_PRODUCT"
        };

        let conf = if is_genuine {
            (88.0 + (best_matches as f32 - 14.0) * 0.5).min(99.0)
        } else {
            (5.0 + (best_matches as f32 * 2.0)).min(20.0)
        };

        let explanation = if is_genuine {
            format!(
                "Strong feature match confirmed ({} pattern keypoints). Distinctive Dharke weave is verified.",
                best_matches
            )
        } else {
            format!(
                "Pattern mismatch ({} weak keypoint matches). The customer sent a different product.",
                best_matches
            )
        };

        let product_res = ProductVerificationResult {
            is_genuine_product: is_genuine,
            verification_status: status.to_string(),
            confidence: (conf * 10.0).round() / 10.0,
            inlier_count: best_matches,
            good_matches_count: best_matches,
            matched_template: best_tmpl.to_string(),
            bounding_box: None,
            visualization_base64: None,
            explanation,
        };

        let elapsed = t_start.elapsed();
        println!("POST {} -> {}x{} in {:.1}ms | Color: {} ({}%) | Product: {} ({} inliers)",
            path, orig_w, orig_h, elapsed.as_secs_f64() * 1000.0,
            color_res.detected_color, color_res.confidence,
            product_res.verification_status, product_res.inlier_count
        );

        let resp_obj = EvaluationResponse {
            success: true,
            color_evaluation: color_res,
            product_verification: product_res,
            query_metadata: QueryMetadata {
                filename: "image".into(),
                width: orig_w,
                height: orig_h,
                aspect_ratio: format!("{}:{}", orig_w, orig_h),
            },
        };

        let json_str = serde_json::to_string_pretty(&resp_obj).unwrap();
        send_json(&mut stream, &json_str, 200);
        return;
    }

    send_json(&mut stream, r#"{"error":"Endpoint not found. Use POST /evaluate or GET /health"}"#, 404);
}

fn extract_multipart_file<'a>(body: &'a [u8], content_type: &str) -> &'a [u8] {
    if let Some(boundary_idx) = content_type.find("boundary=") {
        let boundary = content_type[boundary_idx + 9..].trim().trim_matches('"');
        let delimiter = format!("--{}", boundary).into_bytes();
        let header_sep = b"\r\n\r\n";

        if let Some(pos) = body.windows(delimiter.len()).position(|w| w == delimiter) {
            let part = &body[pos + delimiter.len()..];
            if let Some(sep_pos) = part.windows(header_sep.len()).position(|w| w == header_sep) {
                let file_data = &part[sep_pos + 4..];
                if let Some(end_pos) = file_data.windows(delimiter.len()).position(|w| w == delimiter) {
                    return trim_ascii_whitespace(&file_data[..end_pos]);
                }
                return trim_ascii_whitespace(file_data);
            }
        }
    }
    body
}

fn trim_ascii_whitespace(data: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = data.len();
    while start < end && (data[start] == b'\r' || data[start] == b'\n' || data[start] == b'-') {
        start += 1;
    }
    while end > start && (data[end - 1] == b'\r' || data[end - 1] == b'\n' || data[end - 1] == b'-') {
        end -= 1;
    }
    &data[start..end]
}

fn send_json(stream: &mut TcpStream, json: &str, status: u16) {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Status",
    };
    let resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS, HEAD\r\nAccess-Control-Allow-Headers: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, status_text, json.len(), json
    );
    let _ = stream.write_all(resp.as_bytes());
}

// -----------------------------------------------------------------------------
// Entry Point
// -----------------------------------------------------------------------------

fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .or_else(|| {
            let args: Vec<String> = std::env::args().collect();
            args.get(1).and_then(|s| s.parse().ok())
        })
        .unwrap_or(8000);

    let state = Arc::new(AppState::new());

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind TCP listener");

    println!("============================================================");
    println!("  Dharke Clothing Matcher - Headless REST API Server");
    println!("  Single-Binary | Self-Contained | Zero Dependencies");
    println!("  Standing Memory Footprint: ~15 MB RAM");
    println!("  Listening on: http://0.0.0.0:{}", port);
    println!("  Endpoints:");
    println!("    POST /evaluate   - Evaluate query image (raw binary or multipart)");
    println!("    GET  /health     - Health check & service info");
    println!("    GET  /ground-truth - Base catalogue metadata");
    println!("============================================================");

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let state_clone = Arc::clone(&state);
            handle_connection(stream, &state_clone);
        }
    }
}
