// SVG parsing and rendering for PDF
// Supports basic SVG paths, transforms, and common shapes

use crate::layout::Color;

/// Parsed SVG document
#[derive(Debug, Clone)]
pub struct SvgDocument {
    pub width: f32,
    pub height: f32,
    pub view_box: Option<ViewBox>,
    pub elements: Vec<SvgElement>,
}

#[derive(Debug, Clone)]
pub struct ViewBox {
    pub min_x: f32,
    pub min_y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub enum SvgElement {
    Path(SvgPath),
    Rect(SvgRect),
    Circle(SvgCircle),
    Ellipse(SvgEllipse),
    Line(SvgLine),
    Polyline(SvgPolyline),
    Polygon(SvgPolygon),
    Group(SvgGroup),
}

#[derive(Debug, Clone, Default)]
pub struct SvgStyle {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub opacity: f32,
    pub fill_opacity: f32,
    pub stroke_opacity: f32,
}

impl SvgStyle {
    pub fn new() -> Self {
        Self {
            fill: Some(Color { r: 0, g: 0, b: 0, a: 1.0 }), // Default black fill
            stroke: None,
            stroke_width: 1.0,
            opacity: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Transform {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotate: f32, // degrees
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotate: 0.0,
        }
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        // Apply scale
        let x = x * self.scale_x;
        let y = y * self.scale_y;

        // Apply rotation (around origin)
        let rad = self.rotate.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();
        let x = x * cos_r - y * sin_r;
        let y = x * sin_r + y * cos_r;

        // Apply translation
        (x + self.translate_x, y + self.translate_y)
    }
}

#[derive(Debug, Clone)]
pub struct SvgPath {
    pub commands: Vec<PathCommand>,
    pub style: SvgStyle,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct SvgRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rx: f32,
    pub ry: f32,
    pub style: SvgStyle,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct SvgCircle {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub style: SvgStyle,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct SvgEllipse {
    pub cx: f32,
    pub cy: f32,
    pub rx: f32,
    pub ry: f32,
    pub style: SvgStyle,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct SvgLine {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub style: SvgStyle,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct SvgPolyline {
    pub points: Vec<(f32, f32)>,
    pub style: SvgStyle,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct SvgPolygon {
    pub points: Vec<(f32, f32)>,
    pub style: SvgStyle,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct SvgGroup {
    pub elements: Vec<SvgElement>,
    pub style: SvgStyle,
    pub transform: Transform,
}

/// SVG path commands
#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo(f32, f32),           // M/m
    LineTo(f32, f32),           // L/l
    HorizontalLineTo(f32),      // H/h
    VerticalLineTo(f32),        // V/v
    CurveTo(f32, f32, f32, f32, f32, f32), // C/c (cubic bezier)
    SmoothCurveTo(f32, f32, f32, f32),     // S/s
    QuadraticCurveTo(f32, f32, f32, f32),  // Q/q
    SmoothQuadraticCurveTo(f32, f32),      // T/t
    ArcTo(f32, f32, f32, bool, bool, f32, f32), // A/a
    ClosePath,                  // Z/z
}

/// Parse SVG from string
pub fn parse_svg(svg_content: &str) -> Result<SvgDocument, String> {
    // Simple XML-like parsing (not a full XML parser)
    let mut doc = SvgDocument {
        width: 100.0,
        height: 100.0,
        view_box: None,
        elements: Vec::new(),
    };

    // Extract root SVG attributes
    if let Some(svg_tag) = extract_tag(svg_content, "svg") {
        doc.width = extract_attr_f32(&svg_tag, "width").unwrap_or(100.0);
        doc.height = extract_attr_f32(&svg_tag, "height").unwrap_or(100.0);
        
        if let Some(vb) = extract_attr(&svg_tag, "viewBox") {
            let parts: Vec<f32> = vb.split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() == 4 {
                doc.view_box = Some(ViewBox {
                    min_x: parts[0],
                    min_y: parts[1],
                    width: parts[2],
                    height: parts[3],
                });
            }
        }
    }

    // Parse elements
    doc.elements = parse_elements(svg_content);

    Ok(doc)
}

fn parse_elements(content: &str) -> Vec<SvgElement> {
    let mut elements = Vec::new();

    // Parse paths
    for path_tag in extract_all_tags(content, "path") {
        if let Some(d) = extract_attr(&path_tag, "d") {
            let commands = parse_path_data(&d);
            let style = parse_style(&path_tag);
            let transform = parse_transform(&path_tag);
            elements.push(SvgElement::Path(SvgPath { commands, style, transform }));
        }
    }

    // Parse rects
    for rect_tag in extract_all_tags(content, "rect") {
        let rect = SvgRect {
            x: extract_attr_f32(&rect_tag, "x").unwrap_or(0.0),
            y: extract_attr_f32(&rect_tag, "y").unwrap_or(0.0),
            width: extract_attr_f32(&rect_tag, "width").unwrap_or(0.0),
            height: extract_attr_f32(&rect_tag, "height").unwrap_or(0.0),
            rx: extract_attr_f32(&rect_tag, "rx").unwrap_or(0.0),
            ry: extract_attr_f32(&rect_tag, "ry").unwrap_or(0.0),
            style: parse_style(&rect_tag),
            transform: parse_transform(&rect_tag),
        };
        elements.push(SvgElement::Rect(rect));
    }

    // Parse circles
    for circle_tag in extract_all_tags(content, "circle") {
        let circle = SvgCircle {
            cx: extract_attr_f32(&circle_tag, "cx").unwrap_or(0.0),
            cy: extract_attr_f32(&circle_tag, "cy").unwrap_or(0.0),
            r: extract_attr_f32(&circle_tag, "r").unwrap_or(0.0),
            style: parse_style(&circle_tag),
            transform: parse_transform(&circle_tag),
        };
        elements.push(SvgElement::Circle(circle));
    }

    // Parse ellipses
    for ellipse_tag in extract_all_tags(content, "ellipse") {
        let ellipse = SvgEllipse {
            cx: extract_attr_f32(&ellipse_tag, "cx").unwrap_or(0.0),
            cy: extract_attr_f32(&ellipse_tag, "cy").unwrap_or(0.0),
            rx: extract_attr_f32(&ellipse_tag, "rx").unwrap_or(0.0),
            ry: extract_attr_f32(&ellipse_tag, "ry").unwrap_or(0.0),
            style: parse_style(&ellipse_tag),
            transform: parse_transform(&ellipse_tag),
        };
        elements.push(SvgElement::Ellipse(ellipse));
    }

    // Parse lines
    for line_tag in extract_all_tags(content, "line") {
        let line = SvgLine {
            x1: extract_attr_f32(&line_tag, "x1").unwrap_or(0.0),
            y1: extract_attr_f32(&line_tag, "y1").unwrap_or(0.0),
            x2: extract_attr_f32(&line_tag, "x2").unwrap_or(0.0),
            y2: extract_attr_f32(&line_tag, "y2").unwrap_or(0.0),
            style: parse_style(&line_tag),
            transform: parse_transform(&line_tag),
        };
        elements.push(SvgElement::Line(line));
    }

    // Parse polylines
    for polyline_tag in extract_all_tags(content, "polyline") {
        if let Some(points_str) = extract_attr(&polyline_tag, "points") {
            let points = parse_points(&points_str);
            elements.push(SvgElement::Polyline(SvgPolyline {
                points,
                style: parse_style(&polyline_tag),
                transform: parse_transform(&polyline_tag),
            }));
        }
    }

    // Parse polygons
    for polygon_tag in extract_all_tags(content, "polygon") {
        if let Some(points_str) = extract_attr(&polygon_tag, "points") {
            let points = parse_points(&points_str);
            elements.push(SvgElement::Polygon(SvgPolygon {
                points,
                style: parse_style(&polygon_tag),
                transform: parse_transform(&polygon_tag),
            }));
        }
    }

    // Parse groups (g elements)
    for group_tag in extract_all_tags(content, "g") {
        let inner_content = extract_inner_content(&group_tag);
        let inner_elements = parse_elements(&inner_content);
        if !inner_elements.is_empty() {
            elements.push(SvgElement::Group(SvgGroup {
                elements: inner_elements,
                style: parse_style(&group_tag),
                transform: parse_transform(&group_tag),
            }));
        }
    }

    elements
}

fn parse_points(points_str: &str) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    let nums: Vec<f32> = points_str
        .replace(',', " ")
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    
    for chunk in nums.chunks(2) {
        if chunk.len() == 2 {
            points.push((chunk[0], chunk[1]));
        }
    }
    points
}

fn parse_style(tag: &str) -> SvgStyle {
    let mut style = SvgStyle::new();

    // Parse fill
    if let Some(fill) = extract_attr(tag, "fill") {
        if fill == "none" {
            style.fill = None;
        } else {
            style.fill = parse_color(&fill);
        }
    }

    // Parse stroke
    if let Some(stroke) = extract_attr(tag, "stroke") {
        if stroke != "none" {
            style.stroke = parse_color(&stroke);
        }
    }

    // Parse stroke-width
    if let Some(sw) = extract_attr_f32(tag, "stroke-width") {
        style.stroke_width = sw;
    }

    // Parse opacity
    if let Some(op) = extract_attr_f32(tag, "opacity") {
        style.opacity = op;
    }

    // Parse fill-opacity
    if let Some(fo) = extract_attr_f32(tag, "fill-opacity") {
        style.fill_opacity = fo;
    }

    // Parse stroke-opacity
    if let Some(so) = extract_attr_f32(tag, "stroke-opacity") {
        style.stroke_opacity = so;
    }

    // Parse style attribute (CSS-like)
    if let Some(style_attr) = extract_attr(tag, "style") {
        for part in style_attr.split(';') {
            let kv: Vec<&str> = part.split(':').map(|s| s.trim()).collect();
            if kv.len() == 2 {
                match kv[0] {
                    "fill" => {
                        if kv[1] == "none" {
                            style.fill = None;
                        } else {
                            style.fill = parse_color(kv[1]);
                        }
                    }
                    "stroke" => {
                        if kv[1] != "none" {
                            style.stroke = parse_color(kv[1]);
                        }
                    }
                    "stroke-width" => {
                        if let Ok(sw) = kv[1].trim_end_matches("px").parse() {
                            style.stroke_width = sw;
                        }
                    }
                    "opacity" => {
                        if let Ok(op) = kv[1].parse() {
                            style.opacity = op;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    style
}

fn parse_transform(tag: &str) -> Transform {
    let mut transform = Transform::identity();

    if let Some(transform_attr) = extract_attr(tag, "transform") {
        // Parse translate(x, y)
        if let Some(start) = transform_attr.find("translate(") {
            let rest = &transform_attr[start + 10..];
            if let Some(end) = rest.find(')') {
                let params = &rest[..end];
                let nums: Vec<f32> = params
                    .replace(',', " ")
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if !nums.is_empty() {
                    transform.translate_x = nums[0];
                }
                if nums.len() > 1 {
                    transform.translate_y = nums[1];
                }
            }
        }

        // Parse scale(x, y)
        if let Some(start) = transform_attr.find("scale(") {
            let rest = &transform_attr[start + 6..];
            if let Some(end) = rest.find(')') {
                let params = &rest[..end];
                let nums: Vec<f32> = params
                    .replace(',', " ")
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if !nums.is_empty() {
                    transform.scale_x = nums[0];
                    transform.scale_y = if nums.len() > 1 { nums[1] } else { nums[0] };
                }
            }
        }

        // Parse rotate(angle)
        if let Some(start) = transform_attr.find("rotate(") {
            let rest = &transform_attr[start + 7..];
            if let Some(end) = rest.find(')') {
                let params = &rest[..end];
                let nums: Vec<f32> = params
                    .replace(',', " ")
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if !nums.is_empty() {
                    transform.rotate = nums[0];
                }
            }
        }
    }

    transform
}

fn parse_color(color_str: &str) -> Option<Color> {
    let color_str = color_str.trim();

    // Handle hex colors
    if color_str.starts_with('#') {
        let hex = &color_str[1..];
        if hex.len() == 3 {
            // Short form #RGB
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            return Some(Color { r, g, b, a: 1.0 });
        } else if hex.len() == 6 {
            // Full form #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color { r, g, b, a: 1.0 });
        }
    }

    // Handle rgb() colors
    if color_str.starts_with("rgb(") {
        let inner = &color_str[4..color_str.len() - 1];
        let parts: Vec<u8> = inner
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if parts.len() == 3 {
            return Some(Color { r: parts[0], g: parts[1], b: parts[2], a: 1.0 });
        }
    }

    // Handle rgba() colors
    if color_str.starts_with("rgba(") {
        let inner = &color_str[5..color_str.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() == 4 {
            let r: u8 = parts[0].parse().ok()?;
            let g: u8 = parts[1].parse().ok()?;
            let b: u8 = parts[2].parse().ok()?;
            let a: f32 = parts[3].parse().ok()?;
            return Some(Color { r, g, b, a });
        }
    }

    // Handle named colors
    match color_str.to_lowercase().as_str() {
        "black" => Some(Color { r: 0, g: 0, b: 0, a: 1.0 }),
        "white" => Some(Color { r: 255, g: 255, b: 255, a: 1.0 }),
        "red" => Some(Color { r: 255, g: 0, b: 0, a: 1.0 }),
        "green" => Some(Color { r: 0, g: 128, b: 0, a: 1.0 }),
        "blue" => Some(Color { r: 0, g: 0, b: 255, a: 1.0 }),
        "yellow" => Some(Color { r: 255, g: 255, b: 0, a: 1.0 }),
        "cyan" => Some(Color { r: 0, g: 255, b: 255, a: 1.0 }),
        "magenta" => Some(Color { r: 255, g: 0, b: 255, a: 1.0 }),
        "gray" | "grey" => Some(Color { r: 128, g: 128, b: 128, a: 1.0 }),
        "orange" => Some(Color { r: 255, g: 165, b: 0, a: 1.0 }),
        "purple" => Some(Color { r: 128, g: 0, b: 128, a: 1.0 }),
        "pink" => Some(Color { r: 255, g: 192, b: 203, a: 1.0 }),
        "brown" => Some(Color { r: 165, g: 42, b: 42, a: 1.0 }),
        "currentcolor" | "currentColor" => Some(Color { r: 0, g: 0, b: 0, a: 1.0 }),
        _ => None,
    }
}

/// Parse SVG path data string into commands
fn parse_path_data(d: &str) -> Vec<PathCommand> {
    let mut commands = Vec::new();
    let mut chars = d.chars().peekable();
    let mut current_cmd = ' ';
    let mut current_x = 0.0f32;
    let mut current_y = 0.0f32;

    fn parse_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<f32> {
        // Skip whitespace and commas
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == ',' {
                chars.next();
            } else {
                break;
            }
        }

        let mut num_str = String::new();
        
        // Handle sign
        if let Some(&c) = chars.peek() {
            if c == '-' || c == '+' {
                num_str.push(chars.next().unwrap());
            }
        }

        // Collect digits and decimal point
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                num_str.push(chars.next().unwrap());
            } else if c == 'e' || c == 'E' {
                // Scientific notation
                num_str.push(chars.next().unwrap());
                if let Some(&sign) = chars.peek() {
                    if sign == '-' || sign == '+' {
                        num_str.push(chars.next().unwrap());
                    }
                }
            } else {
                break;
            }
        }

        if num_str.is_empty() {
            None
        } else {
            num_str.parse().ok()
        }
    }

    while let Some(c) = chars.next() {
        if c.is_whitespace() || c == ',' {
            continue;
        }

        if c.is_alphabetic() {
            current_cmd = c;
        } else {
            // Put back the character for number parsing
            let mut temp = c.to_string();
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' || next == '-' || next == '+' || next == 'e' || next == 'E' {
                    temp.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            
            // Re-parse with the number
            let num: f32 = match temp.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            match current_cmd {
                'M' => {
                    current_x = num;
                    if let Some(y) = parse_number(&mut chars) {
                        current_y = y;
                        commands.push(PathCommand::MoveTo(current_x, current_y));
                        current_cmd = 'L'; // Subsequent coordinates are LineTo
                    }
                }
                'm' => {
                    current_x += num;
                    if let Some(dy) = parse_number(&mut chars) {
                        current_y += dy;
                        commands.push(PathCommand::MoveTo(current_x, current_y));
                        current_cmd = 'l';
                    }
                }
                'L' => {
                    current_x = num;
                    if let Some(y) = parse_number(&mut chars) {
                        current_y = y;
                        commands.push(PathCommand::LineTo(current_x, current_y));
                    }
                }
                'l' => {
                    current_x += num;
                    if let Some(dy) = parse_number(&mut chars) {
                        current_y += dy;
                        commands.push(PathCommand::LineTo(current_x, current_y));
                    }
                }
                'H' => {
                    current_x = num;
                    commands.push(PathCommand::HorizontalLineTo(current_x));
                }
                'h' => {
                    current_x += num;
                    commands.push(PathCommand::HorizontalLineTo(current_x));
                }
                'V' => {
                    current_y = num;
                    commands.push(PathCommand::VerticalLineTo(current_y));
                }
                'v' => {
                    current_y += num;
                    commands.push(PathCommand::VerticalLineTo(current_y));
                }
                'C' => {
                    let x1 = num;
                    if let (Some(y1), Some(x2), Some(y2), Some(x), Some(y)) = (
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                    ) {
                        current_x = x;
                        current_y = y;
                        commands.push(PathCommand::CurveTo(x1, y1, x2, y2, x, y));
                    }
                }
                'c' => {
                    let dx1 = num;
                    if let (Some(dy1), Some(dx2), Some(dy2), Some(dx), Some(dy)) = (
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                    ) {
                        let x1 = current_x + dx1;
                        let y1 = current_y + dy1;
                        let x2 = current_x + dx2;
                        let y2 = current_y + dy2;
                        current_x += dx;
                        current_y += dy;
                        commands.push(PathCommand::CurveTo(x1, y1, x2, y2, current_x, current_y));
                    }
                }
                'Q' => {
                    let x1 = num;
                    if let (Some(y1), Some(x), Some(y)) = (
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                    ) {
                        current_x = x;
                        current_y = y;
                        commands.push(PathCommand::QuadraticCurveTo(x1, y1, x, y));
                    }
                }
                'q' => {
                    let dx1 = num;
                    if let (Some(dy1), Some(dx), Some(dy)) = (
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                    ) {
                        let x1 = current_x + dx1;
                        let y1 = current_y + dy1;
                        current_x += dx;
                        current_y += dy;
                        commands.push(PathCommand::QuadraticCurveTo(x1, y1, current_x, current_y));
                    }
                }
                'A' | 'a' => {
                    let rx = num;
                    if let (Some(ry), Some(rotation), Some(large_arc), Some(sweep), Some(x), Some(y)) = (
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                        parse_number(&mut chars),
                    ) {
                        let (end_x, end_y) = if current_cmd == 'A' {
                            (x, y)
                        } else {
                            (current_x + x, current_y + y)
                        };
                        current_x = end_x;
                        current_y = end_y;
                        commands.push(PathCommand::ArcTo(
                            rx, ry, rotation, large_arc != 0.0, sweep != 0.0, end_x, end_y
                        ));
                    }
                }
                'Z' | 'z' => {
                    commands.push(PathCommand::ClosePath);
                }
                _ => {}
            }
            continue;
        }

        // Handle command without immediate number
        match current_cmd {
            'Z' | 'z' => {
                commands.push(PathCommand::ClosePath);
            }
            _ => {}
        }
    }

    commands
}

// Simple XML helpers (not a full parser)

fn extract_tag(content: &str, tag_name: &str) -> Option<String> {
    let open = format!("<{}", tag_name);
    if let Some(start) = content.find(&open) {
        let rest = &content[start..];
        if let Some(end) = rest.find('>') {
            return Some(rest[..=end].to_string());
        }
    }
    None
}

fn extract_all_tags(content: &str, tag_name: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let open = format!("<{}", tag_name);
    let mut search_start = 0;

    while let Some(start) = content[search_start..].find(&open) {
        let abs_start = search_start + start;
        let rest = &content[abs_start..];
        
        // Check if this is a self-closing tag or has content
        if let Some(close_bracket) = rest.find('>') {
            if rest[..close_bracket].ends_with('/') {
                // Self-closing tag
                tags.push(rest[..=close_bracket].to_string());
                search_start = abs_start + close_bracket + 1;
            } else {
                // Find closing tag
                let close_tag = format!("</{}>", tag_name);
                if let Some(close_pos) = rest.find(&close_tag) {
                    tags.push(rest[..close_pos + close_tag.len()].to_string());
                    search_start = abs_start + close_pos + close_tag.len();
                } else {
                    // No closing tag, treat as self-closing
                    tags.push(rest[..=close_bracket].to_string());
                    search_start = abs_start + close_bracket + 1;
                }
            }
        } else {
            break;
        }
    }

    tags
}

fn extract_inner_content(tag: &str) -> String {
    if let Some(first_close) = tag.find('>') {
        if let Some(last_open) = tag.rfind("</") {
            return tag[first_close + 1..last_open].to_string();
        }
    }
    String::new()
}

fn extract_attr(tag: &str, attr_name: &str) -> Option<String> {
    // Try attr="value"
    let pattern1 = format!("{}=\"", attr_name);
    if let Some(start) = tag.find(&pattern1) {
        let rest = &tag[start + pattern1.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }

    // Try attr='value'
    let pattern2 = format!("{}='", attr_name);
    if let Some(start) = tag.find(&pattern2) {
        let rest = &tag[start + pattern2.len()..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    None
}

fn extract_attr_f32(tag: &str, attr_name: &str) -> Option<f32> {
    extract_attr(tag, attr_name).and_then(|v| {
        // Remove units like px, pt, etc.
        let cleaned = v.trim_end_matches(|c: char| c.is_alphabetic() || c == '%');
        cleaned.parse().ok()
    })
}

