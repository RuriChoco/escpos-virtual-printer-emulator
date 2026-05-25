use crate::emulator::EmulatorState;
use crate::gui::{CommandLog, ReceiptViewer, SettingsPanel};
use eframe::egui::{self, CentralPanel, SidePanel, TopBottomPanel, Frame, Margin, Color32, Rounding, Vec2};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Receipt,
    Commands,
    Settings,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Receipt
    }
}

pub struct EscPosEmulatorApp {
    pub emulator_state: std::sync::Arc<tokio::sync::Mutex<EmulatorState>>,
    selected_tab: Tab,
    receipt_viewer: ReceiptViewer,
    command_log: CommandLog,
    settings_panel: SettingsPanel,
    is_dark_mode: bool,
}

impl Default for EscPosEmulatorApp {
    fn default() -> Self {
        Self {
            emulator_state: std::sync::Arc::new(tokio::sync::Mutex::new(EmulatorState::new())),
            selected_tab: Tab::Receipt,
            receipt_viewer: ReceiptViewer::new(),
            command_log: CommandLog::new(),
            settings_panel: SettingsPanel::default(),
            is_dark_mode: true,
        }
    }
}

impl EscPosEmulatorApp {
    pub fn new(emulator_state: std::sync::Arc<tokio::sync::Mutex<EmulatorState>>) -> Self {
        Self {
            emulator_state,
            ..Default::default()
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = if self.is_dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        // Premium touches
        visuals.widgets.noninteractive.bg_fill = if self.is_dark_mode { Color32::from_rgb(15, 15, 20) } else { Color32::from_rgb(245, 245, 250) };
        visuals.widgets.noninteractive.rounding = Rounding::same(6.0);
        visuals.window_rounding = Rounding::same(10.0);
        
        ctx.set_visuals(visuals);
    }
}

impl eframe::App for EscPosEmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.show(ctx);
    }
}

impl EscPosEmulatorApp {
    fn show(&mut self, ctx: &egui::Context) {
        // Side Navigation Panel
        SidePanel::left("side_panel")
            .resizable(false)
            .default_width(180.0)
            .frame(Frame::none().fill(ctx.style().visuals.widgets.noninteractive.bg_fill).inner_margin(Margin::same(15.0)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.heading("PRINTER EMULATOR");
                    ui.label(egui::RichText::new("ESC/POS Emulator").size(10.0).color(Color32::GRAY));
                    ui.add_space(20.0);
                });

                ui.vertical(|ui| {
                    let nav_button = |ui: &mut egui::Ui, tab: Tab, icon: &str, label: &str, current: &mut Tab| {
                        let is_selected = *current == tab;
                        
                        if is_selected {
                            ui.visuals_mut().widgets.active.bg_fill = Color32::from_rgb(60, 100, 255);
                            ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_rgb(80, 120, 255);
                            ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(40, 80, 235);
                        }

                        let button = egui::Button::new(
                            egui::RichText::new(format!("{}  {}", icon, label))
                                .size(14.0)
                                .color(if is_selected { Color32::WHITE } else { ui.visuals().text_color() })
                        )
                        .frame(true)
                        .min_size(Vec2::new(ui.available_width(), 40.0))
                        .rounding(Rounding::same(8.0));

                        if ui.add(button).clicked() {
                            *current = tab;
                        }
                        ui.add_space(8.0);
                    };

                    nav_button(ui, Tab::Receipt, "PRINT", "Receipt", &mut self.selected_tab);
                    nav_button(ui, Tab::Commands, "LOGS", "Commands", &mut self.selected_tab);
                    nav_button(ui, Tab::Settings, "SET", "Settings", &mut self.selected_tab);
                });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    if ui.button(if self.is_dark_mode { "Light" } else { "Dark" }).clicked() {
                        self.is_dark_mode = !self.is_dark_mode;
                    }
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });

        // Status Bar at the bottom
        TopBottomPanel::bottom("status_bar")
            .frame(Frame::none().fill(ctx.style().visuals.widgets.noninteractive.bg_fill).inner_margin(Margin::symmetric(15.0, 5.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("[ON]").color(Color32::from_rgb(0, 255, 100)).strong());
                    ui.label("Network: Online (Port 9100)");
                    ui.separator();
                    
                    if let Ok(state) = self.emulator_state.try_lock() {
                        let p = state.get_printer_state();
                        ui.label(format!("{:?} | Font: {:?}", p.paper_width, p.current_font));
                    } else {
                        ui.label("Status: Ready");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("Built with Rust & egui");
                    });
                });
            });

        // Main Content Area
        CentralPanel::default()
            .frame(Frame::none().fill(if ctx.style().visuals.dark_mode { Color32::from_rgb(25, 25, 35) } else { Color32::from_rgb(240, 240, 245) }).inner_margin(Margin::same(20.0)))
            .show(ctx, |ui| {
                match self.selected_tab {
                    Tab::Receipt => {
                        self.receipt_viewer.show(ui, &self.emulator_state);
                    }
                    Tab::Commands => {
                        self.command_log.show(ui, &self.emulator_state);
                    }
                    Tab::Settings => {
                        if let Ok(mut state) = self.emulator_state.try_lock() {
                            self.settings_panel.show(ui, &mut state);
                        }
                    }
                }
            });
    }
}

