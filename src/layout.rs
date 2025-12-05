use serde::Deserialize;

// ============================================================================
// NODE TYPES
// ============================================================================

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Page,
    View,
    Text,
    Image,
    Svg,
    Table,
    Row,
    Cell,
}

// ============================================================================
// OBJECT FIT
// ============================================================================

#[derive(Debug, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ObjectFit {
    /// Scale to fill the container, may crop (default for cover behavior)
    Cover,
    /// Scale to fit entirely within container, may have empty space
    #[default]
    Contain,
    /// Stretch to fill exactly (distorts aspect ratio)
    Fill,
    /// No scaling, use original size
    None,
    /// Use smaller of none or contain
    ScaleDown,
}

// ============================================================================
// ENUMS
// ============================================================================

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Row,
    Column,
}

/// Main-axis alignment (justify-content)
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MainAlign {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Cross-axis alignment (align-items)
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CrossAlign {
    Start,
    Center,
    End,
    Stretch,
}

/// Position type (like CSS position)
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    /// Normal flow (default)
    Static,
    /// Offset from normal position
    Relative,
    /// Positioned relative to nearest positioned ancestor (or page)
    Absolute,
}

impl Default for Position {
    fn default() -> Self {
        Position::Static
    }
}

/// Dimension value - can be fixed points or percentage
#[derive(Debug, Clone, PartialEq)]
pub enum Dimension {
    /// Fixed value in points
    Pt(f32),
    /// Percentage of parent dimension (0-100)
    Percent(f32),
}

impl Dimension {
    /// Resolve dimension to points given parent size
    pub fn resolve(&self, parent_size: f32) -> f32 {
        match self {
            Dimension::Pt(v) => *v,
            Dimension::Percent(p) => parent_size * p / 100.0,
        }
    }

    /// Check if this is a percentage value
    pub fn is_percent(&self) -> bool {
        matches!(self, Dimension::Percent(_))
    }
}

// Custom deserializer for Dimension to handle both numbers and strings like "50%"
impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct DimensionVisitor;

        impl<'de> Visitor<'de> for DimensionVisitor {
            type Value = Dimension;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or a string like \"50%\"")
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Dimension::Pt(v as f32))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Dimension::Pt(v as f32))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Dimension::Pt(v as f32))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Some(percent_str) = v.strip_suffix('%') {
                    percent_str
                        .trim()
                        .parse::<f32>()
                        .map(Dimension::Percent)
                        .map_err(|_| de::Error::custom(format!("invalid percentage: {}", v)))
                } else {
                    // Try to parse as a plain number
                    v.trim()
                        .parse::<f32>()
                        .map(Dimension::Pt)
                        .map_err(|_| de::Error::custom(format!("invalid dimension: {}", v)))
                }
            }
        }

        deserializer.deserialize_any(DimensionVisitor)
    }
}

/// Text alignment within a text box
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Font weight
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Normal,
    Bold,
}

/// Font style
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

// ============================================================================
// COLOR
// ============================================================================

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default = "default_alpha")]
    pub a: f32,
}

fn default_alpha() -> f32 {
    1.0
}

impl Color {
    pub fn black() -> Self {
        Color { r: 0, g: 0, b: 0, a: 1.0 }
    }

    pub fn white() -> Self {
        Color { r: 255, g: 255, b: 255, a: 1.0 }
    }

    pub fn transparent() -> Self {
        Color { r: 0, g: 0, b: 0, a: 0.0 }
    }
}

// ============================================================================
// BORDER
// ============================================================================

/// Individual border side
#[derive(Debug, Deserialize, Clone, Default)]
pub struct BorderSide {
    pub width: Option<f32>,
    pub color: Option<Color>,
}

/// Full border specification
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Border {
    pub width: Option<f32>,
    pub color: Option<Color>,
    pub radius: Option<f32>,
}

// ============================================================================
// STYLE
// ============================================================================

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Style {
    // --- Dimensions (can be fixed points or percentage like "50%") ---
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    #[serde(alias = "minWidth")]
    pub min_width: Option<Dimension>,
    #[serde(alias = "minHeight")]
    pub min_height: Option<Dimension>,
    #[serde(alias = "maxWidth")]
    pub max_width: Option<Dimension>,
    #[serde(alias = "maxHeight")]
    pub max_height: Option<Dimension>,

    // --- Positioning ---
    pub position: Option<Position>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,

    // --- Flex / Layout ---
    pub direction: Option<Direction>,
    pub wrap: Option<bool>,
    #[serde(alias = "mainAlign")]
    pub main_align: Option<MainAlign>,
    #[serde(alias = "crossAlign")]
    pub cross_align: Option<CrossAlign>,
    pub gap: Option<f32>,
    /// flex-grow: how much of remaining space to take (0 = none, 1 = equal share)
    pub flex: Option<f32>,

    // --- Padding (inside) ---
    pub padding: Option<f32>,
    #[serde(alias = "paddingTop")]
    pub padding_top: Option<f32>,
    #[serde(alias = "paddingRight")]
    pub padding_right: Option<f32>,
    #[serde(alias = "paddingBottom")]
    pub padding_bottom: Option<f32>,
    #[serde(alias = "paddingLeft")]
    pub padding_left: Option<f32>,

    // --- Margin (outside) ---
    pub margin: Option<f32>,
    #[serde(alias = "marginTop")]
    pub margin_top: Option<f32>,
    #[serde(alias = "marginRight")]
    pub margin_right: Option<f32>,
    #[serde(alias = "marginBottom")]
    pub margin_bottom: Option<f32>,
    #[serde(alias = "marginLeft")]
    pub margin_left: Option<f32>,

    // --- Background ---
    #[serde(alias = "backgroundColor")]
    pub background_color: Option<Color>,
    /// Opacity (0.0 = transparent, 1.0 = opaque)
    pub opacity: Option<f32>,

    // --- Border (shorthand) ---
    pub border: Option<Border>,
    #[serde(alias = "borderWidth")]
    pub border_width: Option<f32>,
    #[serde(alias = "borderColor")]
    pub border_color: Option<Color>,
    #[serde(alias = "borderRadius")]
    pub border_radius: Option<f32>,

    // --- Per-side borders ---
    #[serde(alias = "borderTop")]
    pub border_top: Option<BorderSide>,
    #[serde(alias = "borderRight")]
    pub border_right: Option<BorderSide>,
    #[serde(alias = "borderBottom")]
    pub border_bottom: Option<BorderSide>,
    #[serde(alias = "borderLeft")]
    pub border_left: Option<BorderSide>,

    // --- Per-side border widths ---
    #[serde(alias = "borderTopWidth")]
    pub border_top_width: Option<f32>,
    #[serde(alias = "borderRightWidth")]
    pub border_right_width: Option<f32>,
    #[serde(alias = "borderBottomWidth")]
    pub border_bottom_width: Option<f32>,
    #[serde(alias = "borderLeftWidth")]
    pub border_left_width: Option<f32>,

    // --- Per-side border colors ---
    #[serde(alias = "borderTopColor")]
    pub border_top_color: Option<Color>,
    #[serde(alias = "borderRightColor")]
    pub border_right_color: Option<Color>,
    #[serde(alias = "borderBottomColor")]
    pub border_bottom_color: Option<Color>,
    #[serde(alias = "borderLeftColor")]
    pub border_left_color: Option<Color>,

    // --- Per-corner border radius ---
    #[serde(alias = "borderTopLeftRadius")]
    pub border_top_left_radius: Option<f32>,
    #[serde(alias = "borderTopRightRadius")]
    pub border_top_right_radius: Option<f32>,
    #[serde(alias = "borderBottomRightRadius")]
    pub border_bottom_right_radius: Option<f32>,
    #[serde(alias = "borderBottomLeftRadius")]
    pub border_bottom_left_radius: Option<f32>,

    // --- Text ---
    #[serde(alias = "textAlign")]
    pub text_align: Option<TextAlign>,
    pub color: Option<Color>,
    #[serde(alias = "fontSize")]
    pub font_size: Option<f32>,
    #[serde(alias = "fontWeight")]
    pub font_weight: Option<FontWeight>,
    #[serde(alias = "fontStyle")]
    pub font_style: Option<FontStyle>,
    #[serde(alias = "lineHeight")]
    pub line_height: Option<f32>,

    // --- Image ---
    #[serde(alias = "objectFit")]
    pub object_fit: Option<ObjectFit>,
}

// ============================================================================
// JSON NODE
// ============================================================================

#[derive(Debug, Deserialize, Clone)]
pub struct JsonNode {
    #[serde(rename = "type")]
    pub node_type: NodeType,

    #[serde(default)]
    pub style: Style,

    #[serde(default)]
    pub children: Vec<JsonNode>,

    // Text content
    pub text: Option<String>,

    // Legacy properties at node level (prefer style.*)
    // Support both snake_case and camelCase
    #[serde(alias = "fontSize")]
    pub font_size: Option<f32>,

    #[serde(alias = "fontWeight")]
    pub font_weight: Option<FontWeight>,

    #[serde(alias = "fontStyle")]
    pub font_style: Option<FontStyle>,

    #[serde(alias = "textAlign")]
    pub text_align: Option<TextAlign>,

    // Image/SVG source
    pub src: Option<String>,

    // SVG content (alternative to src for inline SVG)
    pub content: Option<String>,

    // Table-specific
    #[serde(alias = "columnWidths")]
    pub column_widths: Option<Vec<Dimension>>,
    #[serde(alias = "colSpan")]
    pub col_span: Option<usize>,
    #[serde(alias = "rowSpan")]
    pub row_span: Option<usize>,
}

// ============================================================================
// PAYLOAD
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LayoutPayload {
    pub root: JsonNode,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

impl Style {
    /// Get padding as (top, right, bottom, left)
    pub fn padding_trbl(&self) -> (f32, f32, f32, f32) {
        let base = self.padding.unwrap_or(0.0);
        (
            self.padding_top.unwrap_or(base),
            self.padding_right.unwrap_or(base),
            self.padding_bottom.unwrap_or(base),
            self.padding_left.unwrap_or(base),
        )
    }

    /// Get margin as (top, right, bottom, left)
    pub fn margin_trbl(&self) -> (f32, f32, f32, f32) {
        let base = self.margin.unwrap_or(0.0);
        (
            self.margin_top.unwrap_or(base),
            self.margin_right.unwrap_or(base),
            self.margin_bottom.unwrap_or(base),
            self.margin_left.unwrap_or(base),
        )
    }

    /// Get effective border width (uniform)
    pub fn border_width(&self) -> f32 {
        self.border_width
            .or_else(|| self.border.as_ref().and_then(|b| b.width))
            .unwrap_or(0.0)
    }

    /// Get border widths as (top, right, bottom, left)
    pub fn border_widths(&self) -> (f32, f32, f32, f32) {
        let base = self.border_width();
        (
            self.border_top_width
                .or_else(|| self.border_top.as_ref().and_then(|b| b.width))
                .unwrap_or(base),
            self.border_right_width
                .or_else(|| self.border_right.as_ref().and_then(|b| b.width))
                .unwrap_or(base),
            self.border_bottom_width
                .or_else(|| self.border_bottom.as_ref().and_then(|b| b.width))
                .unwrap_or(base),
            self.border_left_width
                .or_else(|| self.border_left.as_ref().and_then(|b| b.width))
                .unwrap_or(base),
        )
    }

    /// Get effective border color (uniform)
    pub fn border_color(&self) -> Option<Color> {
        self.border_color
            .clone()
            .or_else(|| self.border.as_ref().and_then(|b| b.color.clone()))
    }

    /// Get border colors as (top, right, bottom, left)
    pub fn border_colors(&self) -> (Option<Color>, Option<Color>, Option<Color>, Option<Color>) {
        let base = self.border_color();
        (
            self.border_top_color
                .clone()
                .or_else(|| self.border_top.as_ref().and_then(|b| b.color.clone()))
                .or_else(|| base.clone()),
            self.border_right_color
                .clone()
                .or_else(|| self.border_right.as_ref().and_then(|b| b.color.clone()))
                .or_else(|| base.clone()),
            self.border_bottom_color
                .clone()
                .or_else(|| self.border_bottom.as_ref().and_then(|b| b.color.clone()))
                .or_else(|| base.clone()),
            self.border_left_color
                .clone()
                .or_else(|| self.border_left.as_ref().and_then(|b| b.color.clone()))
                .or_else(|| base),
        )
    }

    /// Get effective border radius (uniform)
    pub fn border_radius(&self) -> f32 {
        self.border_radius
            .or_else(|| self.border.as_ref().and_then(|b| b.radius))
            .unwrap_or(0.0)
    }

    /// Get border radii as (top-left, top-right, bottom-right, bottom-left)
    pub fn border_radii(&self) -> (f32, f32, f32, f32) {
        let base = self.border_radius();
        (
            self.border_top_left_radius.unwrap_or(base),
            self.border_top_right_radius.unwrap_or(base),
            self.border_bottom_right_radius.unwrap_or(base),
            self.border_bottom_left_radius.unwrap_or(base),
        )
    }

    /// Get opacity (0.0 - 1.0)
    pub fn opacity(&self) -> f32 {
        self.opacity.unwrap_or(1.0).clamp(0.0, 1.0)
    }

    /// Check if any border is defined
    pub fn has_border(&self) -> bool {
        let (t, r, b, l) = self.border_widths();
        t > 0.0 || r > 0.0 || b > 0.0 || l > 0.0
    }
}
