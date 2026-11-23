#[cfg(target_os = "windows")]
use tao::window::Icon;

pub const BADGE_SIZE: u32 = 32;

/// Soma as notificações não lidas de todas as abas abertas.
pub fn compute_total_unread<'a>(counts: impl Iterator<Item = Option<&'a str>>) -> (u64, bool) {
    let mut total: u64 = 0;
    let mut has_plus = false;

    for unread in counts.flatten() {
        if let Some(digits) = unread.strip_suffix('+') {
            has_plus = true;
            total = total.saturating_add(digits.parse::<u64>().unwrap_or(0));
        } else {
            total = total.saturating_add(unread.parse::<u64>().unwrap_or(0));
        }
    }

    (total, has_plus)
}

/// Formata o texto curto do badge da barra de tarefas (`None` quando 0).
pub fn format_badge_label(total: u64, has_plus: bool) -> Option<String> {
    if total == 0 {
        None
    } else if total > 99 {
        Some("99+".to_string())
    } else if has_plus {
        Some(format!("{total}+"))
    } else {
        Some(total.to_string())
    }
}

fn glyph_5x7(ch: char) -> [u8; 7] {
    match ch {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b01110, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        _ => [0; 7],
    }
}

fn glyph_compact_6(ch: char) -> (usize, [u8; 6]) {
    match ch {
        '0' => (4, [0b0110, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110]),
        '1' => (4, [0b0010, 0b0110, 0b0010, 0b0010, 0b0010, 0b0111]),
        '2' => (4, [0b1110, 0b0001, 0b0110, 0b1000, 0b1000, 0b1111]),
        '3' => (4, [0b1110, 0b0001, 0b0110, 0b0001, 0b0001, 0b1110]),
        '4' => (4, [0b1001, 0b1001, 0b1111, 0b0001, 0b0001, 0b0001]),
        '5' => (4, [0b1111, 0b1000, 0b1110, 0b0001, 0b0001, 0b1110]),
        '6' => (4, [0b0111, 0b1000, 0b1110, 0b1001, 0b1001, 0b0110]),
        '7' => (4, [0b1111, 0b0001, 0b0010, 0b0100, 0b0100, 0b0100]),
        '8' => (4, [0b0110, 0b1001, 0b0110, 0b1001, 0b1001, 0b0110]),
        '9' => (4, [0b0110, 0b1001, 0b0111, 0b0001, 0b0001, 0b1110]),
        '+' => (3, [0b000, 0b010, 0b111, 0b010, 0b000, 0b000]),
        _ => (3, [0; 6]),
    }
}

fn draw_badge_background(rgba: &mut [u8], is_pill: bool) {
    let size = BADGE_SIZE as usize;
    let center = (BADGE_SIZE as f32 - 1.0) * 0.5;
    let border_thickness = 1.7_f32;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;

            let dist = if is_pill {
                let r = 12.5_f32;
                let hx = 15.5_f32;
                let hy = 12.5_f32;
                let dx = (fx - center).abs() - (hx - r);
                let dy = (fy - center).abs() - (hy - r);
                dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r
            } else {
                (fx - center).hypot(fy - center) - 15.5_f32
            };

            let alpha = (0.5 - dist).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }

            // Transição suave entre preenchimento vermelho (#E53935) e borda branca
            let ring_mix = ((dist + border_thickness + 0.5).clamp(0.0, 1.0)) as f32;
            let r = (229.0 * (1.0 - ring_mix) + 255.0 * ring_mix).round() as u8;
            let g = (57.0 * (1.0 - ring_mix) + 255.0 * ring_mix).round() as u8;
            let b = (53.0 * (1.0 - ring_mix) + 255.0 * ring_mix).round() as u8;
            let a = (alpha * 255.0).round() as u8;

            let idx = (y * size + x) * 4;
            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = a;
        }
    }
}

fn fill_white_rect(rgba: &mut [u8], x0: usize, y0: usize, w: usize, h: usize) {
    let size = BADGE_SIZE as usize;
    for dy in 0..h {
        let py = y0 + dy;
        if py >= size {
            continue;
        }
        for dx in 0..w {
            let px = x0 + dx;
            if px >= size {
                continue;
            }
            let idx = (py * size + px) * 4;
            rgba[idx] = 255;
            rgba[idx + 1] = 255;
            rgba[idx + 2] = 255;
            rgba[idx + 3] = 255;
        }
    }
}

/// Renderiza um badge `32x32` RGBA com fundo vermelho, borda branca e dígitos nítidos.
pub fn render_badge_rgba(label: &str) -> Vec<u8> {
    let size = BADGE_SIZE as usize;
    let mut rgba = vec![0u8; size * size * 4];
    let chars: Vec<char> = label.chars().take(3).collect();
    let is_pill = chars.len() >= 3;

    draw_badge_background(&mut rgba, is_pill);

    if chars.is_empty() {
        return rgba;
    }

    if chars.len() <= 2 {
        let scale = if chars.len() == 1 { 3 } else { 2 };
        let gap = if chars.len() == 1 { 0 } else { 2 };
        let char_w = 5 * scale;
        let char_h = 7 * scale;
        let total_w = chars.len() * char_w + (chars.len() - 1) * gap;
        let mut cursor_x = (size - total_w) / 2;
        let start_y = (size - char_h) / 2;

        for ch in chars {
            let rows = glyph_5x7(ch);
            for (row_idx, bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    if (bits >> (4 - col)) & 1 == 1 {
                        fill_white_rect(
                            &mut rgba,
                            cursor_x + col * scale,
                            start_y + row_idx * scale,
                            scale,
                            scale,
                        );
                    }
                }
            }
            cursor_x += char_w + gap;
        }
    } else {
        let scale = 2;
        let gap = 1;
        let glyphs: Vec<(usize, [u8; 6])> = chars.iter().map(|&c| glyph_compact_6(c)).collect();
        let total_w: usize = glyphs.iter().map(|(w, _)| w * scale).sum::<usize>()
            + (glyphs.len() - 1) * gap;
        let mut cursor_x = (size - total_w) / 2;
        let start_y = (size - 6 * scale) / 2;

        for (gw, rows) in glyphs {
            for (row_idx, bits) in rows.iter().enumerate() {
                for col in 0..gw {
                    if (bits >> (gw - 1 - col)) & 1 == 1 {
                        fill_white_rect(
                            &mut rgba,
                            cursor_x + col * scale,
                            start_y + row_idx * scale,
                            scale,
                            scale,
                        );
                    }
                }
            }
            cursor_x += gw * scale + gap;
        }
    }

    rgba
}

#[cfg(target_os = "windows")]
pub fn create_badge_icon(label: &str) -> Option<Icon> {
    Icon::from_rgba(render_badge_rgba(label), BADGE_SIZE, BADGE_SIZE).ok()
}

/// Renderiza o ícone base `32x32` RGBA do WhatsApp (usado na bandeja quando há 0 não lidas).
pub fn render_whatsapp_icon_rgba() -> Vec<u8> {
    let size = BADGE_SIZE as usize;
    let mut rgba = vec![0u8; size * size * 4];
    let center = (BADGE_SIZE as f32 - 1.0) * 0.5;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;

            let circle_dist = (fx - center).hypot(fy - center) - 13.8_f32;
            // Pequena ponta do balão no canto inferior esquerdo
            let tail_dist = dist_point_to_triangle(
                (fx, fy),
                (6.0, 21.0),
                (2.5, 29.0),
                (11.0, 26.0),
            );
            let outer_dist = circle_dist.min(tail_dist);
            let alpha = (0.5 - outer_dist).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }

            let t = ((fx + fy) / (2.0 * (BADGE_SIZE as f32 - 1.0))).clamp(0.0, 1.0);
            let mut r = 37.0 * (1.0 - t) + 0.0 * t;
            let mut g = 211.0 * (1.0 - t) + 168.0 * t;
            let mut b = 102.0 * (1.0 - t) + 132.0 * t;

            // Anel branco nítido ao redor do centro
            let ring_dist = ((fx - center).hypot(fy - center) - 10.2_f32).abs() - 1.25_f32;
            // Arco estilizado interno representando telefone/chat
            let phone_dist = dist_phone_arc(fx, fy, center);
            let white_mix = (0.5 - ring_dist.min(phone_dist)).clamp(0.0, 1.0);

            r = r * (1.0 - white_mix) + 255.0 * white_mix;
            g = g * (1.0 - white_mix) + 255.0 * white_mix;
            b = b * (1.0 - white_mix) + 255.0 * white_mix;

            let idx = (y * size + x) * 4;
            rgba[idx] = r.round() as u8;
            rgba[idx + 1] = g.round() as u8;
            rgba[idx + 2] = b.round() as u8;
            rgba[idx + 3] = (alpha * 255.0).round() as u8;
        }
    }

    rgba
}

fn dist_phone_arc(x: f32, y: f32, center: f32) -> f32 {
    let dx = x - (center + 1.0);
    let dy = y - (center - 1.0);
    let angle = dy.atan2(dx);
    // Arco no quadrante inferior-esquerdo (~40° a 145°)
    let radius = 6.2_f32;
    let clamped_angle = angle.clamp(0.65, 2.50);
    let px = (center + 1.0) + radius * clamped_angle.cos();
    let py = (center - 1.0) + radius * clamped_angle.sin();
    (x - px).hypot(y - py) - 1.65_f32
}

fn dist_point_to_triangle(
    p: (f32, f32),
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
) -> f32 {
    let d1 = dist_point_to_segment(p, a, b);
    let d2 = dist_point_to_segment(p, b, c);
    let d3 = dist_point_to_segment(p, c, a);
    let min_edge = d1.min(d2).min(d3);

    let s1 = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    let s2 = (c.0 - b.0) * (p.1 - b.1) - (c.1 - b.1) * (p.0 - b.0);
    let s3 = (a.0 - c.0) * (p.1 - c.1) - (a.1 - c.1) * (p.0 - c.0);
    let inside = (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0);
    if inside {
        -min_edge
    } else {
        min_edge
    }
}

fn dist_point_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let vx = b.0 - a.0;
    let vy = b.1 - a.1;
    let len_sq = vx * vx + vy * vy;
    if len_sq == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / len_sq).clamp(0.0, 1.0);
    let px = a.0 + t * vx;
    let py = a.1 + t * vy;
    (p.0 - px).hypot(p.1 - py)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_unread_aggregates_all_tabs() {
        assert_eq!(compute_total_unread([None, None].into_iter()), (0, false));
        assert_eq!(
            compute_total_unread([Some("3"), None, Some("12")].into_iter()),
            (15, false)
        );
        assert_eq!(
            compute_total_unread([Some("5"), Some("99+")].into_iter()),
            (104, true)
        );
    }

    #[test]
    fn badge_label_formats_bounds() {
        assert_eq!(format_badge_label(0, false), None);
        assert_eq!(format_badge_label(1, false), Some("1".to_string()));
        assert_eq!(format_badge_label(42, false), Some("42".to_string()));
        assert_eq!(format_badge_label(99, false), Some("99".to_string()));
        assert_eq!(format_badge_label(100, false), Some("99+".to_string()));
        assert_eq!(format_badge_label(9, true), Some("9+".to_string()));
    }

    #[test]
    fn render_badge_rgba_draws_expected_pixels() {
        for label in ["3", "24", "99+"] {
            let rgba = render_badge_rgba(label);
            assert_eq!(rgba.len(), (BADGE_SIZE * BADGE_SIZE * 4) as usize);

            // Canto superior esquerdo deve permanecer transparente
            assert_eq!(rgba[3], 0);

            let mut red_pixels = 0usize;
            let mut white_pixels = 0usize;
            for px in rgba.chunks_exact(4) {
                if px[3] > 200 {
                    if px[0] > 200 && px[1] < 90 && px[2] < 90 {
                        red_pixels += 1;
                    } else if px[0] > 240 && px[1] > 240 && px[2] > 240 {
                        white_pixels += 1;
                    }
                }
            }
            assert!(red_pixels > 100, "missing red background for {label}");
            assert!(white_pixels > 40, "missing white digits for {label}");
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn creates_windows_overlay_icon() {
        assert!(create_badge_icon("5").is_some());
        assert!(create_badge_icon("99+").is_some());
    }

    #[test]
    fn renders_whatsapp_tray_base_icon() {
        let rgba = render_whatsapp_icon_rgba();
        assert_eq!(rgba.len(), (BADGE_SIZE * BADGE_SIZE * 4) as usize);
        assert_eq!(rgba[31 * 4 + 3], 0);

        let mut green_pixels = 0usize;
        let mut white_pixels = 0usize;
        for px in rgba.chunks_exact(4) {
            if px[3] > 200 {
                if px[1] > 140 && px[1] > px[0].saturating_add(40) {
                    green_pixels += 1;
                } else if px[0] > 240 && px[1] > 240 && px[2] > 240 {
                    white_pixels += 1;
                }
            }
        }
        assert!(green_pixels > 150, "expected green body pixels");
        assert!(white_pixels > 30, "expected white ring/handset pixels");
    }
}
