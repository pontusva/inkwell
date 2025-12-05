use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder};
use std::io::{BufWriter, Cursor};

use crate::layout::{JsonNode, NodeType, TextAlign, Color, Style};
use crate::layout_box::{LayoutBox, build_layout, measure_layout, place_layout};
use crate::svg::{self, SvgDocument, SvgElement, PathCommand};

// ============================================================================
// CONSTANTS
// ============================================================================

const PAGE_HEIGHT_PT: f32 = 842.0;  // A4 height in points
const PT_TO_MM: f32 = 0.352_777_78;

// ============================================================================
// PAGE STRUCTURE FOR PAGINATION
// ============================================================================

/// Represents a single page with its content
#[derive(Debug, Clone)]
struct PageContent {
    /// The layout boxes to render on this page
    children: Vec<LayoutBox>,
    /// Page style (for background, borders, etc.)
    style: Style,
    /// Page dimensions
    width: f32,
    height: f32,
    /// Padding
    padding_top: f32,
    padding_right: f32,
    padding_bottom: f32,
    padding_left: f32,
}

// ============================================================================
// PUBLIC API
// ============================================================================

pub fn from_layout(root: &JsonNode) -> Vec<u8> {
    // 1) Build layout tree
    let mut root_box = build_layout(root);

    // 2) Measure pass
    measure_layout(&mut root_box);

    // 3) Place pass (start at top-left with margin)
    place_layout(&mut root_box, 0.0, PAGE_HEIGHT_PT);

    // 4) Paginate - split content across pages if needed
    let pages = paginate(&root_box);

    // 5) Create PDF document
    let (doc, page1, layer1) =
        PdfDocument::new("PDF Document", Mm(210.0), Mm(297.0), "Layer 1");

    // Load fonts
    let font_regular = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();
    let font_italic = doc.add_builtin_font(BuiltinFont::HelveticaOblique).unwrap();
    let font_bold_italic = doc.add_builtin_font(BuiltinFont::HelveticaBoldOblique).unwrap();

    let fonts = Fonts {
        regular: font_regular,
        bold: font_bold,
        italic: font_italic,
        bold_italic: font_bold_italic,
    };

    // 6) Draw each page
    for (i, page_content) in pages.iter().enumerate() {
        let layer = if i == 0 {
            doc.get_page(page1).get_layer(layer1)
        } else {
            // Create new page
            let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
            doc.get_page(new_page).get_layer(new_layer)
        };

        draw_page(&page_content, &layer, &fonts, &doc);
    }

    // 7) Export to bytes
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut writer = BufWriter::new(cursor);
        doc.save(&mut writer).unwrap();
    }

    buf
}

// ============================================================================
// PAGINATION
// ============================================================================

/// Find the innermost container with actual content children to paginate
/// This handles nested Page/View structures and finds where the actual content is
fn find_content_page(root: &LayoutBox) -> &LayoutBox {
    // If this is a Page/View with a single Page/View child, recurse into it
    if (root.node.node_type == NodeType::Page || root.node.node_type == NodeType::View) 
        && root.children.len() == 1 
    {
        let child = &root.children[0];
        if child.node.node_type == NodeType::Page || child.node.node_type == NodeType::View {
            // Recurse into the child to find the actual content
            return find_content_page(child);
        }
    }
    root
}

/// Split content across multiple pages
fn paginate(root: &LayoutBox) -> Vec<PageContent> {
    let mut pages: Vec<PageContent> = Vec::new();

    // Find the actual content container (handle nested pages)
    let page = find_content_page(root);

    // Only paginate Page nodes
    if page.node.node_type != NodeType::Page {
        // Non-page root: just render as single page
        pages.push(PageContent {
            children: vec![page.clone()],
            style: Style::default(),
            width: page.width,
            height: page.height,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
        });
        return pages;
    }

    let (pad_t, pad_r, pad_b, pad_l) = page.node.style.padding_trbl();
    let page_width = page.width;
    let page_height = page.height;
    let content_top = page_height - pad_t;  // Top of content area (PDF coords)
    let content_bottom = pad_b;              // Bottom of content area
    let content_height = content_top - content_bottom;

    eprintln!("PAGINATION: page_height={}, pad_t={}, pad_b={}, content_top={}, content_bottom={}, content_height={}", 
              page_height, pad_t, pad_b, content_top, content_bottom, content_height);
    eprintln!("PAGINATION: page has {} children", page.children.len());

    // First, assign each child to a page based on its position
    let mut page_assignments: Vec<(usize, LayoutBox)> = Vec::new(); // (page_number, child)
    
    for (i, child) in page.children.iter().enumerate() {
        let child_bottom = child.y - child.height;
        
        // Calculate which page this child should be on
        // Page 0: content_bottom <= child_bottom
        // Page 1: content_bottom - content_height <= child_bottom < content_bottom
        // etc.
        let child_page = if child_bottom >= content_bottom {
            0
        } else {
            // How far below content_bottom is the child's bottom?
            let overflow = content_bottom - child_bottom;
            (overflow / content_height).ceil() as usize
        };
        
        if i < 10 || i % 50 == 0 {
            eprintln!("Child {}: y={:.1}, height={:.1}, bottom={:.1}, assigned to page {}", 
                      i, child.y, child.height, child_bottom, child_page);
        }
        
        // Clone and reposition the child for its target page
        let mut repositioned_child = child.clone();
        if child_page > 0 {
            let y_offset = child_page as f32 * content_height;
            reposition_layout(&mut repositioned_child, 0.0, y_offset);
        }
        
        page_assignments.push((child_page, repositioned_child));
    }

    // Find the maximum page number
    let max_page = page_assignments.iter().map(|(p, _)| *p).max().unwrap_or(0);
    eprintln!("PAGINATION: max_page = {}", max_page);

    // Create pages and distribute children
    for page_num in 0..=max_page {
        let children_for_page: Vec<LayoutBox> = page_assignments
            .iter()
            .filter(|(p, _)| *p == page_num)
            .map(|(_, child)| child.clone())
            .collect();
        
        eprintln!("PAGINATION: page {} has {} children", page_num, children_for_page.len());
        
        pages.push(PageContent {
            children: children_for_page,
            style: page.node.style.clone(),
            width: page_width,
            height: page_height,
            padding_top: pad_t,
            padding_right: pad_r,
            padding_bottom: pad_b,
            padding_left: pad_l,
        });
    }

    pages
}

/// Reposition a layout box and all its children by an offset
fn reposition_layout(layout: &mut LayoutBox, x_offset: f32, y_offset: f32) {
    layout.x += x_offset;
    layout.y += y_offset;
    for child in &mut layout.children {
        reposition_layout(child, x_offset, y_offset);
    }
}

/// Draw a single page
fn draw_page(page: &PageContent, layer: &PdfLayerReference, fonts: &Fonts, doc: &PdfDocumentReference) {
    // Draw page background if any
    if let Some(ref bg) = page.style.background_color {
        set_fill_color(layer, bg);
        draw_rect(layer, 0.0, 0.0, page.width, page.height, true, false);
    }

    // Draw all children on this page
    for child in &page.children {
        draw_layout(child, layer, fonts, doc);
    }
}

// ============================================================================
// FONTS
// ============================================================================

struct Fonts {
    regular: IndirectFontRef,
    bold: IndirectFontRef,
    italic: IndirectFontRef,
    bold_italic: IndirectFontRef,
}

impl Fonts {
    fn get(&self, bold: bool, italic: bool) -> &IndirectFontRef {
        match (bold, italic) {
            (true, true) => &self.bold_italic,
            (true, false) => &self.bold,
            (false, true) => &self.italic,
            (false, false) => &self.regular,
        }
    }
}

// ============================================================================
// TEXT MEASUREMENT
// ============================================================================

use crate::font_metrics;

fn text_width(text: &str, font_size: f32, metrics: &font_metrics::FontMetrics) -> f32 {
    metrics.string_width(text, font_size)
}

fn line_height(font_size: f32, line_height_mult: f32) -> f32 {
    font_size * line_height_mult
}

// ============================================================================
// DRAWING
// ============================================================================

fn draw_layout(layout: &LayoutBox, layer: &PdfLayerReference, fonts: &Fonts, doc: &PdfDocumentReference) {
    // 1) Draw background (if any)
    draw_background(layout, layer);

    // 2) Draw border (if any)
    draw_border(layout, layer);

    // 3) Draw content
    match layout.node.node_type {
        NodeType::Text => draw_text(layout, layer, fonts),
        NodeType::Image => draw_image(layout, layer, doc),
        NodeType::Svg => draw_svg(layout, layer),
        _ => {
            // Container: draw children
            for child in &layout.children {
                draw_layout(child, layer, fonts, doc);
            }
        }
    }
}

fn draw_background(layout: &LayoutBox, layer: &PdfLayerReference) {
    if let Some(ref bg) = layout.node.style.background_color {
        let opacity = layout.node.style.opacity();
        if bg.a <= 0.0 || opacity <= 0.0 {
            return; // Transparent
        }

        let x = layout.x;
        let y = layout.y - layout.height; // Bottom-left corner
        let w = layout.width;
        let h = layout.height;
        let (r_tl, r_tr, r_br, r_bl) = layout.node.style.border_radii();

        // Apply opacity to background color
        let bg_with_opacity = Color {
            r: bg.r,
            g: bg.g,
            b: bg.b,
            a: bg.a * opacity,
        };
        set_fill_color(layer, &bg_with_opacity);

        if r_tl > 0.0 || r_tr > 0.0 || r_br > 0.0 || r_bl > 0.0 {
            draw_rounded_rect_corners(layer, x, y, w, h, r_tl, r_tr, r_br, r_bl, true, false);
        } else {
            draw_rect(layer, x, y, w, h, true, false);
        }
    }
}

fn draw_border(layout: &LayoutBox, layer: &PdfLayerReference) {
    let border_width = layout.node.style.border_width();
    let border_color = layout.node.style.border_color();
    let (r_tl, r_tr, r_br, r_bl) = layout.node.style.border_radii();

    // Check if we have any border
    if border_width <= 0.0 {
        return;
    }

    let color = border_color.unwrap_or_else(Color::black);
    set_stroke_color(layer, &color);
    layer.set_outline_thickness(border_width);

    let x = layout.x;
    let y = layout.y - layout.height;
    let w = layout.width;
    let h = layout.height;

    if r_tl > 0.0 || r_tr > 0.0 || r_br > 0.0 || r_bl > 0.0 {
        draw_rounded_rect_corners(layer, x, y, w, h, r_tl, r_tr, r_br, r_bl, false, true);
    } else {
        draw_rect(layer, x, y, w, h, false, true);
    }
}

fn draw_line(layer: &PdfLayerReference, x1: f32, y1: f32, x2: f32, y2: f32) {
    let points = vec![
        (Point::new(Mm(x1 * PT_TO_MM), Mm(y1 * PT_TO_MM)), false),
        (Point::new(Mm(x2 * PT_TO_MM), Mm(y2 * PT_TO_MM)), false),
    ];
    let line = Line { points, is_closed: false };
    layer.add_line(line);
}

fn draw_rect(layer: &PdfLayerReference, x: f32, y: f32, w: f32, h: f32, fill: bool, stroke: bool) {
    let points = vec![
        (Point::new(Mm(x * PT_TO_MM), Mm(y * PT_TO_MM)), false),
        (Point::new(Mm((x + w) * PT_TO_MM), Mm(y * PT_TO_MM)), false),
        (Point::new(Mm((x + w) * PT_TO_MM), Mm((y + h) * PT_TO_MM)), false),
        (Point::new(Mm(x * PT_TO_MM), Mm((y + h) * PT_TO_MM)), false),
    ];

    if fill {
        let polygon = Polygon {
            rings: vec![points.clone()],
            mode: if stroke {
                PaintMode::FillStroke
            } else {
                PaintMode::Fill
            },
            winding_order: WindingOrder::NonZero,
        };
        layer.add_polygon(polygon);
    } else if stroke {
        let line = Line { points, is_closed: true };
        layer.add_line(line);
    }
}

fn draw_rounded_rect_corners(
    layer: &PdfLayerReference,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r_tl: f32,  // top-left radius
    r_tr: f32,  // top-right radius
    r_br: f32,  // bottom-right radius
    r_bl: f32,  // bottom-left radius
    fill: bool,
    stroke: bool,
) {
    // Clamp radii to half the smallest dimension
    let max_r = (w / 2.0).min(h / 2.0);
    let r_tl = r_tl.min(max_r);
    let r_tr = r_tr.min(max_r);
    let r_br = r_br.min(max_r);
    let r_bl = r_bl.min(max_r);

    // Number of segments to approximate each quarter circle
    let segments = 8;
    let pi = std::f32::consts::PI;

    let mut points = Vec::new();

    // Helper to add arc points (quarter circle approximation using line segments)
    let add_arc = |points: &mut Vec<(Point, bool)>, cx: f32, cy: f32, r: f32, start_angle: f32, end_angle: f32| {
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = start_angle + t * (end_angle - start_angle);
            let px = cx + r * angle.cos();
            let py = cy + r * angle.sin();
            points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
        }
    };

    // Start at bottom-left, after the corner curve
    // Bottom edge (left to right)
    points.push((Point::new(Mm((x + r_bl) * PT_TO_MM), Mm(y * PT_TO_MM)), false));
    points.push((Point::new(Mm((x + w - r_br) * PT_TO_MM), Mm(y * PT_TO_MM)), false));

    // Bottom-right corner arc (from -90° to 0°, i.e. from bottom to right)
    if r_br > 0.0 {
        let cx = x + w - r_br;
        let cy = y + r_br;
        add_arc(&mut points, cx, cy, r_br, -pi / 2.0, 0.0);
    } else {
        points.push((Point::new(Mm((x + w) * PT_TO_MM), Mm(y * PT_TO_MM)), false));
    }

    // Right edge (bottom to top)
    points.push((Point::new(Mm((x + w) * PT_TO_MM), Mm((y + h - r_tr) * PT_TO_MM)), false));

    // Top-right corner arc (from 0° to 90°)
    if r_tr > 0.0 {
        let cx = x + w - r_tr;
        let cy = y + h - r_tr;
        add_arc(&mut points, cx, cy, r_tr, 0.0, pi / 2.0);
    } else {
        points.push((Point::new(Mm((x + w) * PT_TO_MM), Mm((y + h) * PT_TO_MM)), false));
    }

    // Top edge (right to left)
    points.push((Point::new(Mm((x + r_tl) * PT_TO_MM), Mm((y + h) * PT_TO_MM)), false));

    // Top-left corner arc (from 90° to 180°)
    if r_tl > 0.0 {
        let cx = x + r_tl;
        let cy = y + h - r_tl;
        add_arc(&mut points, cx, cy, r_tl, pi / 2.0, pi);
    } else {
        points.push((Point::new(Mm(x * PT_TO_MM), Mm((y + h) * PT_TO_MM)), false));
    }

    // Left edge (top to bottom)
    points.push((Point::new(Mm(x * PT_TO_MM), Mm((y + r_bl) * PT_TO_MM)), false));

    // Bottom-left corner arc (from 180° to 270°)
    if r_bl > 0.0 {
        let cx = x + r_bl;
        let cy = y + r_bl;
        add_arc(&mut points, cx, cy, r_bl, pi, 3.0 * pi / 2.0);
    }

    if fill {
        let polygon = Polygon {
            rings: vec![points.clone()],
            mode: if stroke {
                PaintMode::FillStroke
            } else {
                PaintMode::Fill
            },
            winding_order: WindingOrder::NonZero,
        };
        layer.add_polygon(polygon);
    } else if stroke {
        let line = Line { points, is_closed: true };
        layer.add_line(line);
    }
}

fn draw_text(layout: &LayoutBox, layer: &PdfLayerReference, fonts: &Fonts) {
    let size = layout.font_size();
    let line_h = layout.line_height_multiplier();
    let text_align = layout.text_align();
    let box_width = layout.width;
    let metrics = layout.font_metrics();

    // Font selection
    let is_bold = layout.is_bold();
    let is_italic = layout.is_italic();
    let font = fonts.get(is_bold, is_italic);

    // Text color
    if let Some(ref color) = layout.node.style.color {
        set_fill_color(layer, color);
    } else {
        set_fill_color(layer, &Color::black());
    }

    let lines = if layout.lines.is_empty() {
        vec![layout.node.text.clone().unwrap_or_default()]
    } else {
        layout.lines.clone()
    };

    let line_height_px = line_height(size, line_h);
    
    // PDF text is drawn from the baseline, not the top.
    // We need to offset down from layout.y (which is the top of the text box)
    // by approximately the font's ascent. For most fonts, ascent ≈ 0.8 * font_size.
    let baseline_offset = size * 0.8;

    for (i, line) in lines.iter().enumerate() {
        let y = layout.y - baseline_offset - (i as f32 * line_height_px);

        let x = match text_align {
            TextAlign::Left => layout.x,
            TextAlign::Center => {
                let lw = text_width(line, size, metrics);
                layout.x + (box_width - lw) / 2.0
            }
            TextAlign::Right => {
                let lw = text_width(line, size, metrics);
                layout.x + box_width - lw
            }
            TextAlign::Justify => {
                // Justify: spread words across the line width
                // For the last line, use left alignment
                if i == lines.len() - 1 || line.split_whitespace().count() <= 1 {
                    layout.x
                } else {
                    draw_justified_line(layer, font, line, size, layout.x, y, box_width, metrics);
                    continue; // Skip normal drawing
                }
            }
        };

        layer.use_text(line, size, Mm(x * PT_TO_MM), Mm(y * PT_TO_MM), font);
    }
}

fn draw_justified_line(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    line: &str,
    size: f32,
    x: f32,
    y: f32,
    box_width: f32,
    metrics: &font_metrics::FontMetrics,
) {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() <= 1 {
        layer.use_text(line, size, Mm(x * PT_TO_MM), Mm(y * PT_TO_MM), font);
        return;
    }

    // Calculate total word width using real metrics
    let total_word_width: f32 = words.iter()
        .map(|w| text_width(w, size, metrics))
        .sum();

    // Calculate space between words
    let total_space = box_width - total_word_width;
    let space_per_gap = total_space / (words.len() - 1) as f32;

    let mut cursor_x = x;
    for (i, word) in words.iter().enumerate() {
        layer.use_text(*word, size, Mm(cursor_x * PT_TO_MM), Mm(y * PT_TO_MM), font);
        cursor_x += text_width(word, size, metrics);
        if i < words.len() - 1 {
            cursor_x += space_per_gap;
        }
    }
}

fn draw_image(layout: &LayoutBox, layer: &PdfLayerReference, doc: &PdfDocumentReference) {
    let src = match &layout.node.src {
        Some(s) if !s.is_empty() => s,
        Some(_) => {
            eprintln!("Image node has empty src");
            draw_image_placeholder(layout, layer);
            return;
        }
        None => {
            eprintln!("Image node missing src");
            draw_image_placeholder(layout, layer);
            return;
        }
    };

    eprintln!("Loading image: {} (first 100 chars)", &src[..src.len().min(100)]);

    // Try to load the image
    let image_data = if src.starts_with("data:") {
        // Base64 data URL
        load_base64_image(src)
    } else if src.starts_with("http://") || src.starts_with("https://") {
        // Remote URL - fetch it
        load_remote_image(src)
    } else {
        // Local file path
        load_local_image(src)
    };

    let image_data = match image_data {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to load image {}: {}", src, e);
            // Draw placeholder rectangle
            draw_image_placeholder(layout, layer);
            return;
        }
    };

    // Add image to PDF
    match add_image_to_pdf(doc, layer, &image_data, layout) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to add image to PDF: {}", e);
            draw_image_placeholder(layout, layer);
        }
    }
}

fn draw_image_placeholder(layout: &LayoutBox, layer: &PdfLayerReference) {
    // Draw a light gray rectangle as placeholder
    let x = layout.x;
    let y = layout.y - layout.height;
    let w = layout.width;
    let h = layout.height;

    // Gray background
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.9, 0.9, 0.9, None)));
    draw_rect(layer, x, y, w, h, true, false);

    // Border
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.7, 0.7, 0.7, None)));
    layer.set_outline_thickness(1.0);
    draw_rect(layer, x, y, w, h, false, true);
}

fn load_base64_image(data_url: &str) -> Result<Vec<u8>, String> {
    // Parse data URL: data:image/png;base64,xxxxx
    let parts: Vec<&str> = data_url.splitn(2, ',').collect();
    if parts.len() != 2 {
        return Err("Invalid data URL format".to_string());
    }

    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, parts[1])
        .map_err(|e| format!("Base64 decode error: {}", e))
}

fn load_remote_image(url: &str) -> Result<Vec<u8>, String> {
    // Use ureq for blocking HTTP (safe in async context)
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    let mut bytes = Vec::new();
    response.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    Ok(bytes)
}

fn load_local_image(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

fn add_image_to_pdf(
    _doc: &PdfDocumentReference,
    layer: &PdfLayerReference,
    image_data: &[u8],
    layout: &LayoutBox,
) -> Result<(), String> {
    // Check if we have any data
    if image_data.is_empty() {
        return Err("Image data is empty".to_string());
    }

    // Log first few bytes for debugging
    let preview: Vec<u8> = image_data.iter().take(16).cloned().collect();
    eprintln!("Image data: {} bytes, starts with: {:?}", image_data.len(), preview);

    // Decode the image using the external image crate
    let img = ::image::ImageReader::new(std::io::Cursor::new(image_data))
        .with_guessed_format()
        .map_err(|e| format!("Failed to guess image format: {}", e))?
        .decode()
        .map_err(|e| format!("Failed to decode image (len={}): {}", image_data.len(), e))?;

    let img_width = img.width();
    let img_height = img.height();

    // Convert to RGB8
    let rgb_image = img.to_rgb8();
    let raw_pixels = rgb_image.into_raw();

    // Create printpdf Image
    let image = printpdf::Image::from(
        printpdf::ImageXObject {
            width: Px(img_width as usize),
            height: Px(img_height as usize),
            color_space: printpdf::ColorSpace::Rgb,
            bits_per_component: printpdf::ColorBits::Bit8,
            interpolate: true,
            image_data: raw_pixels,
            image_filter: None,
            clipping_bbox: None,
            smask: None,
        }
    );

    // Calculate position and size based on objectFit
    let container_x = layout.x;
    let container_y = layout.y - layout.height;
    let container_w = layout.width;
    let container_h = layout.height;

    let img_w = img_width as f32;
    let img_h = img_height as f32;
    let img_aspect = img_w / img_h;
    let container_aspect = container_w / container_h;

    use crate::layout::ObjectFit;
    let object_fit = layout.node.style.object_fit.clone().unwrap_or_default();

    let (render_w, render_h, offset_x, offset_y) = match object_fit {
        ObjectFit::Fill => {
            // Stretch to fill exactly (distorts aspect ratio)
            (container_w, container_h, 0.0, 0.0)
        }
        ObjectFit::Contain => {
            // Scale to fit entirely within container (may have empty space)
            let (w, h) = if img_aspect > container_aspect {
                // Image is wider - fit to width
                (container_w, container_w / img_aspect)
            } else {
                // Image is taller - fit to height
                (container_h * img_aspect, container_h)
            };
            // Center the image
            let ox = (container_w - w) / 2.0;
            let oy = (container_h - h) / 2.0;
            (w, h, ox, oy)
        }
        ObjectFit::Cover => {
            // Scale to fill container, cropping if necessary
            let (w, h) = if img_aspect > container_aspect {
                // Image is wider - fit to height, crop width
                (container_h * img_aspect, container_h)
            } else {
                // Image is taller - fit to width, crop height
                (container_w, container_w / img_aspect)
            };
            // Center the image (overflow will be clipped)
            let ox = (container_w - w) / 2.0;
            let oy = (container_h - h) / 2.0;
            (w, h, ox, oy)
        }
        ObjectFit::None => {
            // No scaling, use original size, centered
            let ox = (container_w - img_w) / 2.0;
            let oy = (container_h - img_h) / 2.0;
            (img_w, img_h, ox, oy)
        }
        ObjectFit::ScaleDown => {
            // Use smaller of none or contain
            let (contain_w, contain_h, _, _) = if img_aspect > container_aspect {
                (container_w, container_w / img_aspect, 0.0, 0.0)
            } else {
                (container_h * img_aspect, container_h, 0.0, 0.0)
            };
            
            let (w, h) = if img_w <= contain_w && img_h <= contain_h {
                // Original is smaller, use it
                (img_w, img_h)
            } else {
                // Use contain
                (contain_w, contain_h)
            };
            let ox = (container_w - w) / 2.0;
            let oy = (container_h - h) / 2.0;
            (w, h, ox, oy)
        }
    };

    let final_x = container_x + offset_x;
    let final_y = container_y + offset_y;

    // Match layout units (points) to PDF image units by forcing 72 DPI so that
    // 1px == 1pt; then scale to the requested render_w/render_h.
    let dpi = 72.0;
    let scale_x = render_w / img_w;
    let scale_y = render_h / img_h;

    // Add image to layer with transformation
    image.add_to_layer(
        layer.clone(),
        printpdf::ImageTransform {
            translate_x: Some(Mm(final_x * PT_TO_MM)),
            translate_y: Some(Mm(final_y * PT_TO_MM)),
            scale_x: Some(scale_x),
            scale_y: Some(scale_y),
            dpi: Some(dpi),
            ..Default::default()
        },
    );

    Ok(())
}

// ============================================================================
// SVG RENDERING
// ============================================================================

fn draw_svg(layout: &LayoutBox, layer: &PdfLayerReference) {
    // Check both `src` and `content` fields for SVG data
    let src = match (&layout.node.src, &layout.node.content) {
        (Some(s), _) if !s.is_empty() => s.clone(),
        (_, Some(c)) if !c.is_empty() => c.clone(), // Use content field if src is empty
        _ => {
            eprintln!("SVG node missing src/content");
            draw_svg_placeholder(layout, layer);
            return;
        }
    };
    let src = &src;

    // Load SVG content
    let svg_content = if src.starts_with("data:") {
        // Data URL (base64 or plain)
        load_svg_data_url(src)
    } else if src.starts_with("http://") || src.starts_with("https://") {
        // Remote URL
        load_remote_svg(src)
    } else if src.starts_with("<svg") || src.starts_with("<?xml") {
        // Inline SVG content
        Ok(src.clone())
    } else {
        // Local file
        load_local_svg(src)
    };

    let svg_content = match svg_content {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to load SVG: {}", e);
            draw_svg_placeholder(layout, layer);
            return;
        }
    };

    // Parse SVG
    let svg_doc = match svg::parse_svg(&svg_content) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Failed to parse SVG: {}", e);
            draw_svg_placeholder(layout, layer);
            return;
        }
    };

    // Render SVG to PDF layer
    render_svg_to_layer(&svg_doc, layout, layer);
}

fn draw_svg_placeholder(layout: &LayoutBox, layer: &PdfLayerReference) {
    let x = layout.x;
    let y = layout.y - layout.height;
    let w = layout.width;
    let h = layout.height;

    // Light gray background with X pattern
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.95, 0.95, 0.95, None)));
    draw_rect(layer, x, y, w, h, true, false);

    // Border
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.8, 0.8, 0.8, None)));
    layer.set_outline_thickness(1.0);
    draw_rect(layer, x, y, w, h, false, true);
}

fn load_svg_data_url(data_url: &str) -> Result<String, String> {
    // data:image/svg+xml;base64,xxxxx or data:image/svg+xml,<svg>...</svg>
    if let Some(comma_pos) = data_url.find(',') {
        let header = &data_url[..comma_pos];
        let content = &data_url[comma_pos + 1..];

        if header.contains("base64") {
            // Base64 encoded
            let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content)
                .map_err(|e| format!("Base64 decode error: {}", e))?;
            String::from_utf8(decoded)
                .map_err(|e| format!("UTF-8 decode error: {}", e))
        } else {
            // URL encoded or plain
            Ok(urlencoding::decode(content)
                .unwrap_or(std::borrow::Cow::Borrowed(content))
                .to_string())
        }
    } else {
        Err("Invalid data URL format".to_string())
    }
}

fn load_remote_svg(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    response.into_string()
        .map_err(|e| format!("Failed to read response: {}", e))
}

fn load_local_svg(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

fn render_svg_to_layer(svg_doc: &SvgDocument, layout: &LayoutBox, layer: &PdfLayerReference) {
    // Calculate scale to fit SVG into layout bounds
    let svg_width = svg_doc.view_box.as_ref().map(|vb| vb.width).unwrap_or(svg_doc.width);
    let svg_height = svg_doc.view_box.as_ref().map(|vb| vb.height).unwrap_or(svg_doc.height);
    
    let scale_x = layout.width / svg_width;
    let scale_y = layout.height / svg_height;
    let scale = scale_x.min(scale_y); // Preserve aspect ratio

    // Calculate offset to position SVG in layout
    let offset_x = layout.x;
    let offset_y = layout.y; // PDF y is from bottom

    // Render each element
    for element in &svg_doc.elements {
        render_svg_element(element, layer, offset_x, offset_y, scale, svg_height);
    }
}

fn render_svg_element(
    element: &SvgElement,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    svg_height: f32,
) {
    match element {
        SvgElement::Path(path) => {
            render_svg_path(path, layer, offset_x, offset_y, scale, svg_height);
        }
        SvgElement::Rect(rect) => {
            render_svg_rect(rect, layer, offset_x, offset_y, scale, svg_height);
        }
        SvgElement::Circle(circle) => {
            render_svg_circle(circle, layer, offset_x, offset_y, scale, svg_height);
        }
        SvgElement::Ellipse(ellipse) => {
            render_svg_ellipse(ellipse, layer, offset_x, offset_y, scale, svg_height);
        }
        SvgElement::Line(line) => {
            render_svg_line(line, layer, offset_x, offset_y, scale, svg_height);
        }
        SvgElement::Polyline(polyline) => {
            render_svg_polyline(polyline, layer, offset_x, offset_y, scale, svg_height);
        }
        SvgElement::Polygon(polygon) => {
            render_svg_polygon(polygon, layer, offset_x, offset_y, scale, svg_height);
        }
        SvgElement::Group(group) => {
            // Apply group transform and render children
            for child in &group.elements {
                render_svg_element(child, layer, offset_x, offset_y, scale, svg_height);
            }
        }
    }
}

fn render_svg_path(
    path: &svg::SvgPath,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    svg_height: f32,
) {
    if path.commands.is_empty() {
        return;
    }

    // Convert SVG path to PDF points
    let mut points: Vec<(Point, bool)> = Vec::new();
    let mut current_x = 0.0f32;
    let mut current_y = 0.0f32;
    let mut start_x = 0.0f32;
    let mut start_y = 0.0f32;

    // Helper to convert SVG coords to PDF coords
    let to_pdf = |x: f32, y: f32| -> (f32, f32) {
        let px = offset_x + x * scale;
        let py = offset_y - (y * scale); // Flip Y and offset from top
        (px, py)
    };

    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo(x, y) => {
                current_x = *x;
                current_y = *y;
                start_x = current_x;
                start_y = current_y;
                let (px, py) = to_pdf(current_x, current_y);
                points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
            }
            PathCommand::LineTo(x, y) => {
                current_x = *x;
                current_y = *y;
                let (px, py) = to_pdf(current_x, current_y);
                points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
            }
            PathCommand::HorizontalLineTo(x) => {
                current_x = *x;
                let (px, py) = to_pdf(current_x, current_y);
                points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
            }
            PathCommand::VerticalLineTo(y) => {
                current_y = *y;
                let (px, py) = to_pdf(current_x, current_y);
                points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
            }
            PathCommand::CurveTo(x1, y1, x2, y2, x, y) => {
                // Approximate cubic bezier with line segments
                let steps = 10;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let t2 = t * t;
                    let t3 = t2 * t;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    let mt3 = mt2 * mt;

                    let bx = mt3 * current_x + 3.0 * mt2 * t * x1 + 3.0 * mt * t2 * x2 + t3 * x;
                    let by = mt3 * current_y + 3.0 * mt2 * t * y1 + 3.0 * mt * t2 * y2 + t3 * y;
                    let (px, py) = to_pdf(bx, by);
                    points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
                }
                current_x = *x;
                current_y = *y;
            }
            PathCommand::QuadraticCurveTo(x1, y1, x, y) => {
                // Approximate quadratic bezier with line segments
                let steps = 8;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let mt = 1.0 - t;
                    let bx = mt * mt * current_x + 2.0 * mt * t * x1 + t * t * x;
                    let by = mt * mt * current_y + 2.0 * mt * t * y1 + t * t * y;
                    let (px, py) = to_pdf(bx, by);
                    points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
                }
                current_x = *x;
                current_y = *y;
            }
            PathCommand::ArcTo(rx, ry, rotation, large_arc, sweep, x, y) => {
                // Approximate arc with line segments (simplified)
                let steps = 16;
                let dx = x - current_x;
                let dy = y - current_y;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let ax = current_x + dx * t;
                    let ay = current_y + dy * t;
                    let (px, py) = to_pdf(ax, ay);
                    points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
                }
                current_x = *x;
                current_y = *y;
                // Suppress unused warnings
                let _ = (rx, ry, rotation, large_arc, sweep);
            }
            PathCommand::SmoothCurveTo(x2, y2, x, y) => {
                // Simplified: treat as line
                current_x = *x;
                current_y = *y;
                let (px, py) = to_pdf(current_x, current_y);
                points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
                let _ = (x2, y2);
            }
            PathCommand::SmoothQuadraticCurveTo(x, y) => {
                current_x = *x;
                current_y = *y;
                let (px, py) = to_pdf(current_x, current_y);
                points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
            }
            PathCommand::ClosePath => {
                let (px, py) = to_pdf(start_x, start_y);
                points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
            }
        }
    }

    if points.is_empty() {
        return;
    }

    // Set style
    let has_fill = path.style.fill.is_some();
    let has_stroke = path.style.stroke.is_some();

    if let Some(ref fill) = path.style.fill {
        set_fill_color(layer, fill);
    }
    if let Some(ref stroke) = path.style.stroke {
        set_stroke_color(layer, stroke);
        layer.set_outline_thickness(path.style.stroke_width * scale);
    }

    // Draw the path
    let mode = match (has_fill, has_stroke) {
        (true, true) => PaintMode::FillStroke,
        (true, false) => PaintMode::Fill,
        (false, true) => PaintMode::Stroke,
        (false, false) => return,
    };

    let polygon = Polygon {
        rings: vec![points],
        mode,
        winding_order: WindingOrder::NonZero,
    };
    layer.add_polygon(polygon);
}

fn render_svg_rect(
    rect: &svg::SvgRect,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    _svg_height: f32,
) {
    let x = offset_x + rect.x * scale;
    let y = offset_y - (rect.y + rect.height) * scale;
    let w = rect.width * scale;
    let h = rect.height * scale;

    let has_fill = rect.style.fill.is_some();
    let has_stroke = rect.style.stroke.is_some();

    if let Some(ref fill) = rect.style.fill {
        set_fill_color(layer, fill);
    }
    if let Some(ref stroke) = rect.style.stroke {
        set_stroke_color(layer, stroke);
        layer.set_outline_thickness(rect.style.stroke_width * scale);
    }

    if rect.rx > 0.0 || rect.ry > 0.0 {
        let r = rect.rx.max(rect.ry) * scale;
        draw_rounded_rect_corners(layer, x, y, w, h, r, r, r, r, has_fill, has_stroke);
    } else {
        if has_fill {
            draw_rect(layer, x, y, w, h, true, false);
        }
        if has_stroke {
            draw_rect(layer, x, y, w, h, false, true);
        }
    }
}

fn render_svg_circle(
    circle: &svg::SvgCircle,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    _svg_height: f32,
) {
    let cx = offset_x + circle.cx * scale;
    let cy = offset_y - circle.cy * scale;
    let r = circle.r * scale;

    let has_fill = circle.style.fill.is_some();
    let has_stroke = circle.style.stroke.is_some();

    if let Some(ref fill) = circle.style.fill {
        set_fill_color(layer, fill);
    }
    if let Some(ref stroke) = circle.style.stroke {
        set_stroke_color(layer, stroke);
        layer.set_outline_thickness(circle.style.stroke_width * scale);
    }

    // Approximate circle with polygon
    let segments = 32;
    let mut points: Vec<(Point, bool)> = Vec::new();
    for i in 0..=segments {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
        let px = cx + r * angle.cos();
        let py = cy + r * angle.sin();
        points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
    }

    let mode = match (has_fill, has_stroke) {
        (true, true) => PaintMode::FillStroke,
        (true, false) => PaintMode::Fill,
        (false, true) => PaintMode::Stroke,
        (false, false) => return,
    };

    let polygon = Polygon {
        rings: vec![points],
        mode,
        winding_order: WindingOrder::NonZero,
    };
    layer.add_polygon(polygon);
}

fn render_svg_ellipse(
    ellipse: &svg::SvgEllipse,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    _svg_height: f32,
) {
    let cx = offset_x + ellipse.cx * scale;
    let cy = offset_y - ellipse.cy * scale;
    let rx = ellipse.rx * scale;
    let ry = ellipse.ry * scale;

    let has_fill = ellipse.style.fill.is_some();
    let has_stroke = ellipse.style.stroke.is_some();

    if let Some(ref fill) = ellipse.style.fill {
        set_fill_color(layer, fill);
    }
    if let Some(ref stroke) = ellipse.style.stroke {
        set_stroke_color(layer, stroke);
        layer.set_outline_thickness(ellipse.style.stroke_width * scale);
    }

    // Approximate ellipse with polygon
    let segments = 32;
    let mut points: Vec<(Point, bool)> = Vec::new();
    for i in 0..=segments {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
        let px = cx + rx * angle.cos();
        let py = cy + ry * angle.sin();
        points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
    }

    let mode = match (has_fill, has_stroke) {
        (true, true) => PaintMode::FillStroke,
        (true, false) => PaintMode::Fill,
        (false, true) => PaintMode::Stroke,
        (false, false) => return,
    };

    let polygon = Polygon {
        rings: vec![points],
        mode,
        winding_order: WindingOrder::NonZero,
    };
    layer.add_polygon(polygon);
}

fn render_svg_line(
    line: &svg::SvgLine,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    _svg_height: f32,
) {
    if line.style.stroke.is_none() {
        return;
    }

    let x1 = offset_x + line.x1 * scale;
    let y1 = offset_y - line.y1 * scale;
    let x2 = offset_x + line.x2 * scale;
    let y2 = offset_y - line.y2 * scale;

    if let Some(ref stroke) = line.style.stroke {
        set_stroke_color(layer, stroke);
        layer.set_outline_thickness(line.style.stroke_width * scale);
    }

    let points = vec![
        (Point::new(Mm(x1 * PT_TO_MM), Mm(y1 * PT_TO_MM)), false),
        (Point::new(Mm(x2 * PT_TO_MM), Mm(y2 * PT_TO_MM)), false),
    ];

    let line_shape = printpdf::Line {
        points,
        is_closed: false,
    };
    layer.add_line(line_shape);
}

fn render_svg_polyline(
    polyline: &svg::SvgPolyline,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    _svg_height: f32,
) {
    if polyline.points.is_empty() {
        return;
    }

    if let Some(ref stroke) = polyline.style.stroke {
        set_stroke_color(layer, stroke);
        layer.set_outline_thickness(polyline.style.stroke_width * scale);
    }

    let points: Vec<(Point, bool)> = polyline.points.iter()
        .map(|(x, y)| {
            let px = offset_x + x * scale;
            let py = offset_y - y * scale;
            (Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false)
        })
        .collect();

    let line_shape = printpdf::Line {
        points,
        is_closed: false,
    };
    layer.add_line(line_shape);
}

fn render_svg_polygon(
    polygon: &svg::SvgPolygon,
    layer: &PdfLayerReference,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    _svg_height: f32,
) {
    if polygon.points.is_empty() {
        return;
    }

    let has_fill = polygon.style.fill.is_some();
    let has_stroke = polygon.style.stroke.is_some();

    if let Some(ref fill) = polygon.style.fill {
        set_fill_color(layer, fill);
    }
    if let Some(ref stroke) = polygon.style.stroke {
        set_stroke_color(layer, stroke);
        layer.set_outline_thickness(polygon.style.stroke_width * scale);
    }

    let mut points: Vec<(Point, bool)> = polygon.points.iter()
        .map(|(x, y)| {
            let px = offset_x + x * scale;
            let py = offset_y - y * scale;
            (Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false)
        })
        .collect();

    // Close the polygon
    if let Some(first) = polygon.points.first() {
        let px = offset_x + first.0 * scale;
        let py = offset_y - first.1 * scale;
        points.push((Point::new(Mm(px * PT_TO_MM), Mm(py * PT_TO_MM)), false));
    }

    let mode = match (has_fill, has_stroke) {
        (true, true) => PaintMode::FillStroke,
        (true, false) => PaintMode::Fill,
        (false, true) => PaintMode::Stroke,
        (false, false) => return,
    };

    let poly = Polygon {
        rings: vec![points],
        mode,
        winding_order: WindingOrder::NonZero,
    };
    layer.add_polygon(poly);
}

// ============================================================================
// COLOR HELPERS
// ============================================================================

fn set_fill_color(layer: &PdfLayerReference, color: &Color) {
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        None,
    )));
}

fn set_stroke_color(layer: &PdfLayerReference, color: &Color) {
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        None,
    )));
}
