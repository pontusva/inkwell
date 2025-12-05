use crate::layout::{
    JsonNode, NodeType, Direction, MainAlign, CrossAlign, TextAlign, FontWeight, FontStyle, Position, Dimension,
};
use crate::font_metrics;

// ============================================================================
// LAYOUT BOX
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct TableLayout {
    pub column_widths: Vec<f32>,
    pub row_heights: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct LayoutBox {
    // Final computed position (PDF coordinates: origin bottom-left)
    pub x: f32,
    pub y: f32,

    // Final computed size (includes padding, excludes margin)
    pub width: f32,
    pub height: f32,

    // Margin (space outside the box)
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,

    // Children
    pub children: Vec<LayoutBox>,

    // Reference to the original node
    pub node: JsonNode,

    // For Text nodes: pre-wrapped lines
    pub lines: Vec<String>,

    // For Table nodes: computed grid info
    pub table: Option<TableLayout>,
}

impl LayoutBox {
    pub fn new(node: JsonNode) -> Self {
        let (mt, mr, mb, ml) = node.style.margin_trbl();

        LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            margin_top: mt,
            margin_right: mr,
            margin_bottom: mb,
            margin_left: ml,
            children: Vec::new(),
            node,
            lines: Vec::new(),
            table: None,
        }
    }

    // --- Style accessors ---

    /// Get fixed width (returns None for percentages - use resolve_width instead)
    pub fn style_width(&self) -> Option<f32> {
        match &self.node.style.width {
            Some(Dimension::Pt(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get fixed height (returns None for percentages - use resolve_height instead)
    pub fn style_height(&self) -> Option<f32> {
        match &self.node.style.height {
            Some(Dimension::Pt(v)) => Some(*v),
            _ => None,
        }
    }

    /// Resolve width given parent width (handles both fixed and percentage)
    pub fn resolve_width(&self, parent_width: f32) -> Option<f32> {
        self.node.style.width.as_ref().map(|d| d.resolve(parent_width))
    }

    /// Resolve height given parent height (handles both fixed and percentage)
    pub fn resolve_height(&self, parent_height: f32) -> Option<f32> {
        self.node.style.height.as_ref().map(|d| d.resolve(parent_height))
    }

    /// Check if width is percentage-based
    pub fn has_percent_width(&self) -> bool {
        matches!(&self.node.style.width, Some(Dimension::Percent(_)))
    }

    /// Check if height is percentage-based
    pub fn has_percent_height(&self) -> bool {
        matches!(&self.node.style.height, Some(Dimension::Percent(_)))
    }

    pub fn flex(&self) -> f32 {
        self.node.style.flex.unwrap_or(0.0)
    }

    pub fn main_align(&self) -> MainAlign {
        self.node.style.main_align.clone().unwrap_or(MainAlign::Start)
    }

    pub fn cross_align(&self) -> CrossAlign {
        self.node.style.cross_align.clone().unwrap_or(CrossAlign::Start)
    }

    pub fn position(&self) -> Position {
        self.node.style.position.clone().unwrap_or(Position::Static)
    }

    pub fn is_absolute(&self) -> bool {
        matches!(self.position(), Position::Absolute)
    }

    pub fn is_relative(&self) -> bool {
        matches!(self.position(), Position::Relative)
    }

    pub fn text_align(&self) -> TextAlign {
        self.node.style.text_align.clone()
            .or_else(|| self.node.text_align.clone())
            .unwrap_or(TextAlign::Left)
    }

    pub fn font_size(&self) -> f32 {
        self.node.style.font_size
            .or(self.node.font_size)
            .unwrap_or(12.0)
    }

    pub fn font_weight(&self) -> Option<FontWeight> {
        self.node.style.font_weight.clone()
            .or_else(|| self.node.font_weight.clone())
    }

    pub fn font_style(&self) -> Option<FontStyle> {
        self.node.style.font_style.clone()
            .or_else(|| self.node.font_style.clone())
    }

    pub fn is_bold(&self) -> bool {
        self.font_weight().map(|w| w == FontWeight::Bold).unwrap_or(false)
    }

    pub fn is_italic(&self) -> bool {
        self.font_style().map(|s| s == FontStyle::Italic).unwrap_or(false)
    }

    pub fn line_height_multiplier(&self) -> f32 {
        self.node.style.line_height.unwrap_or(1.4)
    }

    /// Get font metrics for this node
    pub fn font_metrics(&self) -> &'static font_metrics::FontMetrics {
        font_metrics::get_metrics(self.is_bold(), self.is_italic())
    }

    /// Total outer width including margins
    pub fn outer_width(&self) -> f32 {
        self.margin_left + self.width + self.margin_right
    }

    /// Total outer height including margins
    pub fn outer_height(&self) -> f32 {
        self.margin_top + self.height + self.margin_bottom
    }

    pub fn col_span(&self) -> usize {
        self.node.col_span.unwrap_or(1).max(1)
    }

    pub fn row_span(&self) -> usize {
        self.node.row_span.unwrap_or(1).max(1)
    }
}

// ============================================================================
// TEXT MEASUREMENT (using real font metrics)
// ============================================================================

fn measure_text_width(text: &str, font_size: f32, metrics: &font_metrics::FontMetrics) -> f32 {
    metrics.string_width(text, font_size)
}

fn measure_line_height(font_size: f32, line_height_mult: f32) -> f32 {
    font_size * line_height_mult
}

// ============================================================================
// BUILD LAYOUT TREE
// ============================================================================

pub fn build_layout(node: &JsonNode) -> LayoutBox {
    let mut lb = LayoutBox::new(node.clone());
    for child in &node.children {
        lb.children.push(build_layout(child));
    }
    lb
}

// ============================================================================
// MEASURE PASS
// ============================================================================

/// Measure layout with optional parent dimensions for percentage resolution
/// For the root (Page), parent dimensions should be the page size
pub fn measure_layout(layout: &mut LayoutBox) {
    // Default to A4 page size for root elements
    measure_layout_with_parent(layout, 595.0, 842.0);
}

/// Measure layout with explicit parent dimensions
pub fn measure_layout_with_parent(layout: &mut LayoutBox, parent_width: f32, parent_height: f32) {
    match layout.node.node_type {
        NodeType::Text => measure_text(layout, parent_width),
        NodeType::Page | NodeType::View => measure_container(layout, parent_width, parent_height),
        NodeType::Table => measure_table(layout, parent_width, parent_height),
        NodeType::Row | NodeType::Cell => measure_container(layout, parent_width, parent_height),
        NodeType::Image | NodeType::Svg => measure_image(layout, parent_width, parent_height),
    }
}

fn measure_text(layout: &mut LayoutBox, parent_width: f32) {
    let text = layout.node.text.clone().unwrap_or_default();
    let size = layout.font_size();
    let line_h = layout.line_height_multiplier();
    // Resolve width (could be percentage or fixed)
    let max_width = layout.resolve_width(parent_width).or_else(|| layout.style_width());
    let metrics = layout.font_metrics();

    match max_width {
        Some(w) if w.is_finite() && w > 0.0 => {
            wrap_text(layout, &text, size, line_h, w, metrics);
        }
        _ => {
            // Single line
            layout.lines = vec![text.clone()];
            layout.width = measure_text_width(&text, size, metrics);
            layout.height = measure_line_height(size, line_h);
        }
    }
}

fn wrap_text(
    layout: &mut LayoutBox,
    text: &str,
    size: f32,
    line_h: f32,
    max_width: f32,
    metrics: &font_metrics::FontMetrics,
) {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let tentative = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };

        if measure_text_width(&tentative, size, metrics) > max_width && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            current = tentative;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    layout.lines = lines;
    layout.width = max_width;
    layout.height = measure_line_height(size, line_h) * (layout.lines.len() as f32);
}

fn measure_image(layout: &mut LayoutBox, parent_width: f32, parent_height: f32) {
    // Images require explicit width/height; default to 100x100
    // Resolve percentage dimensions
    layout.width = layout.resolve_width(parent_width).unwrap_or(100.0);
    layout.height = layout.resolve_height(parent_height).unwrap_or(100.0);
}

fn measure_container(layout: &mut LayoutBox, parent_width: f32, parent_height: f32) {
    let dir = layout.node.style.direction.clone().unwrap_or(Direction::Column);
    let gap = layout.node.style.gap.unwrap_or(0.0);
    let (pad_t, pad_r, pad_b, pad_l) = layout.node.style.padding_trbl();

    // Resolve this container's explicit dimensions first (for percentage children)
    let explicit_width = layout.resolve_width(parent_width);
    let explicit_height = layout.resolve_height(parent_height);

    // For width: use explicit width if available, otherwise use parent width
    // (percentage widths usually work because width flows down)
    let child_parent_width = explicit_width.unwrap_or(parent_width) - pad_l - pad_r;
    
    // For height: only pass explicit height to children for percentage resolution
    // If this container has no explicit height, children with height="100%" should
    // NOT expand to fill the grandparent's height - they should size to content.
    // This matches CSS behavior where percentage heights only work with explicit parent heights.
    let child_parent_height = if explicit_height.is_some() || layout.node.node_type == NodeType::Page {
        explicit_height.unwrap_or(parent_height) - pad_t - pad_b
    } else {
        // No explicit height - pass 0 so percentage heights become 0 (effectively auto)
        0.0
    };

    // First, measure all children with resolved parent dimensions
    for child in &mut layout.children {
        measure_layout_with_parent(child, child_parent_width, child_parent_height);
    }

    let (content_w, content_h) = match dir {
        Direction::Column => measure_column(&layout.children, gap),
        Direction::Row => {
            let wrap = layout.node.style.wrap.unwrap_or(false);
            if wrap && explicit_width.is_some() {
                measure_wrapping_row(&layout.children, gap, child_parent_width)
            } else {
                measure_row(&layout.children, gap)
            }
        }
    };

    // Add padding
    let mut width = content_w + pad_l + pad_r;
    let mut height = content_h + pad_t + pad_b;

    // Apply explicit overrides (already resolved from percentages)
    if let Some(w) = explicit_width {
        width = w;
    }
    if let Some(h) = explicit_height {
        height = h;
    }

    // Apply min/max constraints (resolve percentages)
    if let Some(ref min_w) = layout.node.style.min_width {
        width = width.max(min_w.resolve(parent_width));
    }
    if let Some(ref max_w) = layout.node.style.max_width {
        width = width.min(max_w.resolve(parent_width));
    }
    if let Some(ref min_h) = layout.node.style.min_height {
        height = height.max(min_h.resolve(parent_height));
    }
    if let Some(ref max_h) = layout.node.style.max_height {
        height = height.min(max_h.resolve(parent_height));
    }

    layout.width = width;
    layout.height = height;
}

fn measure_table(layout: &mut LayoutBox, parent_width: f32, parent_height: f32) {
    let row_count = layout.children.len();
    let row_gap = layout.node.style.gap.unwrap_or(0.0);
    let col_gap = layout.node.style.gap.unwrap_or(0.0);
    let (pad_t, pad_r, pad_b, pad_l) = layout.node.style.padding_trbl();

    eprintln!("TABLE: row_count={}, row_gap={}, col_gap={}, padding=({},{},{},{})", 
              row_count, row_gap, col_gap, pad_t, pad_r, pad_b, pad_l);

    // Determine explicit width/height
    let explicit_width = layout.resolve_width(parent_width);
    let explicit_height = layout.resolve_height(parent_height);
    eprintln!("TABLE: explicit_width={:?}, explicit_height={:?}, parent_width={}", 
              explicit_width, explicit_height, parent_width);

    // Determine number of columns based on maximum colspan across rows
    let mut num_cols = 0usize;
    for row in &layout.children {
        let mut col_total = 0usize;
        for cell in &row.children {
            col_total += cell.col_span();
        }
        num_cols = num_cols.max(col_total);
    }
    eprintln!("TABLE: num_cols={}", num_cols);

    if num_cols == 0 || row_count == 0 {
        layout.width = explicit_width.unwrap_or(0.0);
        layout.height = explicit_height.unwrap_or(0.0);
        layout.table = Some(TableLayout::default());
        return;
    }

    // Compute column widths
    let mut col_widths = vec![0.0f32; num_cols];
    let inner_available = explicit_width.unwrap_or(parent_width).max(0.0) - pad_l - pad_r;
    eprintln!("TABLE: inner_available={}", inner_available);

    if let Some(ref defs) = layout.node.column_widths {
        eprintln!("TABLE: column_widths defined, len={}", defs.len());
        for (i, dim) in defs.iter().enumerate().take(num_cols) {
            col_widths[i] = dim.resolve(inner_available);
            eprintln!("TABLE: col[{}] = {:?} -> {}", i, dim, col_widths[i]);
        }
    } else {
        eprintln!("TABLE: no column_widths defined");
    }

    let specified_total: f32 = col_widths.iter().sum();
    let total_col_gaps = col_gap * (num_cols.saturating_sub(1) as f32);
    let unspecified = col_widths.iter().filter(|w| **w == 0.0).count();
    let remaining = (inner_available - specified_total - total_col_gaps).max(0.0);
    let default_w = if unspecified > 0 {
        remaining / unspecified as f32
    } else {
        0.0
    };

    for w in col_widths.iter_mut() {
        if *w == 0.0 {
            *w = default_w;
        }
    }

    let inner_width: f32 = col_widths.iter().sum::<f32>() + total_col_gaps;

    // Measure cells and determine row heights (handling row spans)
    let mut row_heights = vec![0.0f32; row_count];
    let mut active_row_spans: Vec<usize> = vec![0; num_cols]; // rows remaining (including current) for each column

    for (row_idx, row) in layout.children.iter_mut().enumerate() {
        let mut col_idx = 0usize;

        // Skip columns covered by rowspans from previous rows
        while col_idx < num_cols && active_row_spans[col_idx] > 0 {
            col_idx += 1;
        }

        for cell in row.children.iter_mut() {
            while col_idx < num_cols && active_row_spans[col_idx] > 0 {
                col_idx += 1;
            }
            if col_idx >= num_cols {
                break;
            }

            let col_span = cell.col_span().min(num_cols - col_idx).max(1);
            let span_width: f32 = col_widths[col_idx..col_idx + col_span]
                .iter()
                .sum::<f32>() + col_gap * (col_span.saturating_sub(1) as f32);

            // Measure the cell's content within its span width
            measure_layout_with_parent(cell, span_width, parent_height);
            cell.width = span_width;

            let cell_height = cell.outer_height();
            if cell.row_span() == 1 {
                row_heights[row_idx] = row_heights[row_idx].max(cell_height);
            }

            let row_span = cell.row_span();
            if row_span > 1 {
                let end = (row_idx + row_span).min(row_count);
                let current: f32 = row_heights[row_idx..end].iter().sum();
                if cell_height > current {
                    let extra = cell_height - current;
                    row_heights[end - 1] += extra;
                }
                for c in col_idx..col_idx + col_span {
                    active_row_spans[c] = active_row_spans[c].max(row_span);
                }
            }

            col_idx += col_span;
        }

        // Consume one row for all active spans
        for span in active_row_spans.iter_mut() {
            if *span > 0 {
                *span -= 1;
            }
        }
    }

    // Any rows without direct height (all cells were row-spanned) fall back to minimal height 0
    let content_height: f32 = row_heights.iter().sum::<f32>()
        + row_gap * row_count.saturating_sub(1) as f32;

    let mut width = pad_l + inner_width + pad_r;
    let mut height = pad_t + content_height + pad_b;

    if let Some(w) = explicit_width {
        width = w;
    }
    if let Some(h) = explicit_height {
        height = h;
    }

    // Apply min/max constraints (resolve percentages)
    if let Some(ref min_w) = layout.node.style.min_width {
        width = width.max(min_w.resolve(parent_width));
    }
    if let Some(ref max_w) = layout.node.style.max_width {
        width = width.min(max_w.resolve(parent_width));
    }
    if let Some(ref min_h) = layout.node.style.min_height {
        height = height.max(min_h.resolve(parent_height));
    }
    if let Some(ref max_h) = layout.node.style.max_height {
        height = height.min(max_h.resolve(parent_height));
    }

    layout.width = width;
    layout.height = height;
    eprintln!("TABLE: final width={}, height={}", width, height);
    eprintln!("TABLE: col_widths={:?}", col_widths);
    eprintln!("TABLE: row_heights={:?}", row_heights);
    layout.table = Some(TableLayout { column_widths: col_widths.clone(), row_heights: row_heights.clone() });
}

fn measure_column(children: &[LayoutBox], gap: f32) -> (f32, f32) {
    let mut width: f32 = 0.0;
    let mut height: f32 = 0.0;
    let mut flow_index = 0;

    for child in children.iter() {
        // Absolute children don't contribute to parent size
        if child.is_absolute() {
            continue;
        }
        width = width.max(child.outer_width());
        height += child.outer_height();
        if flow_index > 0 {
            height += gap;
        }
        flow_index += 1;
    }

    (width, height)
}

fn measure_row(children: &[LayoutBox], gap: f32) -> (f32, f32) {
    let mut width: f32 = 0.0;
    let mut height: f32 = 0.0;
    let mut flow_index = 0;

    for child in children.iter() {
        // Absolute children don't contribute to parent size
        if child.is_absolute() {
            continue;
        }
        width += child.outer_width();
        if flow_index > 0 {
            width += gap;
        }
        height = height.max(child.outer_height());
        flow_index += 1;
    }

    (width, height)
}

fn measure_wrapping_row(children: &[LayoutBox], gap: f32, max_width: f32) -> (f32, f32) {
    let mut total_width: f32 = 0.0;
    let mut total_height: f32 = 0.0;
    let mut line_width: f32 = 0.0;
    let mut line_height: f32 = 0.0;

    for child in children {
        // Absolute children don't contribute to parent size
        if child.is_absolute() {
            continue;
        }

        let cw = child.outer_width();
        let ch = child.outer_height();

        if line_width == 0.0 {
            line_width = cw;
            line_height = ch;
        } else if line_width + gap + cw > max_width {
            // Wrap
            total_width = total_width.max(line_width);
            total_height += line_height + gap;
            line_width = cw;
            line_height = ch;
        } else {
            line_width += gap + cw;
            line_height = line_height.max(ch);
        }
    }

    total_width = total_width.max(line_width);
    total_height += line_height;

    (total_width, total_height)
}

// ============================================================================
// PLACE PASS
// ============================================================================

pub fn place_layout(layout: &mut LayoutBox, x: f32, y: f32) {
    // Position is inside the margin
    layout.x = x + layout.margin_left;
    layout.y = y - layout.margin_top;

    match layout.node.node_type {
        NodeType::Page | NodeType::View => place_container(layout),
        NodeType::Table => place_table(layout),
        NodeType::Row => place_row_element(layout),
        NodeType::Cell => place_cell(layout),
        _ => {} // Text/Image have no children to place
    }

    // Apply relative offset after normal placement
    if layout.is_relative() {
        apply_relative_offset(layout);
    }
}

/// Apply relative positioning offsets
fn apply_relative_offset(layout: &mut LayoutBox) {
    if let Some(top) = layout.node.style.top {
        layout.y -= top; // Move down (PDF y decreases downward)
    }
    if let Some(bottom) = layout.node.style.bottom {
        layout.y += bottom; // Move up
    }
    if let Some(left) = layout.node.style.left {
        layout.x += left; // Move right
    }
    if let Some(right) = layout.node.style.right {
        layout.x -= right; // Move left
    }
}

fn place_container(layout: &mut LayoutBox) {
    let dir = layout.node.style.direction.clone().unwrap_or(Direction::Column);
    let gap = layout.node.style.gap.unwrap_or(0.0);
    let (pad_t, pad_r, pad_b, pad_l) = layout.node.style.padding_trbl();
    let main_align = layout.main_align();
    let cross_align = layout.cross_align();

    let inner_w = layout.width - pad_l - pad_r;
    let inner_h = layout.height - pad_t - pad_b;

    // Separate absolute children from flow children
    let mut absolute_indices: Vec<usize> = Vec::new();
    for (i, child) in layout.children.iter().enumerate() {
        if child.is_absolute() {
            absolute_indices.push(i);
        }
    }

    // Place flow children (non-absolute)
    match dir {
        Direction::Column => {
            place_column(layout, inner_w, inner_h, gap, pad_t, pad_l, main_align, cross_align);
        }
        Direction::Row => {
            let wrap = layout.node.style.wrap.unwrap_or(false);
            if wrap {
                place_wrapping_row(layout, inner_w, inner_h, gap, pad_t, pad_l, main_align, cross_align);
            } else {
                place_row(layout, inner_w, inner_h, gap, pad_t, pad_l, main_align, cross_align);
            }
        }
    }

    // Place absolute children relative to this container
    let container_x = layout.x;
    let container_y = layout.y;
    let container_w = layout.width;
    let container_h = layout.height;

    for i in absolute_indices {
        let child = &mut layout.children[i];
        place_absolute_child(child, container_x, container_y, container_w, container_h, pad_t, pad_r, pad_b, pad_l);
    }
}

fn place_table(layout: &mut LayoutBox) {
    let table_layout = match layout.table.clone() {
        Some(t) => t,
        None => return,
    };

    let cols = table_layout.column_widths.len();
    let row_count = layout.children.len();
    if cols == 0 || row_count == 0 {
        return;
    }

    let (pad_t, _pad_r, _pad_b, pad_l) = layout.node.style.padding_trbl();
    let row_gap = layout.node.style.gap.unwrap_or(0.0);
    let col_gap = layout.node.style.gap.unwrap_or(0.0);
    let inner_width: f32 = table_layout.column_widths.iter().sum::<f32>()
        + col_gap * cols.saturating_sub(1) as f32;

    let mut active_row_spans: Vec<usize> = vec![0; cols]; // remaining rows to skip for each column
    let mut cursor_y = layout.y - pad_t;
    let start_x = layout.x + pad_l;

    for (row_idx, row) in layout.children.iter_mut().enumerate() {
        let row_height = *table_layout.row_heights.get(row_idx).unwrap_or(&0.0);
        let mut col_idx = 0usize;
        let mut cursor_x = start_x;

        // Position the row itself (for backgrounds/borders)
        row.x = start_x;
        row.y = cursor_y;
        row.width = inner_width;
        row.height = row_height;

        // Skip columns occupied by existing row spans
        while col_idx < cols && active_row_spans[col_idx] > 0 {
            cursor_x += table_layout.column_widths[col_idx];
            active_row_spans[col_idx] -= 1;
            col_idx += 1;
        }

        for cell in row.children.iter_mut() {
            while col_idx < cols && active_row_spans[col_idx] > 0 {
                cursor_x += table_layout.column_widths[col_idx] + col_gap;
                active_row_spans[col_idx] -= 1;
                col_idx += 1;
            }
            if col_idx >= cols {
                break;
            }

            let col_span = cell.col_span().min(cols - col_idx).max(1);
            let cell_width: f32 = table_layout.column_widths[col_idx..col_idx + col_span]
                .iter()
                .sum::<f32>() + col_gap * (col_span.saturating_sub(1) as f32);

            let row_span = cell.row_span().max(1);
            let end_row = (row_idx + row_span).min(row_count);
            let span_height: f32 = table_layout.row_heights[row_idx..end_row]
                .iter()
                .sum();
            let available_height = (span_height - cell.margin_top - cell.margin_bottom).max(0.0);

            cell.width = cell_width;
            if cell.height < available_height {
                cell.height = available_height;
            }

            let cell_x = cursor_x + cell.margin_left;
            let cell_y = cursor_y - cell.margin_top;

            place_layout(cell, cell_x - cell.margin_left, cell_y + cell.margin_top);

            for c in col_idx..col_idx + col_span {
                active_row_spans[c] = active_row_spans[c].max(row_span.saturating_sub(1));
            }

            cursor_x += cell_width + col_gap;
            col_idx += col_span;
        }

        // Consume spans for any remaining columns
        while col_idx < cols && active_row_spans[col_idx] > 0 {
            active_row_spans[col_idx] -= 1;
            col_idx += 1;
        }

        cursor_y -= row_height + row_gap;
    }
}

/// Place row element - rows are positioned by the table, but we need to place their children
fn place_row_element(layout: &mut LayoutBox) {
    // Row children (cells) are already positioned by place_table
    // But we need to ensure any nested content inside cells is placed
    for child in layout.children.iter_mut() {
        place_layout(child, child.x, child.y);
    }
}

/// Place cell content
fn place_cell(layout: &mut LayoutBox) {
    let (pad_t, _pad_r, _pad_b, pad_l) = layout.node.style.padding_trbl();
    
    // Position children inside the cell with padding
    let mut cursor_y = layout.y - pad_t;
    let start_x = layout.x + pad_l;
    let gap = layout.node.style.gap.unwrap_or(0.0);
    
    for (i, child) in layout.children.iter_mut().enumerate() {
        if i > 0 {
            cursor_y -= gap;
        }
        place_layout(child, start_x, cursor_y);
        cursor_y -= child.outer_height();
    }
}

/// Place an absolutely positioned child relative to its container
fn place_absolute_child(
    child: &mut LayoutBox,
    container_x: f32,
    container_y: f32,
    container_w: f32,
    container_h: f32,
    pad_t: f32,
    pad_r: f32,
    pad_b: f32,
    pad_l: f32,
) {
    let style = &child.node.style;

    // Calculate x position
    let x = if let Some(left) = style.left {
        container_x + pad_l + left
    } else if let Some(right) = style.right {
        container_x + container_w - pad_r - child.width - right
    } else {
        // Default to left edge
        container_x + pad_l
    };

    // Calculate y position (remember: PDF y increases upward)
    let y = if let Some(top) = style.top {
        container_y - pad_t - top
    } else if let Some(bottom) = style.bottom {
        container_y - container_h + pad_b + child.height + bottom
    } else {
        // Default to top edge
        container_y - pad_t
    };

    child.x = x;
    child.y = y;

    // Recursively place children of the absolute element
    match child.node.node_type {
        NodeType::Page | NodeType::View => place_container(child),
        _ => {}
    }
}

fn place_column(
    layout: &mut LayoutBox,
    inner_w: f32,
    inner_h: f32,
    gap: f32,
    pad_t: f32,
    pad_l: f32,
    main_align: MainAlign,
    cross_align: CrossAlign,
) {
    let x = layout.x;
    let y = layout.y;
    
    // Only count flow children (non-absolute)
    let flow_count = layout.children.iter().filter(|c| !c.is_absolute()).count();
    if flow_count == 0 {
        return;
    }

    // Calculate total children height (flow children only)
    let total_h: f32 = layout.children.iter()
        .filter(|c| !c.is_absolute())
        .map(|c| c.outer_height())
        .sum::<f32>()
        + gap * (flow_count.saturating_sub(1) as f32);

    let free = (inner_h - total_h).max(0.0);

    // Calculate flex (flow children only)
    let total_flex: f32 = layout.children.iter()
        .filter(|c| !c.is_absolute())
        .map(|c| c.flex())
        .sum();
    let flex_unit = if total_flex > 0.0 { free / total_flex } else { 0.0 };

    // Starting position and spacing
    let n = flow_count;
    let (mut cursor_y, base_gap) = match main_align {
        MainAlign::Start => (y - pad_t, gap),
        MainAlign::Center => (y - pad_t - free / 2.0, gap),
        MainAlign::End => (y - pad_t - free, gap),
        MainAlign::SpaceBetween if n > 1 => (y - pad_t, gap + free / (n as f32 - 1.0)),
        MainAlign::SpaceBetween => (y - pad_t, gap),
        MainAlign::SpaceAround => {
            let space = free / (n as f32);
            (y - pad_t - space / 2.0, gap + space)
        }
        MainAlign::SpaceEvenly => {
            let space = free / (n as f32 + 1.0);
            (y - pad_t - space, gap + space)
        }
    };

    let mut flow_index = 0;
    for child in layout.children.iter_mut() {
        // Skip absolute children - they're placed separately
        if child.is_absolute() {
            continue;
        }

        // Apply flex growth
        if child.flex() > 0.0 && total_flex > 0.0 {
            child.height += flex_unit * child.flex();
        }

        // Cross-axis alignment (horizontal)
        let child_x = match cross_align {
            CrossAlign::Start => x + pad_l + child.margin_left,
            CrossAlign::Center => x + pad_l + (inner_w - child.outer_width()) / 2.0 + child.margin_left,
            CrossAlign::End => x + pad_l + inner_w - child.outer_width() + child.margin_left,
            CrossAlign::Stretch => {
                child.width = inner_w - child.margin_left - child.margin_right;
                x + pad_l + child.margin_left
            }
        };

        let child_y = cursor_y - child.margin_top;

        place_layout(child, child_x - child.margin_left, child_y + child.margin_top);

        cursor_y -= child.outer_height();
        if flow_index < n - 1 {
            cursor_y -= base_gap;
        }
        flow_index += 1;
    }
}

fn place_row(
    layout: &mut LayoutBox,
    inner_w: f32,
    inner_h: f32,
    gap: f32,
    pad_t: f32,
    pad_l: f32,
    main_align: MainAlign,
    cross_align: CrossAlign,
) {
    let x = layout.x;
    let y = layout.y;
    
    // Only count flow children (non-absolute)
    let flow_count = layout.children.iter().filter(|c| !c.is_absolute()).count();
    if flow_count == 0 {
        return;
    }

    // Calculate total children width (flow children only)
    let total_w: f32 = layout.children.iter()
        .filter(|c| !c.is_absolute())
        .map(|c| c.outer_width())
        .sum::<f32>()
        + gap * (flow_count.saturating_sub(1) as f32);

    let free = (inner_w - total_w).max(0.0);

    // Calculate flex (flow children only)
    let total_flex: f32 = layout.children.iter()
        .filter(|c| !c.is_absolute())
        .map(|c| c.flex())
        .sum();
    let flex_unit = if total_flex > 0.0 { free / total_flex } else { 0.0 };

    // Starting position and spacing
    let n = flow_count;
    let (mut cursor_x, base_gap) = match main_align {
        MainAlign::Start => (x + pad_l, gap),
        MainAlign::Center => (x + pad_l + free / 2.0, gap),
        MainAlign::End => (x + pad_l + free, gap),
        MainAlign::SpaceBetween if n > 1 => (x + pad_l, gap + free / (n as f32 - 1.0)),
        MainAlign::SpaceBetween => (x + pad_l, gap),
        MainAlign::SpaceAround => {
            let space = free / (n as f32);
            (x + pad_l + space / 2.0, gap + space)
        }
        MainAlign::SpaceEvenly => {
            let space = free / (n as f32 + 1.0);
            (x + pad_l + space, gap + space)
        }
    };

    let mut flow_index = 0;
    for child in layout.children.iter_mut() {
        // Skip absolute children - they're placed separately
        if child.is_absolute() {
            continue;
        }

        // Apply flex growth
        if child.flex() > 0.0 && total_flex > 0.0 {
            child.width += flex_unit * child.flex();
        }

        // Cross-axis alignment (vertical)
        let child_y = match cross_align {
            CrossAlign::Start => y - pad_t - child.margin_top,
            CrossAlign::Center => y - pad_t - (inner_h - child.outer_height()) / 2.0 - child.margin_top,
            CrossAlign::End => y - pad_t - (inner_h - child.outer_height()) - child.margin_top,
            CrossAlign::Stretch => {
                child.height = inner_h - child.margin_top - child.margin_bottom;
                y - pad_t - child.margin_top
            }
        };

        let child_x = cursor_x + child.margin_left;

        place_layout(child, child_x - child.margin_left, child_y + child.margin_top);

        cursor_x += child.outer_width();
        if flow_index < n - 1 {
            cursor_x += base_gap;
        }
        flow_index += 1;
    }
}

fn place_wrapping_row(
    layout: &mut LayoutBox,
    inner_w: f32,
    _inner_h: f32,
    gap: f32,
    pad_t: f32,
    pad_l: f32,
    main_align: MainAlign,
    cross_align: CrossAlign,
) {
    let x = layout.x;
    let y = layout.y;

    // Group children into lines
    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut line_w: f32 = 0.0;

    for (i, child) in layout.children.iter().enumerate() {
        let cw = child.outer_width();

        if line_w == 0.0 {
            lines.last_mut().unwrap().push(i);
            line_w = cw;
        } else if line_w + gap + cw > inner_w {
            lines.push(vec![i]);
            line_w = cw;
        } else {
            lines.last_mut().unwrap().push(i);
            line_w += gap + cw;
        }
    }

    // Place each line
    let mut cursor_y = y - pad_t;

    for line_indices in &lines {
        if line_indices.is_empty() {
            continue;
        }

        let line_total_w: f32 = line_indices.iter()
            .map(|&i| layout.children[i].outer_width())
            .sum::<f32>()
            + gap * (line_indices.len().saturating_sub(1) as f32);

        let line_h: f32 = line_indices.iter()
            .map(|&i| layout.children[i].outer_height())
            .fold(0.0, f32::max);

        let free = (inner_w - line_total_w).max(0.0);
        let n = line_indices.len();

        let (mut cursor_x, base_gap) = match main_align {
            MainAlign::Start => (x + pad_l, gap),
            MainAlign::Center => (x + pad_l + free / 2.0, gap),
            MainAlign::End => (x + pad_l + free, gap),
            MainAlign::SpaceBetween if n > 1 => (x + pad_l, gap + free / (n as f32 - 1.0)),
            MainAlign::SpaceBetween => (x + pad_l, gap),
            MainAlign::SpaceAround => {
                let space = free / (n as f32);
                (x + pad_l + space / 2.0, gap + space)
            }
            MainAlign::SpaceEvenly => {
                let space = free / (n as f32 + 1.0);
                (x + pad_l + space, gap + space)
            }
        };

        for (j, &child_idx) in line_indices.iter().enumerate() {
            let child = &mut layout.children[child_idx];

            let child_y = match cross_align {
                CrossAlign::Start => cursor_y - child.margin_top,
                CrossAlign::Center => cursor_y - (line_h - child.outer_height()) / 2.0 - child.margin_top,
                CrossAlign::End => cursor_y - (line_h - child.outer_height()) - child.margin_top,
                CrossAlign::Stretch => cursor_y - child.margin_top,
            };

            let child_x = cursor_x + child.margin_left;

            place_layout(child, child_x - child.margin_left, child_y + child.margin_top);

            cursor_x += child.outer_width();
            if j < n - 1 {
                cursor_x += base_gap;
            }
        }

        cursor_y -= line_h + gap;
    }
}
