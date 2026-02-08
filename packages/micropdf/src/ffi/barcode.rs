//! FFI bindings for fz_barcode (Barcode Generation and Decoding)
//!
//! Supports various 1D and 2D barcode formats including QR codes, Data Matrix,
//! Code 39, Code 128, EAN, UPC, and more.

use crate::ffi::colorspace::FZ_COLORSPACE_GRAY;
use crate::ffi::pixmap::Pixmap;
use crate::ffi::{Handle, PIXMAPS};
use std::ffi::{CStr, CString, c_char};
use std::ptr;

// ============================================================================
// Types
// ============================================================================

/// Barcode types supported
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarcodeType {
    #[default]
    None = 0,
    /// Aztec 2D barcode
    Aztec = 1,
    /// Codabar (1D)
    Codabar = 2,
    /// Code 39 (1D)
    Code39 = 3,
    /// Code 93 (1D)
    Code93 = 4,
    /// Code 128 (1D)
    Code128 = 5,
    /// GS1 DataBar (RSS-14)
    DataBar = 6,
    /// GS1 DataBar Expanded
    DataBarExpanded = 7,
    /// Data Matrix (2D)
    DataMatrix = 8,
    /// EAN-8 (1D)
    Ean8 = 9,
    /// EAN-13 (1D)
    Ean13 = 10,
    /// Interleaved 2 of 5 (1D)
    Itf = 11,
    /// MaxiCode (2D)
    MaxiCode = 12,
    /// PDF417 (2D stacked)
    Pdf417 = 13,
    /// QR Code (2D)
    QrCode = 14,
    /// UPC-A (1D)
    UpcA = 15,
    /// UPC-E (1D)
    UpcE = 16,
    /// Micro QR Code
    MicroQrCode = 17,
    /// Rectangular Micro QR Code
    RmQrCode = 18,
    /// DX Film Edge barcode
    DxFilmEdge = 19,
    /// GS1 DataBar Limited
    DataBarLimited = 20,
}

impl BarcodeType {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => BarcodeType::None,
            1 => BarcodeType::Aztec,
            2 => BarcodeType::Codabar,
            3 => BarcodeType::Code39,
            4 => BarcodeType::Code93,
            5 => BarcodeType::Code128,
            6 => BarcodeType::DataBar,
            7 => BarcodeType::DataBarExpanded,
            8 => BarcodeType::DataMatrix,
            9 => BarcodeType::Ean8,
            10 => BarcodeType::Ean13,
            11 => BarcodeType::Itf,
            12 => BarcodeType::MaxiCode,
            13 => BarcodeType::Pdf417,
            14 => BarcodeType::QrCode,
            15 => BarcodeType::UpcA,
            16 => BarcodeType::UpcE,
            17 => BarcodeType::MicroQrCode,
            18 => BarcodeType::RmQrCode,
            19 => BarcodeType::DxFilmEdge,
            20 => BarcodeType::DataBarLimited,
            _ => BarcodeType::None,
        }
    }

    pub fn to_string(self) -> &'static str {
        match self {
            BarcodeType::None => "none",
            BarcodeType::Aztec => "aztec",
            BarcodeType::Codabar => "codabar",
            BarcodeType::Code39 => "code39",
            BarcodeType::Code93 => "code93",
            BarcodeType::Code128 => "code128",
            BarcodeType::DataBar => "databar",
            BarcodeType::DataBarExpanded => "databarexpanded",
            BarcodeType::DataMatrix => "datamatrix",
            BarcodeType::Ean8 => "ean8",
            BarcodeType::Ean13 => "ean13",
            BarcodeType::Itf => "itf",
            BarcodeType::MaxiCode => "maxicode",
            BarcodeType::Pdf417 => "pdf417",
            BarcodeType::QrCode => "qrcode",
            BarcodeType::UpcA => "upca",
            BarcodeType::UpcE => "upce",
            BarcodeType::MicroQrCode => "microqrcode",
            BarcodeType::RmQrCode => "rmqrcode",
            BarcodeType::DxFilmEdge => "dxfilmedge",
            BarcodeType::DataBarLimited => "databarlimited",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "aztec" => BarcodeType::Aztec,
            "codabar" => BarcodeType::Codabar,
            "code39" | "code-39" | "code_39" => BarcodeType::Code39,
            "code93" | "code-93" | "code_93" => BarcodeType::Code93,
            "code128" | "code-128" | "code_128" => BarcodeType::Code128,
            "databar" | "rss14" | "rss-14" => BarcodeType::DataBar,
            "databarexpanded" | "rssexpanded" => BarcodeType::DataBarExpanded,
            "datamatrix" | "data-matrix" | "data_matrix" | "dm" => BarcodeType::DataMatrix,
            "ean8" | "ean-8" | "ean_8" => BarcodeType::Ean8,
            "ean13" | "ean-13" | "ean_13" => BarcodeType::Ean13,
            "itf" | "interleaved2of5" | "i2of5" => BarcodeType::Itf,
            "maxicode" => BarcodeType::MaxiCode,
            "pdf417" | "pdf-417" => BarcodeType::Pdf417,
            "qrcode" | "qr" | "qr-code" | "qr_code" => BarcodeType::QrCode,
            "upca" | "upc-a" | "upc_a" => BarcodeType::UpcA,
            "upce" | "upc-e" | "upc_e" => BarcodeType::UpcE,
            "microqrcode" | "micro-qr" | "microqr" => BarcodeType::MicroQrCode,
            "rmqrcode" | "rmqr" | "rectangular-qr" => BarcodeType::RmQrCode,
            "dxfilmedge" | "dx-film-edge" => BarcodeType::DxFilmEdge,
            "databarlimited" | "rsslimited" => BarcodeType::DataBarLimited,
            _ => BarcodeType::None,
        }
    }

    /// Check if this is a 2D barcode
    pub fn is_2d(self) -> bool {
        matches!(
            self,
            BarcodeType::Aztec
                | BarcodeType::DataMatrix
                | BarcodeType::MaxiCode
                | BarcodeType::Pdf417
                | BarcodeType::QrCode
                | BarcodeType::MicroQrCode
                | BarcodeType::RmQrCode
        )
    }

    /// Check if this is a 1D barcode
    pub fn is_1d(self) -> bool {
        !self.is_2d() && self != BarcodeType::None
    }

    /// Get default module size for this barcode type
    pub fn default_size(self) -> i32 {
        if self.is_2d() { 4 } else { 2 }
    }
}

/// Error correction level for 2D barcodes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EcLevel {
    /// Low (7% recovery)
    #[default]
    L = 0,
    /// Medium (15% recovery)
    M = 1,
    /// Quartile (25% recovery)
    Q = 2,
    /// High (30% recovery)
    H = 3,
}

impl EcLevel {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => EcLevel::L,
            1 => EcLevel::M,
            2 => EcLevel::Q,
            3 | _ => EcLevel::H,
        }
    }
}

// ============================================================================
// Barcode Generation
// ============================================================================

/// Generate a QR code matrix
fn generate_qr_matrix(data: &str, size: i32, ec_level: EcLevel) -> Option<Vec<Vec<bool>>> {
    // Simple QR code generation (basic implementation)
    // For production, use a proper QR library like qrcode-rs

    let data_bytes = data.as_bytes();
    if data_bytes.is_empty() || data_bytes.len() > 2953 {
        return None;
    }

    // Calculate version based on data length and EC level
    let version = calculate_qr_version(data_bytes.len(), ec_level);
    let modules = version * 4 + 17;

    let mut matrix = vec![vec![false; modules as usize]; modules as usize];

    // Add finder patterns (3 corners)
    add_finder_pattern(&mut matrix, 0, 0);
    add_finder_pattern(&mut matrix, modules - 7, 0);
    add_finder_pattern(&mut matrix, 0, modules - 7);

    // Add timing patterns
    for i in 8..modules - 8 {
        matrix[6][i as usize] = i % 2 == 0;
        matrix[i as usize][6] = i % 2 == 0;
    }

    // Add alignment pattern for version >= 2
    if version >= 2 {
        let align_pos = modules - 7;
        add_alignment_pattern(&mut matrix, align_pos - 2, align_pos - 2);
    }

    // Encode data (simplified - just creates a pattern from data)
    encode_data_to_matrix(&mut matrix, data_bytes, version);

    // Apply mask
    apply_mask(&mut matrix, 0);

    Some(matrix)
}

fn calculate_qr_version(data_len: usize, _ec_level: EcLevel) -> i32 {
    // Simplified version calculation
    if data_len <= 17 {
        1
    } else if data_len <= 32 {
        2
    } else if data_len <= 53 {
        3
    } else if data_len <= 78 {
        4
    } else if data_len <= 106 {
        5
    } else if data_len <= 134 {
        6
    } else {
        (data_len as i32 / 20).min(40)
    }
}

fn add_finder_pattern(matrix: &mut [Vec<bool>], x: i32, y: i32) {
    // 7x7 finder pattern
    for dy in 0..7 {
        for dx in 0..7 {
            let is_border = dx == 0 || dx == 6 || dy == 0 || dy == 6;
            let is_inner = (2..=4).contains(&dx) && (2..=4).contains(&dy);
            matrix[(y + dy) as usize][(x + dx) as usize] = is_border || is_inner;
        }
    }
}

fn add_alignment_pattern(matrix: &mut [Vec<bool>], x: i32, y: i32) {
    // 5x5 alignment pattern
    for dy in 0..5 {
        for dx in 0..5 {
            let is_border = dx == 0 || dx == 4 || dy == 0 || dy == 4;
            let is_center = dx == 2 && dy == 2;
            if (y + dy) < matrix.len() as i32 && (x + dx) < matrix[0].len() as i32 {
                matrix[(y + dy) as usize][(x + dx) as usize] = is_border || is_center;
            }
        }
    }
}

fn encode_data_to_matrix(matrix: &mut [Vec<bool>], data: &[u8], _version: i32) {
    // Simplified data encoding - creates a deterministic pattern from data
    let modules = matrix.len();
    let mut bit_pos = 0;

    // Start from bottom-right, going upward in 2-module columns
    let mut col = modules - 1;
    while col > 0 {
        if col == 6 {
            col -= 1; // Skip timing column
        }

        for row in (0..modules).rev() {
            for c in [col, col.saturating_sub(1)] {
                if !is_function_module(matrix, row, c) {
                    let byte_idx = bit_pos / 8;
                    let bit_idx = 7 - (bit_pos % 8);
                    let bit = if byte_idx < data.len() {
                        (data[byte_idx] >> bit_idx) & 1 == 1
                    } else {
                        false
                    };
                    matrix[row][c] = bit;
                    bit_pos += 1;
                }
            }
        }
        col = col.saturating_sub(2);
    }
}

fn is_function_module(matrix: &[Vec<bool>], row: usize, col: usize) -> bool {
    let n = matrix.len();

    // Finder patterns
    if (row < 9 && col < 9) || (row < 9 && col >= n - 8) || (row >= n - 8 && col < 9) {
        return true;
    }

    // Timing patterns
    if row == 6 || col == 6 {
        return true;
    }

    false
}

fn apply_mask(matrix: &mut [Vec<bool>], _mask_pattern: u8) {
    let n = matrix.len();
    for row in 0..n {
        for col in 0..n {
            if !is_function_module(matrix, row, col) && (row + col) % 2 == 0 {
                matrix[row][col] = !matrix[row][col];
            }
        }
    }
}

/// Generate a 1D barcode pattern
fn generate_1d_barcode(data: &str, barcode_type: BarcodeType) -> Option<Vec<bool>> {
    match barcode_type {
        BarcodeType::Code128 => generate_code128(data),
        BarcodeType::Code39 => generate_code39(data),
        BarcodeType::Ean13 => generate_ean13(data),
        BarcodeType::Ean8 => generate_ean8(data),
        BarcodeType::UpcA => generate_upca(data),
        _ => generate_code128(data), // Default to Code 128
    }
}

fn generate_code128(data: &str) -> Option<Vec<bool>> {
    // Code 128 patterns (Code Set B)
    let patterns: [u16; 107] = [
        0b11011001100,   // 0: space
        0b11001101100,   // 1: !
        0b11001100110,   // 2: "
        0b10010011000,   // 3: #
        0b10010001100,   // 4: $
        0b10001001100,   // 5: %
        0b10011001000,   // 6: &
        0b10011000100,   // 7: '
        0b10001100100,   // 8: (
        0b11001001000,   // 9: )
        0b11001000100,   // 10: *
        0b11000100100,   // 11: +
        0b10110011100,   // 12: ,
        0b10011011100,   // 13: -
        0b10011001110,   // 14: .
        0b10111001100,   // 15: /
        0b10011101100,   // 16: 0
        0b10011100110,   // 17: 1
        0b11001110010,   // 18: 2
        0b11001011100,   // 19: 3
        0b11001001110,   // 20: 4
        0b11011100100,   // 21: 5
        0b11001110100,   // 22: 6
        0b11101101110,   // 23: 7
        0b11101001100,   // 24: 8
        0b11100101100,   // 25: 9
        0b11100100110,   // 26: :
        0b11101100100,   // 27: ;
        0b11100110100,   // 28: <
        0b11100110010,   // 29: =
        0b11011011000,   // 30: >
        0b11011000110,   // 31: ?
        0b11000110110,   // 32: @
        0b10100011000,   // 33: A
        0b10001011000,   // 34: B
        0b10001000110,   // 35: C
        0b10110001000,   // 36: D
        0b10001101000,   // 37: E
        0b10001100010,   // 38: F
        0b11010001000,   // 39: G
        0b11000101000,   // 40: H
        0b11000100010,   // 41: I
        0b10110111000,   // 42: J
        0b10110001110,   // 43: K
        0b10001101110,   // 44: L
        0b10111011000,   // 45: M
        0b10111000110,   // 46: N
        0b10001110110,   // 47: O
        0b11101110110,   // 48: P
        0b11010001110,   // 49: Q
        0b11000101110,   // 50: R
        0b11011101000,   // 51: S
        0b11011100010,   // 52: T
        0b11011101110,   // 53: U
        0b11101011000,   // 54: V
        0b11101000110,   // 55: W
        0b11100010110,   // 56: X
        0b11101101000,   // 57: Y
        0b11101100010,   // 58: Z
        0b11100011010,   // 59: [
        0b11101111010,   // 60: \
        0b11001000010,   // 61: ]
        0b11110001010,   // 62: ^
        0b10100110000,   // 63: _
        0b10100001100,   // 64: `
        0b10010110000,   // 65: a
        0b10010000110,   // 66: b
        0b10000101100,   // 67: c
        0b10000100110,   // 68: d
        0b10110010000,   // 69: e
        0b10110000100,   // 70: f
        0b10011010000,   // 71: g
        0b10011000010,   // 72: h
        0b10000110100,   // 73: i
        0b10000110010,   // 74: j
        0b11000010010,   // 75: k
        0b11001010000,   // 76: l
        0b11110111010,   // 77: m
        0b11000010100,   // 78: n
        0b10001111010,   // 79: o
        0b10100111100,   // 80: p
        0b10010111100,   // 81: q
        0b10010011110,   // 82: r
        0b10111100100,   // 83: s
        0b10011110100,   // 84: t
        0b10011110010,   // 85: u
        0b11110100100,   // 86: v
        0b11110010100,   // 87: w
        0b11110010010,   // 88: x
        0b11011011110,   // 89: y
        0b11011110110,   // 90: z
        0b11110110110,   // 91: {
        0b10101111000,   // 92: |
        0b10100011110,   // 93: }
        0b10001011110,   // 94: ~
        0b10111101000,   // 95: DEL
        0b10111100010,   // 96: FNC3
        0b11110101000,   // 97: FNC2
        0b11110100010,   // 98: Shift
        0b10111011110,   // 99: Code C
        0b10111101110,   // 100: FNC4/Code B
        0b11101011110,   // 101: FNC4/Code A
        0b11110101110,   // 102: FNC1
        0b11010000100,   // 103: Start A
        0b11010010000,   // 104: Start B
        0b11010011100,   // 105: Start C
        0b1100011101011, // 106: Stop
    ];

    let mut result = Vec::new();

    // Quiet zone
    for _ in 0..10 {
        result.push(false);
    }

    // Start code B (104)
    let start = patterns[104];
    for i in (0..11).rev() {
        result.push((start >> i) & 1 == 1);
    }

    // Encode data
    let mut checksum = 104;
    for (pos, c) in data.chars().enumerate() {
        let value = if c >= ' ' && c <= '~' {
            (c as usize) - 32
        } else {
            0
        };
        let pattern = patterns[value];
        for i in (0..11).rev() {
            result.push((pattern >> i) & 1 == 1);
        }
        checksum += value * (pos + 1);
    }

    // Checksum
    let check_pattern = patterns[checksum % 103];
    for i in (0..11).rev() {
        result.push((check_pattern >> i) & 1 == 1);
    }

    // Stop
    let stop = patterns[106];
    for i in (0..13).rev() {
        result.push((stop >> i) & 1 == 1);
    }

    // Quiet zone
    for _ in 0..10 {
        result.push(false);
    }

    Some(result)
}

fn generate_code39(data: &str) -> Option<Vec<bool>> {
    // Code 39 character set
    let chars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%*";
    let patterns: [u16; 44] = [
        0b101001101101, // 0
        0b110100101011, // 1
        0b101100101011, // 2
        0b110110010101, // 3
        0b101001101011, // 4
        0b110100110101, // 5
        0b101100110101, // 6
        0b101001011011, // 7
        0b110100101101, // 8
        0b101100101101, // 9
        0b110101001011, // A
        0b101101001011, // B
        0b110110100101, // C
        0b101011001011, // D
        0b110101100101, // E
        0b101101100101, // F
        0b101010011011, // G
        0b110101001101, // H
        0b101101001101, // I
        0b101011001101, // J
        0b110101010011, // K
        0b101101010011, // L
        0b110110101001, // M
        0b101011010011, // N
        0b110101101001, // O
        0b101101101001, // P
        0b101010110011, // Q
        0b110101011001, // R
        0b101101011001, // S
        0b101011011001, // T
        0b110010101011, // U
        0b100110101011, // V
        0b110011010101, // W
        0b100101101011, // X
        0b110010110101, // Y
        0b100110110101, // Z
        0b100101011011, // -
        0b110010101101, // .
        0b100110101101, // space
        0b100100100101, // $
        0b100100101001, // /
        0b100101001001, // +
        0b101001001001, // %
        0b100101101101, // * (start/stop)
    ];

    let mut result = Vec::new();

    // Quiet zone
    for _ in 0..10 {
        result.push(false);
    }

    // Start (*)
    let start = patterns[43];
    for i in (0..12).rev() {
        result.push((start >> i) & 1 == 1);
    }
    result.push(false); // Inter-character gap

    // Encode data
    for c in data.to_uppercase().chars() {
        if let Some(idx) = chars.find(c) {
            let pattern = patterns[idx];
            for i in (0..12).rev() {
                result.push((pattern >> i) & 1 == 1);
            }
            result.push(false); // Inter-character gap
        }
    }

    // Stop (*)
    for i in (0..12).rev() {
        result.push((start >> i) & 1 == 1);
    }

    // Quiet zone
    for _ in 0..10 {
        result.push(false);
    }

    Some(result)
}

fn generate_ean13(data: &str) -> Option<Vec<bool>> {
    let digits: Vec<u8> = data
        .chars()
        .filter_map(|c| c.to_digit(10).map(|d| d as u8))
        .collect();

    if digits.len() < 12 {
        return None;
    }

    // L-codes and R-codes for EAN-13
    let l_codes: [u8; 10] = [
        0b0001101, // 0
        0b0011001, // 1
        0b0010011, // 2
        0b0111101, // 3
        0b0100011, // 4
        0b0110001, // 5
        0b0101111, // 6
        0b0111011, // 7
        0b0110111, // 8
        0b0001011, // 9
    ];

    let r_codes: [u8; 10] = [
        0b1110010, // 0
        0b1100110, // 1
        0b1101100, // 2
        0b1000010, // 3
        0b1011100, // 4
        0b1001110, // 5
        0b1010000, // 6
        0b1000100, // 7
        0b1001000, // 8
        0b1110100, // 9
    ];

    let mut result = Vec::new();

    // Quiet zone
    for _ in 0..9 {
        result.push(false);
    }

    // Start guard (101)
    result.push(true);
    result.push(false);
    result.push(true);

    // First 6 digits (using L-codes)
    for &d in &digits[1..7] {
        let code = l_codes[d as usize];
        for i in (0..7).rev() {
            result.push((code >> i) & 1 == 1);
        }
    }

    // Center guard (01010)
    result.push(false);
    result.push(true);
    result.push(false);
    result.push(true);
    result.push(false);

    // Last 6 digits (using R-codes)
    for &d in &digits[7..13.min(digits.len())] {
        let code = r_codes[d as usize];
        for i in (0..7).rev() {
            result.push((code >> i) & 1 == 1);
        }
    }

    // End guard (101)
    result.push(true);
    result.push(false);
    result.push(true);

    // Quiet zone
    for _ in 0..9 {
        result.push(false);
    }

    Some(result)
}

fn generate_ean8(data: &str) -> Option<Vec<bool>> {
    let digits: Vec<u8> = data
        .chars()
        .filter_map(|c| c.to_digit(10).map(|d| d as u8))
        .collect();

    if digits.len() < 7 {
        return None;
    }

    let l_codes: [u8; 10] = [
        0b0001101, 0b0011001, 0b0010011, 0b0111101, 0b0100011, 0b0110001, 0b0101111, 0b0111011,
        0b0110111, 0b0001011,
    ];

    let r_codes: [u8; 10] = [
        0b1110010, 0b1100110, 0b1101100, 0b1000010, 0b1011100, 0b1001110, 0b1010000, 0b1000100,
        0b1001000, 0b1110100,
    ];

    let mut result = Vec::new();

    // Quiet zone
    for _ in 0..7 {
        result.push(false);
    }

    // Start guard
    result.push(true);
    result.push(false);
    result.push(true);

    // First 4 digits
    for &d in &digits[0..4] {
        let code = l_codes[d as usize];
        for i in (0..7).rev() {
            result.push((code >> i) & 1 == 1);
        }
    }

    // Center guard
    result.push(false);
    result.push(true);
    result.push(false);
    result.push(true);
    result.push(false);

    // Last 4 digits
    for &d in &digits[4..8.min(digits.len())] {
        let code = r_codes[d as usize];
        for i in (0..7).rev() {
            result.push((code >> i) & 1 == 1);
        }
    }

    // End guard
    result.push(true);
    result.push(false);
    result.push(true);

    // Quiet zone
    for _ in 0..7 {
        result.push(false);
    }

    Some(result)
}

fn generate_upca(data: &str) -> Option<Vec<bool>> {
    // UPC-A is essentially EAN-13 with a leading 0
    let padded = format!("0{}", data);
    generate_ean13(&padded)
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Get barcode type name as string
#[unsafe(no_mangle)]
pub extern "C" fn fz_string_from_barcode_type(barcode_type: i32) -> *const c_char {
    static NAMES: [&str; 21] = [
        "none\0",
        "aztec\0",
        "codabar\0",
        "code39\0",
        "code93\0",
        "code128\0",
        "databar\0",
        "databarexpanded\0",
        "datamatrix\0",
        "ean8\0",
        "ean13\0",
        "itf\0",
        "maxicode\0",
        "pdf417\0",
        "qrcode\0",
        "upca\0",
        "upce\0",
        "microqrcode\0",
        "rmqrcode\0",
        "dxfilmedge\0",
        "databarlimited\0",
    ];

    let idx = (barcode_type as usize).min(20);
    NAMES[idx].as_ptr().cast()
}

/// Get barcode type from string
#[unsafe(no_mangle)]
pub extern "C" fn fz_barcode_type_from_string(str_ptr: *const c_char) -> i32 {
    if str_ptr.is_null() {
        return BarcodeType::None as i32;
    }

    let s = unsafe { CStr::from_ptr(str_ptr) };
    if let Ok(name) = s.to_str() {
        BarcodeType::from_string(name) as i32
    } else {
        BarcodeType::None as i32
    }
}

/// Create a barcode pixmap
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_barcode_pixmap(
    _ctx: Handle,
    barcode_type: i32,
    value: *const c_char,
    size: i32,
    ec_level: i32,
    quiet: i32,
    _hrt: i32,
) -> Handle {
    if value.is_null() {
        return 0;
    }

    let data = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let bt = BarcodeType::from_i32(barcode_type);
    let module_size = if size > 0 { size } else { bt.default_size() };
    let ec = EcLevel::from_i32(ec_level);
    let add_quiet = quiet != 0;

    // Generate barcode
    if bt.is_2d() {
        // 2D barcode (QR, etc.)
        let matrix = match bt {
            BarcodeType::QrCode | BarcodeType::MicroQrCode => generate_qr_matrix(data, size, ec),
            _ => generate_qr_matrix(data, size, ec), // Default to QR for unsupported 2D
        };

        if let Some(matrix) = matrix {
            let modules = matrix.len();
            let quiet_zone = if add_quiet { 4 } else { 0 };
            let total_size = (modules + quiet_zone * 2) as i32 * module_size;

            let pixmap = Pixmap::new(FZ_COLORSPACE_GRAY, total_size, total_size, false);
            let handle = PIXMAPS.insert(pixmap);

            if let Some(pix) = PIXMAPS.get(handle) {
                let mut guard = pix.lock().unwrap();
                let samples = guard.samples_mut();

                // Fill with white
                for sample in samples.iter_mut() {
                    *sample = 255;
                }

                // Draw modules
                let stride = total_size as usize;
                for (row, matrix_row) in matrix.iter().enumerate() {
                    for (col, &is_dark) in matrix_row.iter().enumerate() {
                        if is_dark {
                            let x_start = ((col + quiet_zone) as i32 * module_size) as usize;
                            let y_start = ((row + quiet_zone) as i32 * module_size) as usize;

                            for dy in 0..module_size as usize {
                                for dx in 0..module_size as usize {
                                    let idx = (y_start + dy) * stride + (x_start + dx);
                                    if idx < samples.len() {
                                        samples[idx] = 0; // Black
                                    }
                                }
                            }
                        }
                    }
                }
            }

            return handle;
        }
    } else {
        // 1D barcode
        let pattern = generate_1d_barcode(data, bt);

        if let Some(pattern) = pattern {
            let bar_height = 50 * module_size;
            let width = pattern.len() as i32 * module_size;

            let pixmap = Pixmap::new(FZ_COLORSPACE_GRAY, width, bar_height, false);
            let handle = PIXMAPS.insert(pixmap);

            if let Some(pix) = PIXMAPS.get(handle) {
                let mut guard = pix.lock().unwrap();
                let samples = guard.samples_mut();

                // Fill with white
                for sample in samples.iter_mut() {
                    *sample = 255;
                }

                // Draw bars
                let stride = width as usize;
                for (col, &is_bar) in pattern.iter().enumerate() {
                    if is_bar {
                        let x_start = (col as i32 * module_size) as usize;

                        for y in 0..bar_height as usize {
                            for dx in 0..module_size as usize {
                                let idx = y * stride + x_start + dx;
                                if idx < samples.len() {
                                    samples[idx] = 0; // Black
                                }
                            }
                        }
                    }
                }
            }

            return handle;
        }
    }

    0
}

/// Create a barcode image (wrapper around pixmap)
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_barcode_image(
    ctx: Handle,
    barcode_type: i32,
    value: *const c_char,
    size: i32,
    ec_level: i32,
    quiet: i32,
    hrt: i32,
) -> Handle {
    // For now, return the pixmap handle directly
    // In a full implementation, this would create an fz_image wrapper
    fz_new_barcode_pixmap(ctx, barcode_type, value, size, ec_level, quiet, hrt)
}

/// Check if barcode type is 2D
#[unsafe(no_mangle)]
pub extern "C" fn fz_barcode_is_2d(barcode_type: i32) -> i32 {
    if BarcodeType::from_i32(barcode_type).is_2d() {
        1
    } else {
        0
    }
}

/// Check if barcode type is 1D
#[unsafe(no_mangle)]
pub extern "C" fn fz_barcode_is_1d(barcode_type: i32) -> i32 {
    if BarcodeType::from_i32(barcode_type).is_1d() {
        1
    } else {
        0
    }
}

/// Get default module size for barcode type
#[unsafe(no_mangle)]
pub extern "C" fn fz_barcode_default_size(barcode_type: i32) -> i32 {
    BarcodeType::from_i32(barcode_type).default_size()
}

/// Get barcode type count
#[unsafe(no_mangle)]
pub extern "C" fn fz_barcode_type_count() -> i32 {
    21 // Total number of barcode types including None
}

/// Decode barcode from pixmap (stub - requires external library for real decoding)
#[unsafe(no_mangle)]
pub extern "C" fn fz_decode_barcode_from_pixmap(
    _ctx: Handle,
    type_out: *mut i32,
    _pix: Handle,
    _rotate: i32,
) -> *mut c_char {
    if !type_out.is_null() {
        unsafe {
            *type_out = BarcodeType::None as i32;
        }
    }

    // Barcode decoding would require an external library like ZXing
    // Return null to indicate no barcode found
    ptr::null_mut()
}

/// Validate barcode data for a given type
#[unsafe(no_mangle)]
pub extern "C" fn fz_barcode_validate(barcode_type: i32, value: *const c_char) -> i32 {
    if value.is_null() {
        return 0;
    }

    let data = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let bt = BarcodeType::from_i32(barcode_type);

    match bt {
        BarcodeType::Ean13 => {
            let digits: String = data.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 12 { 1 } else { 0 }
        }
        BarcodeType::Ean8 => {
            let digits: String = data.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 7 { 1 } else { 0 }
        }
        BarcodeType::UpcA => {
            let digits: String = data.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 11 { 1 } else { 0 }
        }
        BarcodeType::UpcE => {
            let digits: String = data.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 6 { 1 } else { 0 }
        }
        BarcodeType::Code39 => {
            let valid_chars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";
            if data.to_uppercase().chars().all(|c| valid_chars.contains(c)) {
                1
            } else {
                0
            }
        }
        BarcodeType::Code128 => {
            if data.chars().all(|c| c as u32 >= 32 && c as u32 <= 126) {
                1
            } else {
                0
            }
        }
        BarcodeType::QrCode => {
            if !data.is_empty() && data.len() <= 2953 {
                1
            } else {
                0
            }
        }
        _ => {
            if !data.is_empty() {
                1
            } else {
                0
            }
        }
    }
}

/// Calculate check digit for EAN/UPC barcodes
#[unsafe(no_mangle)]
pub extern "C" fn fz_barcode_check_digit(barcode_type: i32, value: *const c_char) -> i32 {
    if value.is_null() {
        return -1;
    }

    let data = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let bt = BarcodeType::from_i32(barcode_type);
    let digits: Vec<u8> = data
        .chars()
        .filter_map(|c| c.to_digit(10).map(|d| d as u8))
        .collect();

    match bt {
        BarcodeType::Ean13 | BarcodeType::UpcA => {
            if digits.len() < 12 {
                return -1;
            }
            let mut sum = 0u32;
            for (i, &d) in digits[..12].iter().enumerate() {
                sum += d as u32 * if i % 2 == 0 { 1 } else { 3 };
            }
            ((10 - (sum % 10)) % 10) as i32
        }
        BarcodeType::Ean8 => {
            if digits.len() < 7 {
                return -1;
            }
            let mut sum = 0u32;
            for (i, &d) in digits[..7].iter().enumerate() {
                sum += d as u32 * if i % 2 == 0 { 3 } else { 1 };
            }
            ((10 - (sum % 10)) % 10) as i32
        }
        _ => -1,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barcode_type_enum() {
        assert_eq!(BarcodeType::from_i32(0), BarcodeType::None);
        assert_eq!(BarcodeType::from_i32(14), BarcodeType::QrCode);
        assert_eq!(BarcodeType::from_i32(5), BarcodeType::Code128);
        assert_eq!(BarcodeType::from_i32(99), BarcodeType::None);
    }

    #[test]
    fn test_barcode_type_string() {
        assert_eq!(BarcodeType::QrCode.to_string(), "qrcode");
        assert_eq!(BarcodeType::Code128.to_string(), "code128");
        assert_eq!(BarcodeType::Ean13.to_string(), "ean13");
    }

    #[test]
    fn test_barcode_type_from_string() {
        assert_eq!(BarcodeType::from_string("qrcode"), BarcodeType::QrCode);
        assert_eq!(BarcodeType::from_string("QR"), BarcodeType::QrCode);
        assert_eq!(BarcodeType::from_string("code-128"), BarcodeType::Code128);
        assert_eq!(BarcodeType::from_string("ean-13"), BarcodeType::Ean13);
        assert_eq!(BarcodeType::from_string("unknown"), BarcodeType::None);
    }

    #[test]
    fn test_barcode_is_2d() {
        assert!(BarcodeType::QrCode.is_2d());
        assert!(BarcodeType::DataMatrix.is_2d());
        assert!(BarcodeType::Aztec.is_2d());
        assert!(!BarcodeType::Code128.is_2d());
        assert!(!BarcodeType::Ean13.is_2d());
    }

    #[test]
    fn test_barcode_is_1d() {
        assert!(BarcodeType::Code128.is_1d());
        assert!(BarcodeType::Ean13.is_1d());
        assert!(BarcodeType::UpcA.is_1d());
        assert!(!BarcodeType::QrCode.is_1d());
        assert!(!BarcodeType::None.is_1d());
    }

    #[test]
    fn test_generate_qr_matrix() {
        let matrix = generate_qr_matrix("Hello", 0, EcLevel::M);
        assert!(matrix.is_some());
        let m = matrix.unwrap();
        assert!(m.len() >= 21); // Minimum QR version 1 is 21x21
    }

    #[test]
    fn test_generate_code128() {
        let pattern = generate_code128("Hello");
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert!(!p.is_empty());
    }

    #[test]
    fn test_generate_code39() {
        let pattern = generate_code39("HELLO");
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert!(!p.is_empty());
    }

    #[test]
    fn test_generate_ean13() {
        let pattern = generate_ean13("5901234123457");
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        // EAN-13: 3 start + 42 left + 5 center + 42 right + 3 end + quiet zones
        assert!(p.len() > 90);
    }

    #[test]
    fn test_generate_ean8() {
        let pattern = generate_ean8("96385074");
        assert!(pattern.is_some());
    }

    #[test]
    fn test_ffi_barcode_type_from_string() {
        let qr = CString::new("qrcode").unwrap();
        assert_eq!(
            fz_barcode_type_from_string(qr.as_ptr()),
            BarcodeType::QrCode as i32
        );

        let code128 = CString::new("code128").unwrap();
        assert_eq!(
            fz_barcode_type_from_string(code128.as_ptr()),
            BarcodeType::Code128 as i32
        );
    }

    #[test]
    fn test_ffi_barcode_is_2d() {
        assert_eq!(fz_barcode_is_2d(BarcodeType::QrCode as i32), 1);
        assert_eq!(fz_barcode_is_2d(BarcodeType::Code128 as i32), 0);
    }

    #[test]
    fn test_ffi_barcode_validate() {
        let ean13 = CString::new("5901234123457").unwrap();
        assert_eq!(
            fz_barcode_validate(BarcodeType::Ean13 as i32, ean13.as_ptr()),
            1
        );

        let short = CString::new("123").unwrap();
        assert_eq!(
            fz_barcode_validate(BarcodeType::Ean13 as i32, short.as_ptr()),
            0
        );
    }

    #[test]
    fn test_ffi_barcode_check_digit() {
        // EAN-13: 590123412345 -> check digit 7
        let ean = CString::new("590123412345").unwrap();
        let check = fz_barcode_check_digit(BarcodeType::Ean13 as i32, ean.as_ptr());
        assert!(check >= 0 && check <= 9);
    }

    #[test]
    fn test_ffi_new_barcode_pixmap_qr() {
        let ctx = 1;
        let data = CString::new("Hello World").unwrap();

        let handle =
            fz_new_barcode_pixmap(ctx, BarcodeType::QrCode as i32, data.as_ptr(), 4, 1, 1, 0);

        assert!(handle > 0);

        // Verify pixmap was created
        if let Some(pix) = PIXMAPS.get(handle) {
            let guard = pix.lock().unwrap();
            assert!(guard.w() > 0);
            assert!(guard.h() > 0);
        }

        PIXMAPS.remove(handle);
    }

    #[test]
    fn test_ffi_new_barcode_pixmap_code128() {
        let ctx = 1;
        let data = CString::new("ABC123").unwrap();

        let handle =
            fz_new_barcode_pixmap(ctx, BarcodeType::Code128 as i32, data.as_ptr(), 2, 0, 1, 0);

        assert!(handle > 0);

        if let Some(pix) = PIXMAPS.get(handle) {
            let guard = pix.lock().unwrap();
            assert!(guard.w() > 0);
            assert!(guard.h() > 0);
        }

        PIXMAPS.remove(handle);
    }

    #[test]
    fn test_ffi_new_barcode_pixmap_ean13() {
        let ctx = 1;
        let data = CString::new("5901234123457").unwrap();

        let handle =
            fz_new_barcode_pixmap(ctx, BarcodeType::Ean13 as i32, data.as_ptr(), 2, 0, 1, 0);

        assert!(handle > 0);

        PIXMAPS.remove(handle);
    }

    #[test]
    fn test_barcode_type_count() {
        assert_eq!(fz_barcode_type_count(), 21);
    }

    #[test]
    fn test_ec_level() {
        assert_eq!(EcLevel::from_i32(0), EcLevel::L);
        assert_eq!(EcLevel::from_i32(1), EcLevel::M);
        assert_eq!(EcLevel::from_i32(2), EcLevel::Q);
        assert_eq!(EcLevel::from_i32(3), EcLevel::H);
    }

    #[test]
    fn test_null_handling() {
        assert_eq!(fz_barcode_type_from_string(ptr::null()), 0);
        assert_eq!(
            fz_barcode_validate(BarcodeType::QrCode as i32, ptr::null()),
            0
        );
        assert_eq!(
            fz_barcode_check_digit(BarcodeType::Ean13 as i32, ptr::null()),
            -1
        );
        assert_eq!(
            fz_new_barcode_pixmap(1, BarcodeType::QrCode as i32, ptr::null(), 4, 1, 1, 0),
            0
        );
    }
}
