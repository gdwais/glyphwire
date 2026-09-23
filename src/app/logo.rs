//! Original bitmap artwork, painted on the physical pixel grid (no font or image filtering).
use super::{egui, Color32, CYAN, MAGENTA, TEXT};
use egui::emath::GuiRounding;

const EMBLEM: [&str; 16] = [
    "...CCCCCCCC.....",
    "..CC......CC....",
    ".CC........CC...",
    ".C...CCCC...C...",
    ".C..CC......C...",
    ".C..C.......C...",
    ".C..C..CCC..C...",
    ".C..C....C..C...",
    ".C..CC..CC..C...",
    ".C...CCCC...C...",
    ".CC........CC...",
    "..CC......CC.M..",
    "...CCCCCCCC.M...",
    "...........M..M.",
    "..........M..M..",
    "............M...",
];

// Five-by-seven letterforms; the narrow spacing and double slash echo a terminal badge.
const WORDMARK: [(char, [u8; 7]); 10] = [
    ('G', [14, 17, 16, 23, 17, 17, 14]),
    ('L', [16, 16, 16, 16, 16, 16, 31]),
    ('Y', [17, 17, 10, 4, 4, 4, 4]),
    ('P', [30, 17, 17, 30, 16, 16, 16]),
    ('H', [17, 17, 17, 31, 17, 17, 17]),
    ('/', [1, 2, 2, 4, 8, 8, 16]),
    ('W', [17, 17, 17, 21, 21, 21, 10]),
    ('I', [31, 4, 4, 4, 4, 4, 31]),
    ('R', [30, 17, 17, 30, 20, 18, 17]),
    ('E', [31, 16, 16, 30, 16, 16, 31]),
];

pub(super) fn show(ui: &mut egui::Ui) {
    let scale = ui.pixels_per_point();
    // Integer physical pixels keep edges sharp at Retina and fractional UI scales.
    let pixel = 2.0_f32.round_to_pixels(scale).max(1.0 / scale);
    let letter_pixel = 2.5_f32.round_to_pixels(scale).max(pixel);
    let width = 22.0 * pixel + 69.0 * letter_pixel;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 16.0 * pixel), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Glyphwire"));
    response.on_hover_text("GLYPH // WIRE — Markdown editor");
    let painter = ui.painter();
    let origin = rect.min.round_to_pixels(scale);
    let paint_pixel = |pos, size, color: Color32| {
        painter.rect_filled(
            egui::Rect::from_min_size(pos, egui::Vec2::splat(size)),
            0,
            color,
        );
    };
    for (y, row) in EMBLEM.iter().enumerate() {
        for (x, value) in row.bytes().enumerate() {
            let color = match value {
                b'C' => CYAN,
                b'M' => MAGENTA,
                _ => continue,
            };
            paint_pixel(
                origin + egui::vec2(x as f32, y as f32) * pixel,
                pixel,
                color,
            );
        }
    }
    let baseline = (origin + egui::vec2(22.0 * pixel, (rect.height() - 7.0 * letter_pixel) / 2.0))
        .round_to_pixels(scale);
    let mut column = 0;
    for (word, color) in [("GLYPH", CYAN), ("//", MAGENTA), ("WIRE", TEXT)] {
        for character in word.chars() {
            let (_, rows) = WORDMARK
                .iter()
                .find(|(letter, _)| *letter == character)
                .expect("Logo glyph");
            for (y, row) in rows.iter().enumerate() {
                for x in 0..5 {
                    if row & (1 << (4 - x)) != 0 {
                        paint_pixel(
                            baseline + egui::vec2((column + x) as f32, y as f32) * letter_pixel,
                            letter_pixel,
                            color,
                        );
                    }
                }
            }
            column += 6;
        }
        column += 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artwork_has_valid_pixel_dimensions_and_glyphs() {
        for row in EMBLEM {
            assert_eq!(row.len(), 16);
            assert!(row.bytes().all(|pixel| b".CM".contains(&pixel)));
        }
        for character in "GLYPH//WIRE".chars() {
            let (_, rows) = WORDMARK
                .iter()
                .find(|(letter, _)| *letter == character)
                .unwrap();
            assert!(rows.iter().all(|row| *row < 32));
        }
    }

    #[test]
    fn logo_paints_crisp_theme_pixels_inside_its_bounds() {
        for scale in [1.0, 1.25, 2.0] {
            let context = egui::Context::default();
            context.set_pixels_per_point(scale);
            let output = context.run(egui::RawInput::default(), |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    show(ui);
                });
            });
            let mut colors = Vec::new();
            for clipped in output.shapes {
                if let egui::Shape::Rect(shape) = clipped.shape {
                    if [CYAN, MAGENTA, TEXT].contains(&shape.fill) {
                        colors.push(shape.fill);
                        assert!(clipped.clip_rect.contains_rect(shape.rect));
                        for edge in [
                            shape.rect.min.x,
                            shape.rect.min.y,
                            shape.rect.max.x,
                            shape.rect.max.y,
                        ] {
                            let physical = edge * output.pixels_per_point;
                            assert!((physical - physical.round()).abs() < 0.001);
                        }
                    }
                }
            }
            assert!([CYAN, MAGENTA, TEXT]
                .iter()
                .all(|color| colors.contains(color)));
        }
    }
}
