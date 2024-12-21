use std::io::Cursor;

use ab_glyph::{Font, FontArc, GlyphId, PxScale, ScaleFont};
use imageproc::{
    drawing::draw_text_mut,
    image::{self, codecs::webp::WebPEncoder, ColorType, ImageBuffer, Rgba, RgbaImage},
};

const BACKGROUND_IMAGE: &[u8] = include_bytes!("../assets/Tile1.webp");
const TEXT_COLOR: Rgba<u8> = Rgba([70, 50, 40, 255]);
/*
const SHADOW_COLOR: Rgba<u8> = Rgba([50, 50, 50, 80]);
const RECT_COLOR: Rgba<u8> = Rgba([220, 210, 200, 80]);
*/

fn draw_soft_centered_rect(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: Rgba<u8>,
    blur_radius: u32, // Radius für die weiche Transparenz
) {
    let blur_radius = blur_radius as f32;

    for dx in 0..width {
        for dy in 0..height {
            let px = x + dx as i32;
            let py = y + dy as i32;

            if px >= 0 && py >= 0 && px < img.width() as i32 && py < img.height() as i32 {
                // Abstand zu den Rändern berechnen
                let distance_to_edge_x = (dx as f32).min((width - dx) as f32);
                let distance_to_edge_y = (dy as f32).min((height - dy) as f32);
                let edge_distance = distance_to_edge_x.min(distance_to_edge_y);

                // Weiche Kante: Alpha-Wert basierend auf Abstand zu den Rändern
                let edge_alpha = if edge_distance < blur_radius {
                    edge_distance / blur_radius
                } else {
                    1.0
                };

                // Basis-Transparenz anwenden
                let base_alpha = color[3] as f32 / 255.0;
                let final_alpha = base_alpha * edge_alpha;

                // Hintergrund mischen (Alpha-Blending)
                let base_pixel = img.get_pixel(px as u32, py as u32);
                let blended_pixel = Rgba([
                    ((1.0 - final_alpha) * base_pixel[0] as f32 + final_alpha * color[0] as f32)
                        as u8,
                    ((1.0 - final_alpha) * base_pixel[1] as f32 + final_alpha * color[1] as f32)
                        as u8,
                    ((1.0 - final_alpha) * base_pixel[2] as f32 + final_alpha * color[2] as f32)
                        as u8,
                    255, // Endpixel ist voll sichtbar
                ]);

                img.put_pixel(px as u32, py as u32, blended_pixel);
            }
        }
    }
}

pub fn create_haiku_image(
    haiku: &str,
) -> Result<Cursor<Vec<u8>>, Box<dyn std::error::Error>> {
    // Bild einlesen
    let img_result = image::load_from_memory(BACKGROUND_IMAGE)
        .map_err(|e| format!("Konnte das eingebettete Hintergrundbild nicht laden: {}", e))?;

    let mut img: RgbaImage = ImageBuffer::from(img_result);
    let (width, height) = img.dimensions();

    // Schriftart laden
    let font_data = include_bytes!("../assets/LeagueSpartan-Medium.ttf");
    let font = FontArc::try_from_slice(font_data)?;

    // Schriftgröße festlegen
    let scale = PxScale::from(55.0);
    let scaled_font = font.as_scaled(scale);

    // Zeilenhöhe aus der Schriftgröße ableiten
    let ascent = scaled_font.ascent();
    let descent = scaled_font.descent();
    let line_height = ((ascent - descent) + 30.0).ceil() as i32;
    let total_text_height = line_height * haiku.lines().count() as i32;

    // Rechteck
    let rect_width = width as i32 * 9 / 10; // 90% der Bildbreite
    let rect_height = total_text_height + 60; // Zusätzlicher Rand
    let rect_x = (width as i32 - rect_width) / 2;
    let rect_y = ((height as i32 - rect_height) / 2) + descent as i32;
    
    // Weiches Rechteck zeichnen
    draw_soft_centered_rect(
        &mut img,
        rect_x,
        rect_y,
        rect_width as u32,
        rect_height as u32,
        Rgba([220, 210, 200, 200]), // Warmer Farbton mit leichter Transparenz
        30, // Weichheit der Kanten
    );

    /*
    // Halbtransparentes Rechteck zeichnen
    draw_transparent_rect(
        &mut img,
        rect_x,
        rect_y,
        rect_width as u32,
        rect_height as u32,
        RECT_COLOR,
    );
    */

    // Zentrierte Y-Position
    let mut y_offset = (height as i32 - total_text_height) / 2;

    let mut max_text_width = 0.0;

    // Text auf das Bild zeichnen
    for line in haiku.lines() {
        // Textbreite berechnen
        let mut text_width: f32 = 0.0;
        let mut prev_glyph_id: Option<GlyphId> = None;
        for ch in line.chars() {
            let glyph_id = font.glyph_id(ch);

            if let Some(prev_id) = prev_glyph_id {
                // Kerning berücksichtigen
                text_width += scaled_font.kern(prev_id, glyph_id);
            }

            // Advance Width hinzufügen
            text_width += scaled_font.h_advance(glyph_id);

            prev_glyph_id = Some(glyph_id);
        }

        if text_width > max_text_width {
            max_text_width = text_width;
        }

        // Zentrierte X-Position
        let x_pos = ((width as f32 - text_width) / 2.0) as i32;

        /*
        // Schlagschatten zeichnen (leicht versetzt)
        draw_text_mut(
            &mut img,
            SHADOW_COLOR,
            x_pos + 2,    // x-Position leicht nach rechts
            y_offset + 2, // y-Position leicht nach unten
            scale,
            &font,
            line,
        );
        */

        // Haupttext zeichnen
        draw_text_mut(&mut img, TEXT_COLOR, x_pos, y_offset, scale, &font, line);
        y_offset += line_height;
    }

    // Bild als WebP kodieren
    let mut buffer = Cursor::new(Vec::new());
    let encoder = WebPEncoder::new_lossless(&mut buffer);
    encoder.encode(
        &img,
        img.width(),
        img.height(),
        ColorType::Rgba8.into(),
    )?;

    Ok(buffer)
}
