mod middleware;

use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use log::{info};
use serde::{Deserialize};
use crate::middleware::PrometheusMetricsMiddleware;

#[derive(Deserialize)]
struct Order {
    product_id: String,
    quantity: i32,
}

#[post("/order")]
async fn create_order(order: web::Json<Order>) -> impl Responder {
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

    HttpServer::new(move || {
        App::new()
            .wrap(PrometheusMetricsMiddleware::new())
            .service(create_order)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}