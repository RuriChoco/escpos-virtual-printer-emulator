use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscPosCommand {
    // Basic commands
    Text(String),
    NewLine,
    LineFeed,
    CarriageReturn,

    // Font commands
    SetFont(Font),
    SetFontSize(u32),

    // Formatting commands
    SetJustification(Justification),
    SetEmphasis(bool),
    SetUnderline(bool),
    SetItalic(bool),
    SetLineHeight(u32),

    // Print commands
    CutPaper,
    PrintImage(Vec<u8>),
    /// Raster bitmap with width (bytes per row) and height (rows)
    PrintRasterImage { width_bytes: u16, height: u16, data: Vec<u8> },
    /// Barcode: system, data
    PrintBarcode { system: u8, data: String },

    // Codepage selection (ESC t n)
    SetCodepage(u8),

    // Control commands
    InitializePrinter,
    /// Generate pulse (open drawer)
    GeneratePulse { pin: u8, t1: u8, t2: u8 },
    /// Print and feed n lines
    PrintAndFeed(u8),
    
    // Barcode settings
    SetBarcodeHeight(u8),
    SetBarcodeWidth(u8),
    SetHriFont(u8),
    SetHriPosition(u8),

    // Unknown commands
    Unknown(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Font {
    FontA,
    FontB,
    FontC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Justification {
    Left,
    Center,
    Right,
}
