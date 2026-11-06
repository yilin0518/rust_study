use axum::{
    Json, Router,
    extract::{self, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

async fn root() -> &'static str {
    "Hello, World!"
}

async fn healthz() -> &'static str {
    "ok"
}

#[derive(Serialize, Deserialize, Clone)]
struct Product {
    id: u32,
    name: String,
    price_cents: u32,
    stock: u32,
}

async fn get_product(
    extract::Path(id): extract::Path<u32>,
    State(products): State<Arc<Vec<Product>>>,
) -> Result<Json<Product>, StatusCode> {
    let product = products
        .iter()
        .find(|p| p.id == id)
        .ok_or(StatusCode::NOT_FOUND);
    match product {
        Ok(product) => Ok(Json(product.clone())),
        Err(status) => Err(status),
    }
}

async fn get_products(State(products): State<Arc<Vec<Product>>>) -> Json<Vec<Product>> {
    Json(products.iter().cloned().collect())
}

#[tokio::main]
async fn main() {
    let products = Arc::new(vec![
        Product {
            id: 1,
            name: "Example Product 1".to_string(),
            price_cents: 1000,
            stock: 10,
        },
        Product {
            id: 2,
            name: "Example Product 2".to_string(),
            price_cents: 2000,
            stock: 20,
        },
    ]);
    let app = Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .route("/products", get(get_products))
        .route("/products/:id", get(get_product))
        .with_state(products);
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
