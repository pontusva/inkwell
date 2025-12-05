//! Font metrics for accurate text measurement.
//!
//! Provides character width tables for built-in PDF fonts.
//! Widths are in 1/1000 of the font's em square (standard PDF units).

use std::collections::HashMap;
use std::sync::OnceLock;

/// Font metrics for a specific font variant
#[derive(Debug, Clone)]
pub struct FontMetrics {
    /// Character widths in 1/1000 em units
    widths: HashMap<char, u16>,
    /// Default width for unknown characters
    default_width: u16,
    /// Units per em (typically 1000 for Type1 fonts)
    pub units_per_em: u16,
    /// Ascender height in em units
    pub ascender: i16,
    /// Descender depth in em units (negative)
    pub descender: i16,
}

impl FontMetrics {
    /// Get the width of a character in em units (1/1000)
    pub fn char_width(&self, c: char) -> u16 {
        *self.widths.get(&c).unwrap_or(&self.default_width)
    }

    /// Get the width of a string in points
    pub fn string_width(&self, text: &str, font_size: f32) -> f32 {
        let total_units: u32 = text.chars().map(|c| self.char_width(c) as u32).sum();
        (total_units as f32 / self.units_per_em as f32) * font_size
    }

    /// Get the width of a single character in points
    pub fn char_width_pt(&self, c: char, font_size: f32) -> f32 {
        (self.char_width(c) as f32 / self.units_per_em as f32) * font_size
    }

    /// Get line height in points
    pub fn line_height(&self, font_size: f32, line_height_multiplier: f32) -> f32 {
        font_size * line_height_multiplier
    }
}

// ============================================================================
// HELVETICA METRICS
// ============================================================================

// Standard Helvetica character widths (from Adobe Font Metrics)
// Values are in 1/1000 of em square
fn helvetica_widths() -> HashMap<char, u16> {
    let mut m = HashMap::new();
    
    // Control characters and space
    m.insert(' ', 278);
    m.insert('!', 278);
    m.insert('"', 355);
    m.insert('#', 556);
    m.insert('$', 556);
    m.insert('%', 889);
    m.insert('&', 667);
    m.insert('\'', 191);
    m.insert('(', 333);
    m.insert(')', 333);
    m.insert('*', 389);
    m.insert('+', 584);
    m.insert(',', 278);
    m.insert('-', 333);
    m.insert('.', 278);
    m.insert('/', 278);
    
    // Digits
    m.insert('0', 556);
    m.insert('1', 556);
    m.insert('2', 556);
    m.insert('3', 556);
    m.insert('4', 556);
    m.insert('5', 556);
    m.insert('6', 556);
    m.insert('7', 556);
    m.insert('8', 556);
    m.insert('9', 556);
    
    // Punctuation
    m.insert(':', 278);
    m.insert(';', 278);
    m.insert('<', 584);
    m.insert('=', 584);
    m.insert('>', 584);
    m.insert('?', 556);
    m.insert('@', 1015);
    
    // Uppercase letters
    m.insert('A', 667);
    m.insert('B', 667);
    m.insert('C', 722);
    m.insert('D', 722);
    m.insert('E', 667);
    m.insert('F', 611);
    m.insert('G', 778);
    m.insert('H', 722);
    m.insert('I', 278);
    m.insert('J', 500);
    m.insert('K', 667);
    m.insert('L', 556);
    m.insert('M', 833);
    m.insert('N', 722);
    m.insert('O', 778);
    m.insert('P', 667);
    m.insert('Q', 778);
    m.insert('R', 722);
    m.insert('S', 667);
    m.insert('T', 611);
    m.insert('U', 722);
    m.insert('V', 667);
    m.insert('W', 944);
    m.insert('X', 667);
    m.insert('Y', 667);
    m.insert('Z', 611);
    
    // Brackets and special
    m.insert('[', 278);
    m.insert('\\', 278);
    m.insert(']', 278);
    m.insert('^', 469);
    m.insert('_', 556);
    m.insert('`', 333);
    
    // Lowercase letters
    m.insert('a', 556);
    m.insert('b', 556);
    m.insert('c', 500);
    m.insert('d', 556);
    m.insert('e', 556);
    m.insert('f', 278);
    m.insert('g', 556);
    m.insert('h', 556);
    m.insert('i', 222);
    m.insert('j', 222);
    m.insert('k', 500);
    m.insert('l', 222);
    m.insert('m', 833);
    m.insert('n', 556);
    m.insert('o', 556);
    m.insert('p', 556);
    m.insert('q', 556);
    m.insert('r', 333);
    m.insert('s', 500);
    m.insert('t', 278);
    m.insert('u', 556);
    m.insert('v', 500);
    m.insert('w', 722);
    m.insert('x', 500);
    m.insert('y', 500);
    m.insert('z', 500);
    
    // More punctuation
    m.insert('{', 334);
    m.insert('|', 260);
    m.insert('}', 334);
    m.insert('~', 584);
    
    // Extended ASCII / Latin-1 Supplement (common ones)
    m.insert('–', 556);  // en-dash
    m.insert('—', 1000); // em-dash
    m.insert('\u{2018}', 222);  // left single quote '
    m.insert('\u{2019}', 222);  // right single quote '
    m.insert('\u{201C}', 333);  // left double quote "
    m.insert('\u{201D}', 333);  // right double quote "
    m.insert('…', 1000); // ellipsis
    m.insert('€', 556);  // euro
    m.insert('£', 556);  // pound
    m.insert('¥', 556);  // yen
    m.insert('©', 737);  // copyright
    m.insert('®', 737);  // registered
    m.insert('™', 1000); // trademark
    m.insert('°', 400);  // degree
    m.insert('±', 584);  // plus-minus
    m.insert('×', 584);  // multiplication
    m.insert('÷', 584);  // division
    
    m
}

fn helvetica_bold_widths() -> HashMap<char, u16> {
    let mut m = HashMap::new();
    
    m.insert(' ', 278);
    m.insert('!', 333);
    m.insert('"', 474);
    m.insert('#', 556);
    m.insert('$', 556);
    m.insert('%', 889);
    m.insert('&', 722);
    m.insert('\'', 238);
    m.insert('(', 333);
    m.insert(')', 333);
    m.insert('*', 389);
    m.insert('+', 584);
    m.insert(',', 278);
    m.insert('-', 333);
    m.insert('.', 278);
    m.insert('/', 278);
    
    // Digits
    for c in '0'..='9' {
        m.insert(c, 556);
    }
    
    m.insert(':', 333);
    m.insert(';', 333);
    m.insert('<', 584);
    m.insert('=', 584);
    m.insert('>', 584);
    m.insert('?', 611);
    m.insert('@', 975);
    
    // Uppercase
    m.insert('A', 722);
    m.insert('B', 722);
    m.insert('C', 722);
    m.insert('D', 722);
    m.insert('E', 667);
    m.insert('F', 611);
    m.insert('G', 778);
    m.insert('H', 722);
    m.insert('I', 278);
    m.insert('J', 556);
    m.insert('K', 722);
    m.insert('L', 611);
    m.insert('M', 833);
    m.insert('N', 722);
    m.insert('O', 778);
    m.insert('P', 667);
    m.insert('Q', 778);
    m.insert('R', 722);
    m.insert('S', 667);
    m.insert('T', 611);
    m.insert('U', 722);
    m.insert('V', 667);
    m.insert('W', 944);
    m.insert('X', 667);
    m.insert('Y', 667);
    m.insert('Z', 611);
    
    m.insert('[', 333);
    m.insert('\\', 278);
    m.insert(']', 333);
    m.insert('^', 584);
    m.insert('_', 556);
    m.insert('`', 333);
    
    // Lowercase
    m.insert('a', 556);
    m.insert('b', 611);
    m.insert('c', 556);
    m.insert('d', 611);
    m.insert('e', 556);
    m.insert('f', 333);
    m.insert('g', 611);
    m.insert('h', 611);
    m.insert('i', 278);
    m.insert('j', 278);
    m.insert('k', 556);
    m.insert('l', 278);
    m.insert('m', 889);
    m.insert('n', 611);
    m.insert('o', 611);
    m.insert('p', 611);
    m.insert('q', 611);
    m.insert('r', 389);
    m.insert('s', 556);
    m.insert('t', 333);
    m.insert('u', 611);
    m.insert('v', 556);
    m.insert('w', 778);
    m.insert('x', 556);
    m.insert('y', 556);
    m.insert('z', 500);
    
    m.insert('{', 389);
    m.insert('|', 280);
    m.insert('}', 389);
    m.insert('~', 584);
    
    m
}

// ============================================================================
// GLOBAL METRICS CACHE
// ============================================================================

static HELVETICA: OnceLock<FontMetrics> = OnceLock::new();
static HELVETICA_BOLD: OnceLock<FontMetrics> = OnceLock::new();
static HELVETICA_OBLIQUE: OnceLock<FontMetrics> = OnceLock::new();
static HELVETICA_BOLD_OBLIQUE: OnceLock<FontMetrics> = OnceLock::new();

pub fn helvetica() -> &'static FontMetrics {
    HELVETICA.get_or_init(|| FontMetrics {
        widths: helvetica_widths(),
        default_width: 556, // Average width
        units_per_em: 1000,
        ascender: 718,
        descender: -207,
    })
}

pub fn helvetica_bold() -> &'static FontMetrics {
    HELVETICA_BOLD.get_or_init(|| FontMetrics {
        widths: helvetica_bold_widths(),
        default_width: 556,
        units_per_em: 1000,
        ascender: 718,
        descender: -207,
    })
}

pub fn helvetica_oblique() -> &'static FontMetrics {
    // Oblique uses same widths as regular
    HELVETICA_OBLIQUE.get_or_init(|| FontMetrics {
        widths: helvetica_widths(),
        default_width: 556,
        units_per_em: 1000,
        ascender: 718,
        descender: -207,
    })
}

pub fn helvetica_bold_oblique() -> &'static FontMetrics {
    HELVETICA_BOLD_OBLIQUE.get_or_init(|| FontMetrics {
        widths: helvetica_bold_widths(),
        default_width: 556,
        units_per_em: 1000,
        ascender: 718,
        descender: -207,
    })
}

/// Get font metrics for a given font variant
pub fn get_metrics(bold: bool, italic: bool) -> &'static FontMetrics {
    match (bold, italic) {
        (true, true) => helvetica_bold_oblique(),
        (true, false) => helvetica_bold(),
        (false, true) => helvetica_oblique(),
        (false, false) => helvetica(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_width() {
        let metrics = helvetica();
        
        // "Hello" at 12pt
        let width = metrics.string_width("Hello", 12.0);
        // H=722, e=556, l=222, l=222, o=556 = 2278 units
        // 2278/1000 * 12 = 27.336
        assert!((width - 27.336).abs() < 0.01);
    }

    #[test]
    fn test_space_width() {
        let metrics = helvetica();
        let space_width = metrics.char_width_pt(' ', 12.0);
        // 278/1000 * 12 = 3.336
        assert!((space_width - 3.336).abs() < 0.01);
    }
}

