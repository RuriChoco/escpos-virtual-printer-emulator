use crate::emulator::EmulatorState;
use egui::{Ui, RichText, Color32, Grid};
use std::process::Command;

pub struct SettingsPanel {
    diagnostic_output: String,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            diagnostic_output: "Ready for diagnostics...".to_string(),
        }
    }
}

impl SettingsPanel {
    pub fn show(&mut self, ui: &mut Ui, _state: &mut EmulatorState) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("PRINTER SETTINGS");
            ui.add_space(10.0);

            // Virtual printer management
            egui::CollapsingHeader::new(RichText::new("SYSTEM PRINTER INTEGRATION").strong())
                .default_open(true)
                .show(ui, |ui| {
                    ui.label("Connect the virtual printer to your operating system's spooler.");
                    ui.add_space(8.0);
                    
                    Grid::new("printer_mgmt_grid")
                        .num_columns(2)
                        .spacing([20.0, 12.0])
                        .show(ui, |ui| {
                            let is_windows = cfg!(target_os = "windows");
                            let is_linux = cfg!(target_os = "linux");

                            ui.label("Windows (PowerShell):");
                            ui.horizontal(|ui| {
                                if ui.add_enabled(is_windows, egui::Button::new("INSTALL").min_size(egui::vec2(80.0, 0.0))).clicked() {
                                    self.install_windows_printer();
                                }
                                if ui.add_enabled(is_windows, egui::Button::new("UNINSTALL").min_size(egui::vec2(80.0, 0.0))).clicked() {
                                    self.uninstall_windows_printer();
                                }
                            });
                            ui.end_row();

                            ui.label("Linux (CUPS):");
                            ui.horizontal(|ui| {
                                if ui.add_enabled(is_linux, egui::Button::new("INSTALL").min_size(egui::vec2(80.0, 0.0))).clicked() {
                                    self.install_linux_printer();
                                }
                                if ui.add_enabled(is_linux, egui::Button::new("UNINSTALL").min_size(egui::vec2(80.0, 0.0))).clicked() {
                                    self.uninstall_linux_printer();
                                }
                            });
                            ui.end_row();

                            ui.label("Maintenance:");
                            if ui.button("RUN DIAGNOSTIC").clicked() {
                                self.check_printer_status();
                            }
                            ui.end_row();
                        });
                    
                    ui.add_space(10.0);
                    if cfg!(target_os = "linux") {
                        ui.label(RichText::new("⚠ Notice: You are on Linux. Use the CUPS driver options.").color(Color32::from_rgb(255, 165, 0)).size(11.0));
                    }
                    ui.label(RichText::new("Note: Requires administrator/sudo privileges").size(10.0).weak());
                });

            ui.add_space(15.0);

            // Network settings
            egui::CollapsingHeader::new(RichText::new("NETWORK CONFIGURATION").strong())
                .default_open(true)
                .show(ui, |ui| {
                    ui.label("The emulator listens for ESC/POS data on these parameters.");
                    ui.add_space(8.0);
                    
                    Grid::new("network_grid")
                        .num_columns(2)
                        .spacing([20.0, 12.0])
                        .show(ui, |ui| {
                            ui.label("Host Address:");
                            ui.label(RichText::new("127.0.0.1").monospace().color(Color32::LIGHT_BLUE));
                            ui.end_row();

                            ui.label("TCP Port:");
                            ui.label(RichText::new("9100").monospace().color(Color32::LIGHT_BLUE));
                            ui.end_row();

                            ui.label("Connectivity:");
                            if ui.button("TEST PORT 9100").clicked() {
                                self.test_network_connection();
                            }
                            ui.end_row();
                        });
                });

            ui.add_space(15.0);

            // Diagnostic Console
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("DIAGNOSTIC CONSOLE").strong().size(14.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("CLEAR").clicked() {
                                self.diagnostic_output.clear();
                            }
                        });
                    });
                    ui.add_space(5.0);
                    
                    egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.diagnostic_output)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(12)
                                    .interactive(false)
                            );
                        });
                });
            });

            ui.add_space(20.0);
        });
    }

    fn log_diag(&mut self, msg: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        self.diagnostic_output.push_str(&format!("[{}] {}\n", timestamp, msg));
        println!("[DIAG] {}", msg);
    }

    fn install_windows_printer(&mut self) {
        let output = Command::new("powershell")
            .args([
                "-Command",
                "Add-PrinterPort -Name '127.0.0.1:9100' -PrinterHostAddress '127.0.0.1' -PortNumber 9100; \
                 $driver = (Get-PrinterDriver | Where-Object { $_.Name -like '*Microsoft*' } | Select-Object -First 1).Name; \
                 Add-Printer -Name 'ESC_POS_Virtual_Printer' -DriverName $driver -PortName '127.0.0.1:9100'; \
                 Write-Host 'Printer installed successfully'"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    self.log_diag("✅ Windows printer installed successfully");
                } else {
                    self.log_diag(&format!("❌ Error: {}", String::from_utf8_lossy(&output.stderr)));
                }
            }
            Err(e) => self.log_diag(&format!("❌ Failed to launch PowerShell: {}", e)),
        }
    }

    fn install_linux_printer(&mut self) {
        let output = Command::new("pkexec")
            .args([
                "bash",
                "-c",
                "lpadmin -p ESC_POS_Linux_Printer -E -v socket://127.0.0.1:9100 -m raw && \
                 lpadmin -d ESC_POS_Linux_Printer"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    self.log_diag("✅ Linux printer installed successfully as raw device");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("was dismissed") || stderr.contains("not authorized") {
                        self.log_diag("ℹ️ Installation cancelled by user");
                    } else {
                        self.log_diag(&format!("❌ Error: {}", stderr));
                    }
                }
            }
            Err(e) => self.log_diag(&format!("❌ Failed to launch pkexec: {}", e)),
        }
    }

    fn uninstall_windows_printer(&mut self) {
        let output = Command::new("powershell")
            .args([
                "-Command",
                "Remove-Printer -Name 'ESC_POS_Virtual_Printer' -Confirm:$false; \
                 Remove-PrinterPort -Name '127.0.0.1:9100'"
            ])
            .output();

        match output {
            Ok(_) => self.log_diag("✅ Windows printer uninstalled successfully"),
            Err(e) => self.log_diag(&format!("❌ Failed to uninstall Windows printer: {}", e)),
        }
    }

    fn uninstall_linux_printer(&mut self) {
        let output = Command::new("pkexec")
            .args([
                "lpadmin",
                "-x",
                "ESC_POS_Linux_Printer"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    self.log_diag("✅ Linux printer uninstalled successfully");
                } else {
                    self.log_diag(&format!("❌ Error: {}", String::from_utf8_lossy(&output.stderr)));
                }
            }
            Err(e) => self.log_diag(&format!("❌ Failed to launch pkexec: {}", e)),
        }
    }

    fn check_printer_status(&mut self) {
        if cfg!(target_os = "windows") {
            let output = Command::new("powershell")
                .args([
                    "-Command",
                    "Get-Printer -Name 'ESC_POS_Virtual_Printer' -ErrorAction SilentlyContinue | Select-Object Name, PortName, PrinterStatus"
                ])
                .output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.trim().is_empty() {
                        self.log_diag("ℹ️ Virtual printer is not installed on Windows.");
                    } else {
                        self.log_diag(&format!("✅ Windows Status:\n{}", stdout));
                    }
                }
                Err(e) => self.log_diag(&format!("❌ Status check failed: {}", e)),
            }
        } else if cfg!(target_os = "linux") {
            let output = Command::new("lpstat")
                .args(["-p", "ESC_POS_Linux_Printer"])
                .output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains("printer ESC_POS_Linux_Printer") {
                        self.log_diag(&format!("✅ Linux Status: Installed and ready\n{}", stdout));
                    } else {
                        self.log_diag("ℹ️ Linux Status: Printer 'ESC_POS_Linux_Printer' not found.");
                    }
                }
                Err(e) => self.log_diag(&format!("❌ Linux status check failed: {}", e)),
            }
        }
    }

    fn test_network_connection(&mut self) {
        use std::net::{TcpStream, SocketAddr};
        use std::time::Duration;

        let addr: SocketAddr = "127.0.0.1:9100".parse().unwrap();
        match TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
            Ok(_) => self.log_diag("✅ Success: Port 9100 is open and reachable"),
            Err(e) => self.log_diag(&format!("❌ Error: Cannot connect to port 9100: {}", e)),
        }
    }
}

