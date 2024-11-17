// sidecar/src/main.rs
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use prometheus::{IntCounterVec, HistogramVec, Registry, Opts, Encoder};
use lazy_static::lazy_static;
use log::{info, error, warn};
use std::net::UdpSocket;
use std::thread;
use regex::Regex;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    
    // Counter for total requests by path, method, and status
    static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["path", "method", "status"]
    ).unwrap();
    
    // Histogram for request duration
    static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        ),
        &["path", "method"]
    ).unwrap();

    // Counter for business metrics - orders
    static ref ORDER_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("order_total", "Total number of orders"),
        &["product_id"]
    ).unwrap();

    // Regex for parsing metrics
    static ref REQUEST_METRIC_RE: Regex = Regex::new(
        r#"request\{path="([^"]+)",method="([^"]+)",status=(\d+)\}\s+([\d\.]+)"#
    ).unwrap();

    static ref ORDER_METRIC_RE: Regex = Regex::new(
        r#"order\{product_id="([^"]+)",quantity=(\d+)\}\s+(\d+)"#
    ).unwrap();
}

fn init_metrics() {
    // Register all metrics with Prometheus
    REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_REQUEST_DURATION_SECONDS.clone())).unwrap();
    REGISTRY.register(Box::new(ORDER_TOTAL.clone())).unwrap();
}

#[get("/metrics")]
async fn serve_metrics() -> impl Responder {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    
    encoder.encode(&metric_families, &mut buffer)
        .unwrap_or_else(|e| error!("Failed to encode metrics: {}", e));

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(buffer)
}

async fn metrics_collector() {
    let socket = UdpSocket::bind("0.0.0.0:9092").expect("Failed to bind UDP socket");
    let mut buf = [0; 4096];

    info!("Metrics collector started and listening for UDP metrics on port 9092");

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, addr)) => {
                info!("Received UDP packet from {}", addr);
                match String::from_utf8(buf[..size].to_vec()) {
                    Ok(metric_str) => {
                        info!("Processing metric: {}", metric_str);
                        process_metric(&metric_str);
                    },
                    Err(e) => error!("Failed to parse UDP data as UTF-8: {}", e)
                }
            },
            Err(e) => error!("Failed to receive UDP packet: {}", e)
        }
    }
}

fn process_metric(metric_str: &str) {
    info!("Processing metric string: {}", metric_str);

    // First try to match as a request metric
    if let Some(caps) = REQUEST_METRIC_RE.captures(metric_str) {
        info!("Matched request metric");
        let path = caps.get(1).map_or("", |m| m.as_str());
        let method = caps.get(2).map_or("", |m| m.as_str());
        let status = caps.get(3).map_or("", |m| m.as_str());
        let duration: f64 = caps.get(4)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0.0);

        // Increment request counter
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[path, method, status])
            .inc();

        // Observe request duration
        HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&[path, method])
            .observe(duration);

        info!("Updated metrics for path: {}, method: {}, status: {}", path, method, status);
        return;
    }

    // Then try to match as an order metric
    if let Some(caps) = ORDER_METRIC_RE.captures(metric_str) {
        let product_id = caps.get(1).map_or("", |m| m.as_str());
        let quantity: i64 = caps.get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);

        // Convert i64 to u64 safely
        if quantity >= 0 {
            ORDER_TOTAL
                .with_label_values(&[product_id])
                .inc_by(quantity.try_into().unwrap());
        } else {
            warn!("Received negative quantity in order metric: {}", quantity);
        }

        return;
    }

    warn!("Received unrecognized metric format: {}", metric_str);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    init_metrics();
    
    // Start metrics collector in a separate thread
    thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(metrics_collector());
    });

    info!("Starting HTTP metrics server on port 9091");

    HttpServer::new(move || {
        App::new().service(serve_metrics)
    })
    .bind("0.0.0.0:9091")?
    .run()
    .await
}