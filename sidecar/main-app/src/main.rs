use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// Counter for total requests
static REQUEST_COUNT: AtomicUsize = AtomicUsize::new(0);
// Store start time
static START_TIME: once_cell::sync::Lazy<SystemTime> = once_cell::sync::Lazy::new(SystemTime::now);

#[derive(Serialize)]
struct Metrics {
    total_requests: usize,
    uptime_seconds: u64,
}

#[derive(Deserialize)]
struct Order {
    product_id: String,
    quantity: i32,
}

#[get("/metrics")]
async fn metrics() -> impl Responder {
    let uptime = START_TIME
        .elapsed()
        .unwrap_or_default()
        .as_secs();
    
    let metrics = Metrics {
        total_requests: REQUEST_COUNT.load(Ordering::Relaxed),
        uptime_seconds: uptime,
    };
    HttpResponse::Ok().json(metrics)
}

#[post("/order")]
async fn create_order(order: web::Json<Order>) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    info!("Received order for product: {}", order.product_id);
    
    HttpResponse::Ok().json(format!(
        "Order processed for {} units of product {}",
        order.quantity, order.product_id
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting main application server");

    HttpServer::new(|| {
        App::new()
            .service(metrics)
            .service(create_order)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}