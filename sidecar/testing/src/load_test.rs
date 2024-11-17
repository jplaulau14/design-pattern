use chrono::Local;
use futures::stream::{self, StreamExt};
use rand::seq::SliceRandom;
use serde_json::json;
use std::time::Instant;
use log::{info, error};

#[derive(Debug)]
struct TestConfig {
    total_orders: usize,
    concurrent_orders: usize,
}

async fn send_order(client: &reqwest::Client, order_id: usize) -> Result<(), Box<dyn std::error::Error>> {
    let products = ["LAPTOP", "PHONE", "TABLET", "WATCH", "HEADPHONES"];
    let product = products.choose(&mut rand::thread_rng()).unwrap();
    let quantity = rand::random::<u32>() % 5 + 1;

    let payload = json!({
        "product_id": format!("{}_{}", product, order_id),
        "quantity": quantity
    });

    let response = client
        .post("http://localhost:8080/order")
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        info!("[{}] Order {} processed successfully", Local::now(), order_id);
    } else {
        error!("[{}] Order {} failed with status {}", Local::now(), order_id, response.status());
    }

    Ok(())
}

async fn check_metrics(client: &reqwest::Client) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .get("http://localhost:9091/metrics")
        .send()
        .await?
        .text()
        .await?;

    for line in response.lines() {
        if line.starts_with("total_requests") {
            if let Some(value) = line.split_whitespace().last() {
                info!(
                    "\n[{}] Current Metrics:\nTotal Requests: {}\n",
                    Local::now(),
                    value
                );
            }
        }
    }

    Ok(())
}

async fn run_load_test(config: TestConfig) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let start_time = Instant::now();

    info!(
        "Starting load test with {} total orders, {} concurrent orders",
        config.total_orders, config.concurrent_orders
    );

    // Create chunks of orders to process concurrently
    let chunks = (0..config.total_orders).collect::<Vec<_>>()
        .chunks(config.concurrent_orders)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();

    for chunk in chunks {
        // Process orders in parallel
        let futures = stream::iter(chunk)
            .map(|order_id| send_order(&client, order_id))
            .buffered(config.concurrent_orders);
        
        futures::pin_mut!(futures);
        
        while let Some(result) = futures.next().await {
            if let Err(e) = result {
                error!("Error processing order: {}", e);
            }
        }

        // Check metrics after each batch
        if let Err(e) = check_metrics(&client).await {
            error!("Error checking metrics: {}", e);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    let duration = start_time.elapsed();
    info!("\nLoad test completed in {:.2} seconds", duration.as_secs_f64());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = TestConfig {
        total_orders: 100,
        concurrent_orders: 10,
    };

    run_load_test(config).await
}