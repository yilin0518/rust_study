use axum::{
    Json, Router,
    extract::{self, Path, State},
    http::StatusCode,
    routing::{get, post, put, delete},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

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

#[derive(Deserialize)]
struct ProductRequest {
    name: String,
    price_cents: u32,
    stock: u32,
}

impl ProductRequest {
    fn check_product_request(&self) -> Result<(), StatusCode> {
        if self.name.trim().is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }
        if self.name.trim().chars().count() > 100 {
            return Err(StatusCode::BAD_REQUEST);
        }
        if self.price_cents > 1_000_000 {
            return Err(StatusCode::BAD_REQUEST);
        }
        if self.stock > 1_000 {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok(())
    }
}

struct AppState {
    products: Vec<Product>,
    next_id: u32,
}

impl AppState {
    fn new() -> Self {
        AppState {
            products: Vec::new(),
            next_id: 1,
        }
    }
}

async fn create_product(
    State(app_state): State<Arc<Mutex<AppState>>>,
    extract::Json(product_req): extract::Json<ProductRequest>,
) -> Result<(StatusCode, Json<Product>), StatusCode> {
    product_req.check_product_request()?;
    let mut app_state = app_state.lock().await;
    let product = Product {
        id: app_state.next_id,
        name: product_req.name.trim().to_string(),
        price_cents: product_req.price_cents,
        stock: product_req.stock,
    };
    app_state
        .next_id
        .checked_add(1)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    app_state.next_id += 1;
    app_state.products.push(product.clone());
    Ok((StatusCode::CREATED, Json(product)))
}

async fn update_product(
    State(app_state): State<Arc<Mutex<AppState>>>,
    Path(id): Path<u32>,
    extract::Json(product_up): extract::Json<ProductRequest>,
) -> Result<Json<Product>, StatusCode> {
    product_up.check_product_request()?;
    let mut app_state = app_state.lock().await;
    let product = app_state
        .products
        .iter_mut()
        .find(|product| product.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    product.name = product_up.name.trim().to_string();
    product.price_cents = product_up.price_cents;
    product.stock = product_up.stock;
    Ok(Json(product.clone()))
}

async fn delete_product(
    State(app_state): State<Arc<Mutex<AppState>>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, StatusCode> {
    let mut app_state = app_state.lock().await;
    app_state.products.iter().find(|product| {
        product.id == id
    }).ok_or(StatusCode::NOT_FOUND)?;
    app_state.products.retain(|product| {
        product.id != id
    });
    Ok(StatusCode::NO_CONTENT)
}

async fn get_product(
    extract::Path(id): extract::Path<u32>,
    State(app_state): State<Arc<Mutex<AppState>>>,
) -> Result<Json<Product>, StatusCode> {
    let app_state = app_state.lock().await;
    let product = app_state
        .products
        .iter()
        .find(|p| p.id == id)
        .ok_or(StatusCode::NOT_FOUND);
    match product {
        Ok(product) => Ok(Json(product.clone())),
        Err(status) => Err(status),
    }
}

async fn get_products(State(app_state): State<Arc<Mutex<AppState>>>) -> Json<Vec<Product>> {
    let app_state = app_state.lock().await;
    Json(app_state.products.to_vec())
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(Mutex::new(AppState::new()));
    let app = Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .route("/products", get(get_products))
        .route("/products", post(create_product))
        .route("/products/:id", get(get_product))
        .route("/products/:id", put(update_product))
        .route("/products/:id", delete(delete_product))
        .with_state(app_state);
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
