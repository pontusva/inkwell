# Inkwell

A high-performance PDF rendering engine written in Rust. Inkwell accepts a JSON-based layout description and generates pixel-perfect (well, maybe not yet!) PDF documents with support for flexbox-like layouts, text wrapping, images, SVG graphics, and tables.

## Features

- **Flexbox-inspired Layout Engine** — Row/column layouts with alignment, gap, flex-grow, and wrapping support
- **Rich Text Rendering** — Multi-line text with automatic wrapping, text alignment (left, center, right, justify), and font variants (bold, italic)
- **Image Support** — Embed images from URLs, base64 data URIs, or local files with `object-fit` options (cover, contain, fill, none, scale-down)
- **SVG Rendering** — Parse and render SVG graphics including paths, shapes, and basic transforms
- **Tables** — Full table support with column widths, row/column spans, and cell styling
- **Styling** — CSS-like styling with padding, margin, borders (including per-side and rounded corners), backgrounds, and opacity
- **Positioning** — Static, relative, and absolute positioning
- **Pagination** — Automatic multi-page document generation
- **HTTP API** — Simple REST endpoint for PDF generation

## Quick Start

### Prerequisites

- Rust 1.70+ (2021 edition)

### Installation

```bash
git clone https://github.com/your-username/inkwell.git
cd inkwell
cargo build --release
```

### Running the Server

```bash
cargo run --release
```

The server starts on `http://127.0.0.1:3001`.

### Generate a PDF

Send a POST request to `/render-pdf` with a JSON layout:

```bash
curl -X POST http://localhost:3001/render-pdf \
  -H "Content-Type: application/json" \
  -d '{
    "root": {
      "type": "page",
      "style": {
        "width": 595,
        "height": 842,
        "padding": 40,
        "backgroundColor": { "r": 255, "g": 255, "b": 255, "a": 1 }
      },
      "children": [
        {
          "type": "text",
          "text": "Hello, Inkwell!",
          "style": {
            "fontSize": 24,
            "fontWeight": "bold",
            "color": { "r": 0, "g": 0, "b": 0, "a": 1 }
          }
        }
      ]
    }
  }' --output hello.pdf
```

## Layout JSON Schema

### Node Types

| Type    | Description                              |
| ------- | ---------------------------------------- |
| `page`  | Root container representing a PDF page   |
| `view`  | Generic container for grouping elements  |
| `text`  | Text content with wrapping and alignment |
| `image` | Raster images (PNG, JPEG, etc.)          |
| `svg`   | Vector graphics                          |
| `table` | Table container                          |
| `row`   | Table row                                |
| `cell`  | Table cell                               |

### Basic Structure

```json
{
  "root": {
    "type": "page",
    "style": { ... },
    "children": [ ... ]
  }
}
```

### Style Properties

#### Dimensions

```json
{
  "width": 200,
  "height": 100,
  "minWidth": 50,
  "maxWidth": 500,
  "minHeight": 50,
  "maxHeight": 500
}
```

Dimensions can be fixed points or percentages:

```json
{
  "width": "50%",
  "height": 100
}
```

#### Layout (Flexbox-like)

```json
{
  "direction": "column", // "row" | "column"
  "mainAlign": "center", // "start" | "center" | "end" | "space-between" | "space-around" | "space-evenly"
  "crossAlign": "stretch", // "start" | "center" | "end" | "stretch"
  "gap": 10,
  "wrap": true,
  "flex": 1
}
```

#### Spacing

```json
{
  "padding": 20,
  "paddingTop": 10,
  "paddingRight": 15,
  "paddingBottom": 10,
  "paddingLeft": 15,
  "margin": 10,
  "marginTop": 5
}
```

#### Background & Opacity

```json
{
  "backgroundColor": { "r": 240, "g": 240, "b": 240, "a": 1 },
  "opacity": 0.9
}
```

#### Borders

```json
{
  "borderWidth": 1,
  "borderColor": { "r": 0, "g": 0, "b": 0, "a": 1 },
  "borderRadius": 8,
  "borderTopLeftRadius": 4,
  "borderTopRightRadius": 4
}
```

Per-side borders:

```json
{
  "borderTopWidth": 2,
  "borderTopColor": { "r": 255, "g": 0, "b": 0, "a": 1 },
  "borderBottomWidth": 1
}
```

#### Text Styling

```json
{
  "fontSize": 14,
  "fontWeight": "bold", // "normal" | "bold"
  "fontStyle": "italic", // "normal" | "italic"
  "textAlign": "justify", // "left" | "center" | "right" | "justify"
  "lineHeight": 1.5,
  "color": { "r": 51, "g": 51, "b": 51, "a": 1 }
}
```

#### Positioning

```json
{
  "position": "absolute", // "static" | "relative" | "absolute"
  "top": 10,
  "right": 10,
  "bottom": 10,
  "left": 10
}
```

#### Images

```json
{
  "objectFit": "cover" // "cover" | "contain" | "fill" | "none" | "scale-down"
}
```

### Complete Example

```json
{
  "root": {
    "type": "page",
    "style": {
      "width": 595,
      "height": 842,
      "padding": 40,
      "backgroundColor": { "r": 255, "g": 255, "b": 255, "a": 1 }
    },
    "children": [
      {
        "type": "view",
        "style": {
          "direction": "row",
          "gap": 20,
          "marginBottom": 30
        },
        "children": [
          {
            "type": "image",
            "src": "https://example.com/logo.png",
            "style": {
              "width": 80,
              "height": 80,
              "objectFit": "contain"
            }
          },
          {
            "type": "view",
            "style": { "flex": 1 },
            "children": [
              {
                "type": "text",
                "text": "Company Name",
                "style": {
                  "fontSize": 24,
                  "fontWeight": "bold"
                }
              },
              {
                "type": "text",
                "text": "Tagline goes here",
                "style": {
                  "fontSize": 12,
                  "color": { "r": 128, "g": 128, "b": 128, "a": 1 }
                }
              }
            ]
          }
        ]
      },
      {
        "type": "table",
        "columnWidths": ["30%", "70%"],
        "style": {
          "width": "100%",
          "borderWidth": 1,
          "borderColor": { "r": 200, "g": 200, "b": 200, "a": 1 }
        },
        "children": [
          {
            "type": "row",
            "children": [
              {
                "type": "cell",
                "style": {
                  "padding": 8,
                  "backgroundColor": { "r": 240, "g": 240, "b": 240, "a": 1 }
                },
                "children": [
                  {
                    "type": "text",
                    "text": "Item",
                    "style": { "fontWeight": "bold" }
                  }
                ]
              },
              {
                "type": "cell",
                "style": {
                  "padding": 8,
                  "backgroundColor": { "r": 240, "g": 240, "b": 240, "a": 1 }
                },
                "children": [
                  {
                    "type": "text",
                    "text": "Description",
                    "style": { "fontWeight": "bold" }
                  }
                ]
              }
            ]
          },
          {
            "type": "row",
            "children": [
              {
                "type": "cell",
                "style": { "padding": 8 },
                "children": [{ "type": "text", "text": "Widget A" }]
              },
              {
                "type": "cell",
                "style": { "padding": 8 },
                "children": [
                  {
                    "type": "text",
                    "text": "A fantastic widget that does amazing things."
                  }
                ]
              }
            ]
          }
        ]
      }
    ]
  }
}
```

### SVG Support

Embed SVG graphics via URL, data URI, or inline content:

```json
{
  "type": "svg",
  "src": "https://example.com/icon.svg",
  "style": { "width": 24, "height": 24 }
}
```

Or inline:

```json
{
  "type": "svg",
  "content": "<svg viewBox='0 0 24 24'><circle cx='12' cy='12' r='10' fill='red'/></svg>",
  "style": { "width": 24, "height": 24 }
}
```

Supported SVG elements:

- `<path>` with full path command support (M, L, H, V, C, S, Q, T, A, Z)
- `<rect>` with optional rounded corners
- `<circle>`, `<ellipse>`
- `<line>`, `<polyline>`, `<polygon>`
- `<g>` groups with transforms

## Architecture

```
src/
├── main.rs          # HTTP server (Axum) and API endpoint
├── layout.rs        # JSON schema types and style definitions
├── layout_box.rs    # Layout tree construction and measurement
├── pdf.rs           # PDF generation and rendering
├── svg.rs           # SVG parsing and rendering
└── font_metrics.rs  # Helvetica font metrics for text measurement
```

### Layout Pipeline

1. **Parse** — JSON payload is deserialized into a tree of `JsonNode`
2. **Build** — Nodes are converted to `LayoutBox` tree
3. **Measure** — Intrinsic sizes are calculated (text wrapping, image dimensions)
4. **Place** — Final positions are computed using flexbox-like algorithm
5. **Paginate** — Content is split across pages if needed
6. **Render** — PDF primitives are drawn using `printpdf`

## Dependencies

| Crate                  | Purpose                          |
| ---------------------- | -------------------------------- |
| `axum`                 | HTTP server framework            |
| `tokio`                | Async runtime                    |
| `serde` / `serde_json` | JSON serialization               |
| `printpdf`             | PDF generation                   |
| `image`                | Image decoding                   |
| `base64`               | Base64 decoding for data URIs    |
| `ureq`                 | HTTP client for remote resources |
| `tower-http`           | CORS middleware                  |

## License

MIT
