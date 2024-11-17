use log::{error, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
struct Metrics {
    total_requests: usize,
    uptime_seconds: u64,
}

#[derive(Serialize)]
struct PrometheusMetrics {
    total_requests: usize,
    uptime_seconds: u64,
    timestamp: u64,
}

async fn fetch_and_process_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://main-app:8080/metrics")
        .send()
        .await?
        .json::<Metrics>()
        .await?;

    // Transform into Prometheus format
    let prometheus_metrics = PrometheusMetrics {
        total_requests: response.total_requests,
        uptime_seconds: response.uptime_seconds,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    info!(
        "# HELP total_requests Total number of requests processed\n\
         # TYPE total_requests counter\n\
         total_requests {}\n\
         # HELP uptime_seconds Total uptime in seconds\n\
         # TYPE uptime_seconds gauge\n\
         uptime_seconds {}",
        prometheus_metrics.total_requests, prometheus_metrics.uptime_seconds
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting metrics sidecar");

    let mut interval = tokio::time::interval(Duration::from_secs(15));

    loop {
        interval.tick().await;
        if let Err(e) = fetch_and_process_metrics().await {
            error!("Error processing metrics: {}", e);
        }
    }
}