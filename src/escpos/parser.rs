use crate::escpos::commands::{EscPosCommand, Font, Justification};
use anyhow::Result;

pub struct EscPosParser {
    buffer: Vec<u8>,
}

impl EscPosParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }

    pub fn parse_stream(&mut self, data: &[u8]) -> Result<Vec<EscPosCommand>> {
        self.buffer.extend_from_slice(data);
        let mut commands = Vec::new();
        let mut i = 0;

        while i < self.buffer.len() {
            match self.buffer[i] {
                b'\n' => {
                    commands.push(EscPosCommand::NewLine);
                    i += 1;
                }
                b'\r' => {
                    commands.push(EscPosCommand::CarriageReturn);
                    i += 1;
                }
                0x1B => {
                    // ESC sequence
                    if i + 1 >= self.buffer.len() {
                        break; // Wait for more data
                    }
                    match self.parse_esc_command(&self.buffer[i..]) {
                        Ok(Some((cmd, consumed))) => {
                            commands.push(cmd);
                            i += consumed;
                        }
                        Ok(Option::None) => break, // Incomplete, wait for more
                        Err(_) => { i += 2; } // Skip bad ESC sequence
                    }
                }
                0x1D => {
                    // GS sequence
                    if i + 1 >= self.buffer.len() {
                        break;
                    }
                    match self.parse_gs_command(&self.buffer[i..]) {
                        Ok(Some((cmd, consumed))) => {
                            commands.push(cmd);
                            i += consumed;
                        }
                        Ok(Option::None) => break,
                        Err(_) => { i += 2; }
                    }
                }
                _ => {
                    // Normal text bytes
                    let text_start = i;
                    while i < self.buffer.len()
                        && self.buffer[i] != 0x1B
                        && self.buffer[i] != 0x1D
                        && self.buffer[i] != b'\n'
                        && self.buffer[i] != b'\r'
                    {
                        i += 1;
                    }
                    if i > text_start {
                        let text = self.decode_text(&self.buffer[text_start..i]);
                        if !text.is_empty() {
                            commands.push(EscPosCommand::Text(text));
                        }
                    }
                }
            }
        }

        if i > 0 {
            self.buffer.drain(0..i);
        }

        Ok(commands)
    }

    /// Parse ESC (0x1B) commands. Returns (command, bytes_consumed).
    fn parse_esc_command(&self, data: &[u8]) -> Result<Option<(EscPosCommand, usize)>> {
        if data.len() < 2 {
            return Ok(None);
        }

        match data[1] {
            // Initialize printer
            b'@' => Ok(Some((EscPosCommand::InitializePrinter, 2))),

            // Select font
            b'M' => {
                if data.len() < 3 { return Ok(None); }
                let font = match data[2] {
                    0 => Font::FontA,
                    1 => Font::FontB,
                    2 => Font::FontC,
                    _ => Font::FontA,
                };
                Ok(Some((EscPosCommand::SetFont(font), 3)))
            }

            // Justification
            b'a' => {
                if data.len() < 3 { return Ok(None); }
                let j = match data[2] {
                    0 => Justification::Left,
                    1 => Justification::Center,
                    2 => Justification::Right,
                    _ => Justification::Left,
                };
                Ok(Some((EscPosCommand::SetJustification(j), 3)))
            }

            // Emphasis on/off
            b'E' => Ok(Some((EscPosCommand::SetEmphasis(true), 2))),
            b'F' => Ok(Some((EscPosCommand::SetEmphasis(false), 2))),

            // Underline
            b'-' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetUnderline(data[2] != 0), 3)))
            }

            // Italic on/off
            b'4' => Ok(Some((EscPosCommand::SetItalic(true), 2))),
            b'5' => Ok(Some((EscPosCommand::SetItalic(false), 2))),

            // Line height
            b'3' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetLineHeight(data[2] as u32), 3)))
            }

            // Font size / print mode
            b'!' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetFontSize(data[2] as u32), 3)))
            }

            // Codepage selection (ESC t n)
            b't' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetCodepage(data[2]), 3)))
            }
            
            // Generate pulse (ESC p m t1 t2)
            b'p' => {
                if data.len() < 5 { return Ok(None); }
                Ok(Some((EscPosCommand::GeneratePulse { pin: data[2], t1: data[3], t2: data[4] }, 5)))
            }
            
            // Print and feed n lines (ESC d n)
            b'd' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::PrintAndFeed(data[2]), 3)))
            }

            // Cut paper
            b'm' | b'i' => Ok(Some((EscPosCommand::CutPaper, 2))),

            // Paper feed
            b'J' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::LineFeed, 3)))
            }

            // Bit image (ESC *) — simplified
            b'*' => {
                if data.len() < 4 { return Ok(None); }
                let m = data[2];
                let nl = data[3] as u16;
                if data.len() < 5 { return Ok(None); }
                let nh = data[4] as u16;
                let n_dots = nl + nh * 256;
                let bytes_per_col: u16 = match m { 0 | 1 => 1, 32 | 33 => 3, _ => 1 };
                let total = bytes_per_col as usize * n_dots as usize;
                let consumed = 5 + total;
                if data.len() < consumed { return Ok(None); }
                let image_data = data[5..consumed].to_vec();
                Ok(Some((EscPosCommand::PrintImage(image_data), consumed)))
            }

            _ => {
                Ok(Some((EscPosCommand::Unknown(data[..2].to_vec()), 2)))
            }
        }
    }

    /// Parse GS (0x1D) commands. Returns (command, bytes_consumed).
    fn parse_gs_command(&self, data: &[u8]) -> Result<Option<(EscPosCommand, usize)>> {
        if data.len() < 2 {
            return Ok(None);
        }

        match data[1] {
            // GS v 0 — Print raster bit image
            b'v' => {
                if data.len() < 8 { return Ok(None); }
                // GS v 0 m xL xH yL yH d1...dk
                let _mode = data[3]; // 0=normal, 1=double-width, 2=double-height, 3=both
                let x_l = data[4] as u16;
                let x_h = data[5] as u16;
                let y_l = data[6] as u16;
                let y_h = data[7] as u16;
                let width_bytes = x_l + x_h * 256; // bytes per row
                let height = y_l + y_h * 256;       // number of rows
                let total = width_bytes as usize * height as usize;
                let consumed = 8 + total;
                if data.len() < consumed { return Ok(None); }
                let image_data = data[8..consumed].to_vec();
                Ok(Some((
                    EscPosCommand::PrintRasterImage { width_bytes, height, data: image_data },
                    consumed,
                )))
            }

            // GS V — Cut paper (with variants)
            b'V' => {
                if data.len() < 3 { return Ok(None); }
                match data[2] {
                    0 | 1 => Ok(Some((EscPosCommand::CutPaper, 3))),
                    65 | 66 => {
                        // GS V 65/66 n — need one more byte
                        if data.len() < 4 { return Ok(None); }
                        Ok(Some((EscPosCommand::CutPaper, 4)))
                    }
                    _ => Ok(Some((EscPosCommand::CutPaper, 3))),
                }
            }

            // GS H — Select HRI position
            b'H' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetHriPosition(data[2]), 3)))
            }

            // GS f — Select HRI font
            b'f' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetHriFont(data[2]), 3)))
            }

            // GS w — Set barcode width
            b'w' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetBarcodeWidth(data[2]), 3)))
            }

            // GS h — Set barcode height
            b'h' => {
                if data.len() < 3 { return Ok(None); }
                Ok(Some((EscPosCommand::SetBarcodeHeight(data[2]), 3)))
            }

            // GS k — Print barcode
            b'k' => {
                if data.len() < 3 { return Ok(None); }
                let m = data[2];
                if m <= 6 {
                    // Format 1: GS k m d1...dk NUL
                    let mut k = 0;
                    while 3 + k < data.len() && data[3 + k] != 0 {
                        k += 1;
                    }
                    if 3 + k >= data.len() {
                        return Ok(None); // Incomplete
                    }
                    let barcode_data = self.decode_text(&data[3..3 + k]);
                    Ok(Some((EscPosCommand::PrintBarcode { system: m, data: barcode_data }, 3 + k + 1)))
                } else if (65..=73).contains(&m) {
                    // Format 2: GS k m n d1...dn
                    if data.len() < 4 { return Ok(None); }
                    let n = data[3] as usize;
                    if data.len() < 4 + n { return Ok(None); }
                    let barcode_data = self.decode_text(&data[4..4 + n]);
                    Ok(Some((EscPosCommand::PrintBarcode { system: m, data: barcode_data }, 4 + n)))
                } else {
                    Ok(Some((EscPosCommand::Unknown(data[..2].to_vec()), 2)))
                }
            }

            _ => {
                Ok(Some((EscPosCommand::Unknown(data[..2].to_vec()), 2)))
            }
        }
    }

    /// Decode text according to current codepage (simplified to CP437 for now)
    fn decode_text(&self, data: &[u8]) -> String {
        // For now, we use UTF-8 lossy as a base, 
        // but we can add full CP437 mapping here if needed.
        // ESC/POS usually defaults to CP437.
        String::from_utf8_lossy(data).to_string()
    }
}

impl Default for EscPosParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EscPosParser {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
        }
    }
}
