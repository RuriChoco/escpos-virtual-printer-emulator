use crate::emulator::EmulatorState;
use egui::{Color32, Frame, Margin, ScrollArea, Ui, RichText};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CommandLog {
    show_timestamps: bool,
    show_raw_data: bool,
    max_display_lines: usize,
    filter_text: String,
}

impl Default for CommandLog {
    fn default() -> Self {
        Self {
            show_timestamps: true,
            show_raw_data: false,
            max_display_lines: 1000,
            filter_text: String::new(),
        }
    }
}

impl CommandLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui, emulator_state: &Arc<Mutex<EmulatorState>>) {
        ui.horizontal(|ui| {
            ui.heading("COMMAND LOG");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("CLEAR").clicked() {
                    if let Ok(mut state) = emulator_state.try_lock() {
                        state.clear_history();
                    }
                }
                ui.checkbox(&mut self.show_raw_data, "Raw Data");
                ui.checkbox(&mut self.show_timestamps, "Timestamps");
            });
        });

        ui.add_space(10.0);

        // Filter bar
        ui.horizontal(|ui| {
            ui.label("SEARCH:");
            ui.text_edit_singleline(&mut self.filter_text);
            if !self.filter_text.is_empty() {
                if ui.button("X").clicked() {
                    self.filter_text.clear();
                }
            }
        });

        ui.add_space(5.0);

        // Log area
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if let Ok(state) = emulator_state.try_lock() {
                self.render_command_list(ui, &state);
            } else {
                ui.label("Cannot load emulator state");
            }
        });
    }

    fn render_command_list(&self, ui: &mut Ui, state: &EmulatorState) {
        let history = state.get_command_history();
        
        if history.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(RichText::new("No commands received").weak());
            });
            return;
        }

        // Apply filter
        let filtered_commands: Vec<_> = history.iter()
            .filter(|entry| {
                if self.filter_text.is_empty() {
                    return true;
                }
                
                let search_term = self.filter_text.to_lowercase();
                match &entry.command {
                    crate::escpos::commands::EscPosCommand::Text(text) => {
                        text.to_lowercase().contains(&search_term)
                    }
                    _ => {
                        format!("{:?}", entry.command).to_lowercase().contains(&search_term)
                    }
                }
            })
            .collect();

        // Limit displayed lines
        let display_commands: Vec<_> = filtered_commands.iter()
            .rev() // Most recent first
            .take(self.max_display_lines)
            .collect();

        for entry in &display_commands {
            self.render_command_entry(ui, entry);
            ui.add_space(4.0);
        }

        if filtered_commands.len() > self.max_display_lines {
            ui.label(format!("... and {} more commands hidden by limit", filtered_commands.len() - self.max_display_lines));
        }
    }

    fn render_command_entry(&self, ui: &mut Ui, entry: &crate::emulator::CommandEntry) {
        let (icon, label, color) = match &entry.command {
            crate::escpos::commands::EscPosCommand::Text(text) => {
                ("TXT", format!("TEXT: \"{}\"", text), Color32::from_rgb(100, 200, 255))
            }
            crate::escpos::commands::EscPosCommand::NewLine => {
                ("LF", "NEW LINE".to_string(), Color32::from_rgb(150, 150, 150))
            }
            crate::escpos::commands::EscPosCommand::SetFont(_) | 
            crate::escpos::commands::EscPosCommand::SetJustification(_) |
            crate::escpos::commands::EscPosCommand::SetEmphasis(_) |
            crate::escpos::commands::EscPosCommand::SetUnderline(_) |
            crate::escpos::commands::EscPosCommand::SetItalic(_) |
            crate::escpos::commands::EscPosCommand::SetFontSize(_) |
            crate::escpos::commands::EscPosCommand::SetLineHeight(_) |
            crate::escpos::commands::EscPosCommand::SetCodepage(_) => {
                ("SET", format!("{:?}", entry.command), Color32::from_rgb(150, 255, 150))
            }
            crate::escpos::commands::EscPosCommand::CutPaper => {
                ("CUT", "PAPER CUT".to_string(), Color32::from_rgb(255, 150, 50))
            }
            crate::escpos::commands::EscPosCommand::GeneratePulse { .. } => {
                ("DRW", "CASH DRAWER PULSE".to_string(), Color32::from_rgb(255, 200, 50))
            }
            crate::escpos::commands::EscPosCommand::PrintAndFeed(n) => {
                ("FED", format!("FEED {} LINES", n), Color32::from_rgb(150, 200, 150))
            }
            crate::escpos::commands::EscPosCommand::SetBarcodeHeight(_) |
            crate::escpos::commands::EscPosCommand::SetBarcodeWidth(_) |
            crate::escpos::commands::EscPosCommand::SetHriFont(_) |
            crate::escpos::commands::EscPosCommand::SetHriPosition(_) => {
                ("BAR", format!("{:?}", entry.command), Color32::from_rgb(100, 255, 200))
            }
            crate::escpos::commands::EscPosCommand::PrintImage(_) |
            crate::escpos::commands::EscPosCommand::PrintRasterImage { .. } => {
                ("IMG", "IMAGE DATA".to_string(), Color32::from_rgb(200, 150, 255))
            }
            crate::escpos::commands::EscPosCommand::Unknown(bytes) => {
                ("???", format!("UNKNOWN: {:02X?}", bytes), Color32::from_rgb(255, 100, 100))
            }
            _ => {
                ("CMD", format!("{:?}", entry.command), Color32::from_rgb(200, 200, 200))
            }
        };

        Frame::none()
            .fill(ui.visuals().widgets.noninteractive.bg_fill)
            .rounding(4.0)
            .inner_margin(Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if self.show_timestamps {
                        if let Ok(duration) = entry.timestamp.duration_since(std::time::UNIX_EPOCH) {
                            let secs = duration.as_secs() % 60;
                            let mins = (duration.as_secs() / 60) % 60;
                            ui.label(RichText::new(format!("{:02}:{:02}", mins, secs)).size(10.0).weak());
                        }
                    }

                    ui.label(RichText::new(icon).size(10.0).strong());
                    ui.label(RichText::new(label).color(color).monospace());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("COPY").on_hover_text("Copy command text").clicked() {
                            ui.output_mut(|o| o.copied_text = format!("{:?}", entry.command));
                        }
                    });
                });

                if self.show_raw_data && !entry.raw_data.is_empty() {
                    ui.add_space(2.0);
                    let hex_data: String = entry.raw_data.iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    ui.label(RichText::new(format!("RAW: {}", hex_data)).size(9.0).weak().monospace());
                }
            });
    }
}

