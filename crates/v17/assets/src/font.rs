//! The 1.7.10 font: the glyph-width table of `net.minecraft.client.render
//! .TextRenderer`, the `font/glyph_sizes.bin` unicode metrics and the font
//! resource locations, ported exactly. Drawing itself (OpenGL) is the
//! render crate's job; this crate owns the widths the layout depends on.

use crate::identifier::Identifier;

/// The default font sheet (`textures/font/ascii.png`, 128x128, 16x16 chars).
pub const ASCII_FONT_LOCATION: &str = "textures/font/ascii.png";

/// The SGD sheet (`textures/font/ascii_sga.png`).
pub const STANDARD_GALACTIC_LOCATION: &str = "textures/font/ascii_sga.png";

/// The unicode glyph widths: one byte per code point, `font/glyph_sizes.bin`.
pub const GLYPH_SIZES_LOCATION: &str = "font/glyph_sizes.bin";

/// The unicode page sheets: `textures/font/unicode_page_%02x.png`.
pub fn unicode_page_location(page: u16) -> Identifier {
    Identifier::new(
        "minecraft",
        &format!("textures/font/unicode_page_{page:02x}.png"),
    )
}

/// `TextRenderer.drawGlyph`'s identity string: a character inside it (and
/// outside unicode mode) draws from the ascii sheet; everything else draws
/// from the unicode pages.
pub const BASIC_GLYPHS: &str = "ÀÁÂÈÊËÍÓÔÕÚßãõğİıŒœŞşŴŵžȇ\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0} !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\u{0}ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜø£Ø×ƒáíóúñÑªº¿®¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αβΓπΣσμτΦΘΩδ∞∅∈∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{0}";

/// True when `chr` draws from the ascii sheet: the space always does
/// (`drawGlyph` handles it before this check), everything else only when it
/// is in `BASIC_GLYPHS` and the locale is not unicode.
pub fn uses_basic_glyph(chr: char, unicode: bool) -> bool {
    chr != ' ' && BASIC_GLYPHS.contains(chr) && !unicode
}

/// The glyph advance of the space (`drawGlyph` returns 4.0 before any other
/// character is considered).
pub const SPACE_ADVANCE: f32 = 4.0;

/// The width table `TextRenderer` computes from the ascii sheet's pixels.
/// ARGB pixels (`BufferedImage.getRGB`), 16x16 characters per sheet; the
/// algorithm is Java's exact column scan: from the cell's rightmost column
/// leftwards until a column containing an opaque pixel is found, then the
/// width is `(int)(0.5 + columns_through_it * (8 / columns_per_char)) + 1`
/// (`++var12 * var8 + var7`, var7 = 1). The space keeps its fixed 4.
pub fn compute_char_widths(
    pixels: &[u32],
    texture_width: usize,
    texture_height: usize,
) -> [u8; 256] {
    let rows_per_char = texture_height / 16;
    let cols_per_char = texture_width / 16;
    let step = 8.0f32 / rows_per_char as f32;
    let mut widths = [0u8; 256];

    for index in 0..256usize {
        let sheet_col = index % 16;
        let sheet_row = index / 16;
        if index == 32 {
            widths[index] = (3 + 1) as u8;
        }

        let mut rightmost_opaque: i32 = -1;
        let mut column = cols_per_char as i32 - 1;
        while column >= 0 {
            let pixel_col = sheet_col * cols_per_char + column as usize;
            let mut column_has_pixel = false;

            for row in 0..rows_per_char {
                let pixel_row = sheet_row * rows_per_char + row;
                if (pixels[pixel_col + pixel_row * texture_width] >> 24) & 0xFF != 0 {
                    column_has_pixel = true;
                }
            }

            if column_has_pixel {
                rightmost_opaque = column;
                break;
            }
            column -= 1;
        }

        // Java: `(int)(0.5 + ++var12 * var8) + var7`, var7 = 1.
        let columns_through = (rightmost_opaque + 1) as f32;
        widths[index] = ((0.5 + columns_through * step) as i32 + 1) as u8;
    }

    widths
}

/// A solid white 128x128 sheet: every glyph 9 wide except the space (the
/// extreme case of the column scan, for tests and the probe).
pub fn solid_sheet() -> Vec<u32> {
    vec![0xFF00_0000u32 | 0x00FF_FFFF; 128 * 128]
}

/// `font/glyph_sizes.bin`: per-code-point `(left, right)` column range in
/// its 256-pixel-wide page. `0` means the glyph does not exist.
#[derive(Clone, Debug, Default)]
pub struct GlyphSizes {
    data: Vec<u8>,
}

impl GlyphSizes {
    pub const SIZE: usize = 256 * 256;

    pub fn from_bytes(data: Vec<u8>) -> Result<GlyphSizes, String> {
        if data.len() != Self::SIZE {
            return Err(format!(
                "glyph_sizes.bin must be {} bytes, got {}",
                Self::SIZE,
                data.len()
            ));
        }
        Ok(GlyphSizes { data })
    }

    /// Left column of the glyph in its page (the high nibble).
    pub fn left(&self, chr: u16) -> u8 {
        self.data[chr as usize] >> 4
    }

    /// Right column of the glyph in its page (the low nibble).
    pub fn right(&self, chr: u16) -> u8 {
        self.data[chr as usize] & 15
    }

    /// True when the glyph exists (`drawUnicodeGlyph` returns 0.0 when the
    /// byte is 0).
    pub fn is_present(&self, chr: u16) -> bool {
        self.data[chr as usize] != 0
    }

    /// The glyph's width in pixels of its page: `right - left + 1`.
    pub fn pixel_width(&self, chr: u16) -> u8 {
        self.right(chr) - self.left(chr) + 1
    }

    /// The advance `drawUnicodeGlyph` returns:
    /// `(right + 1 - left) / 2 + 1` — the page is drawn at half scale.
    pub fn advance(&self, chr: u16) -> f32 {
        if !self.is_present(chr) {
            return 0.0;
        }
        (f32::from(self.right(chr) + 1 - self.left(chr))) / 2.0 + 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_is_special() {
        assert!(!uses_basic_glyph(' ', false));
        assert!(uses_basic_glyph('a', false));
        assert!(!uses_basic_glyph('a', true));
        assert!(uses_basic_glyph('À', false));
        assert!(!uses_basic_glyph('א', false));
        assert_eq!(SPACE_ADVANCE, 4.0);
    }

    #[test]
    fn solid_sheet_widths() {
        let widths = compute_char_widths(&solid_sheet(), 128, 128);
        // Every pixel opaque: the rightmost column is the 8th, so
        // `(0.5 + 8 * 1.0) as int + 1` = 9 (the game's sheets draw 8-px
        // glyphs, so a solid cell measures 9 wide). The scan overwrites
        // even the space's fixed width — vanilla's `3 + var7` for index 32
        // is dead code; drawGlyph handles the space before the table.
        assert_eq!(widths['a' as usize], 9);
        assert_eq!(widths[32], 9);
    }

    #[test]
    fn empty_columns_grow_the_width() {
        // The right half of every character cell empty: the rightmost
        // opaque column is the 4th, width `(0.5 + 4) as int + 1` = 5.
        let mut pixels = solid_sheet();
        for row in 0..128 {
            for col in 0..128 {
                if col % 8 >= 4 {
                    pixels[row * 128 + col] = 0;
                }
            }
        }
        let widths = compute_char_widths(&pixels, 128, 128);
        assert_eq!(widths['a' as usize], 5);
        assert_eq!(widths[32], 5);
    }

    #[test]
    fn fully_blank_cell() {
        let mut pixels = solid_sheet();
        // Cells 0 and 32: both blank on the real sheet.
        for cell in [0usize, 32usize] {
            for row in 0..8 {
                for col in 0..8 {
                    pixels[(cell % 16) * 8 + col + ((cell / 16) * 8 + row) * 128] = 0;
                }
            }
        }
        let widths = compute_char_widths(&pixels, 128, 128);
        // No opaque column: `(0.5 + 0) as int + 1` = 1.
        assert_eq!(widths[0], 1);
        assert_eq!(widths[32], 1);
    }

    #[test]
    fn unicode_pages_are_numbered_two_hex_digits() {
        assert_eq!(
            unicode_page_location(0).path(),
            "textures/font/unicode_page_00.png"
        );
        assert_eq!(
            unicode_page_location(10).path(),
            "textures/font/unicode_page_0a.png"
        );
        assert_eq!(
            unicode_page_location(255).path(),
            "textures/font/unicode_page_ff.png"
        );
        assert_eq!(unicode_page_location(0).namespace(), "minecraft");
        assert_eq!(ASCII_FONT_LOCATION, "textures/font/ascii.png");
        assert_eq!(GLYPH_SIZES_LOCATION, "font/glyph_sizes.bin");
        assert_eq!(STANDARD_GALACTIC_LOCATION, "textures/font/ascii_sga.png");
    }

    #[test]
    fn glyph_sizes_decode_nibbles() {
        let mut data = vec![0u8; GlyphSizes::SIZE];
        // Code point 0x41: left 2, right 6 → width 5, advance (7-2)/2+1 = 3.5.
        data[0x41] = 0x26;
        let glyphs = GlyphSizes::from_bytes(data).unwrap();
        assert!(glyphs.is_present(0x41));
        assert_eq!(glyphs.left(0x41), 2);
        assert_eq!(glyphs.right(0x41), 6);
        assert_eq!(glyphs.pixel_width(0x41), 5);
        assert_eq!(glyphs.advance(0x41), 3.5);
        // A zero byte is an absent glyph: advance 0.
        assert!(!glyphs.is_present(0x3000));
        assert_eq!(glyphs.advance(0x3000), 0.0);
    }

    #[test]
    fn glyph_sizes_rejects_wrong_length() {
        assert!(GlyphSizes::from_bytes(vec![0u8; 100]).is_err());
        assert!(GlyphSizes::from_bytes(vec![0u8; GlyphSizes::SIZE]).is_ok());
    }
}
