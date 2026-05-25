use crate::emulator::EmulatorState;
use crate::escpos::printer::{PrinterState, ReceiptLine};
use egui::{Color32, ColorImage, Frame, Margin, Rounding, ScrollArea, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ReceiptViewer {
    show_paper_edges: bool,
    auto_scroll: bool,
    /// Cache of rendered bitmap textures (keyed by data hash)
    bitmap_cache: HashMap<u64, TextureHandle>,
}

impl Default for ReceiptViewer {
    fn default() -> Self {
        Self {
            show_paper_edges: true,
            auto_scroll: true,
            bitmap_cache: HashMap::new(),
        }
    }
}

fn hash_bytes(data: &[u8]) -> u64 {
    // Simple FNV-1a hash for cache key
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

impl ReceiptViewer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui, emulator_state: &Arc<Mutex<EmulatorState>>) {
        ui.horizontal(|ui| {
            ui.heading("RECEIPT VIEWER");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("CLEAR").clicked() {
                    if let Ok(mut state) = emulator_state.try_lock() {
                        state.clear_printer_buffer();
                    }
                    self.bitmap_cache.clear();
                }
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                ui.checkbox(&mut self.show_paper_edges, "Edges");
                
                ui.separator();
                
                if let Ok(mut state) = emulator_state.try_lock() {
                    let mut current_width = match state.get_printer_state().paper_width {
                        crate::escpos::printer::PaperWidth::Width58mm => 0,
                        crate::escpos::printer::PaperWidth::Width80mm => 1,
                    };
                    
                    if ui.selectable_value(&mut current_width, 0, "58mm").changed() {
                        state.set_paper_width(50);
                    }
                    if ui.selectable_value(&mut current_width, 1, "80mm").changed() {
                        state.set_paper_width(80);
                    }
                }
            });
        });
        
        ui.add_space(10.0);

        // Receipt display area
        ScrollArea::vertical()
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    if let Ok(state) = emulator_state.try_lock() {
                        self.render_receipt(ui, &state);
                    } else {
                        ui.label("Cannot load emulator state");
                    }
                });
            });
    }

    fn render_receipt(&mut self, ui: &mut Ui, state: &EmulatorState) {
        let printer_state = state.get_printer_state();
        let buffer = printer_state.get_buffer();

        if buffer.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label(egui::RichText::new("No receipt data available").size(18.0).weak());
                ui.label("Send ESC/POS commands to see the receipt here");
            });
            return;
        }

        // Paper simulation
        let paper_width_mm = match printer_state.paper_width {
            crate::escpos::printer::PaperWidth::Width58mm => 58.0,
            crate::escpos::printer::PaperWidth::Width80mm => 80.0,
        };
        
        // Scale mm to pixels (roughly 3.8 pixels per mm for visual representation)
        let display_width = paper_width_mm * 4.0;
        let paper_color = Color32::from_rgb(252, 252, 245); // Off-white/Cream
        let ink_color = Color32::from_rgb(40, 40, 45); // Dark grey "thermal" ink

        Frame::none()
            .fill(paper_color)
            .rounding(Rounding {
                nw: 2.0,
                ne: 2.0,
                sw: 0.0,
                se: 0.0,
            })
            .shadow(egui::epaint::Shadow {
                extrusion: 10.0,
                color: Color32::from_black_alpha(40),
            })
            .inner_margin(Margin::symmetric(20.0, 30.0))
            .show(ui, |ui| {
                ui.set_width(display_width);
                ui.vertical(|ui| {
                    ui.add_space(5.0);

                    // Receipt content
                    for line in buffer {
                        match line {
                            ReceiptLine::Text(text) => {
                                let mut rich_text = egui::RichText::new(text)
                                    .monospace()
                                    .color(ink_color)
                                    .size(13.0);

                                if printer_state.emphasis {
                                    rich_text = rich_text.strong();
                                }

                                match printer_state.justification {
                                    crate::escpos::commands::Justification::Left => {
                                        ui.label(rich_text);
                                    }
                                    crate::escpos::commands::Justification::Center => {
                                        ui.vertical_centered(|ui| ui.label(rich_text));
                                    }
                                    crate::escpos::commands::Justification::Right => {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(rich_text));
                                    }
                                }
                            }
                            ReceiptLine::Bitmap { width_px, height_px, data } => {
                                self.render_bitmap(ui, *width_px, *height_px, data, display_width);
                            }
                            ReceiptLine::Barcode { data, .. } => {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(5.0);
                                    let barcode_width = (display_width * 0.8).min(200.0);
                                    let barcode_height = 40.0;
                                    let (rect, _response) = ui.allocate_at_least(egui::vec2(barcode_width, barcode_height), egui::Sense::hover());
                                    
                                    let painter = ui.painter();
                                    // Draw a "simulated" barcode
                                    let mut x = rect.left();
                                    let mut i = 0;
                                    while x < rect.right() {
                                        let w = if i % 3 == 0 { 2.0 } else { 1.0 };
                                        if i % 2 == 0 {
                                            painter.rect_filled(
                                                egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(x + w, rect.bottom())),
                                                0.0,
                                                ink_color
                                            );
                                        }
                                        x += w + 1.0;
                                        i += 1;
                                    }
                                    
                                    ui.label(egui::RichText::new(data).monospace().size(10.0).color(ink_color));
                                    ui.add_space(5.0);
                                });
                            }
                            ReceiptLine::Separator => {
                                ui.add_space(5.0);
                                ui.painter().hline(
                                    ui.cursor().left()..=ui.cursor().right(),
                                    ui.cursor().top(),
                                    Stroke::new(1.0, Color32::from_gray(200)),
                                );
                                ui.add_space(5.0);
                            }
                        }
                    }

                    // Bottom "serrated" edge visualization
                    ui.add_space(20.0);
                    let rect = ui.available_rect_before_wrap();
                    let painter = ui.painter();
                    let serrated_y = rect.top() + 10.0;
                    
                    if self.show_paper_edges {
                        for x in (rect.left() as i32..rect.right() as i32).step_by(10) {
                            painter.line_segment(
                                [
                                    egui::pos2(x as f32, serrated_y),
                                    egui::pos2(x as f32 + 5.0, serrated_y + 5.0),
                                ],
                                Stroke::new(1.0, Color32::from_gray(200)),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(x as f32 + 5.0, serrated_y + 5.0),
                                    egui::pos2(x as f32 + 10.0, serrated_y),
                                ],
                                Stroke::new(1.0, Color32::from_gray(200)),
                            );
                        }
                    }
                });
            });
    }

    fn render_bitmap(&mut self, ui: &mut Ui, width_px: u32, height_px: u32, data: &[u8], paper_width: f32) {
        let cache_key = hash_bytes(data);

        // Get or create texture
        let texture = self.bitmap_cache.entry(cache_key).or_insert_with(|| {
            let rgb_image = PrinterState::bitmap_to_rgb(width_px, height_px, data);
            let size = [rgb_image.width() as usize, rgb_image.height() as usize];
            let pixels: Vec<egui::Color32> = rgb_image
                .pixels()
                .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                .collect();
            let color_image = ColorImage { size, pixels };
            ui.ctx().load_texture(
                format!("bitmap_{}", cache_key),
                color_image,
                TextureOptions::NEAREST,
            )
        });

        // Scale to fit paper width
        let scale = (paper_width / width_px as f32).min(1.0);
        let display_size = Vec2::new(width_px as f32 * scale, height_px as f32 * scale);
        
        ui.vertical_centered(|ui| {
            ui.image((texture.id(), display_size));
        });
    }
}

