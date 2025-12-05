use axum::{routing::post, Json, Router};
use axum::response::IntoResponse;
use std::net::SocketAddr;

use tower_http::cors::{CorsLayer, Any};
use crate::layout::LayoutPayload;
use crate::pdf::from_layout;

mod pdf;
mod layout;
mod layout_box;
mod font_metrics;
mod svg;


#[tokio::main]
async fn main() {
    // Allow all origins for now (you can restrict later)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/render-pdf", post(render_pdf))
        .layer(cors);  // <-- attach CORS middleware

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    println!("PDF engine listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}



async fn render_pdf(Json(payload): Json<LayoutPayload>) -> impl IntoResponse {
    let pdf_bytes = from_layout(&payload.root);

    (
        [
            ("Content-Type", "application/pdf"),
            ("Content-Disposition", "attachment; filename=\"doc.pdf\""),
        ],
        pdf_bytes,
    )
}