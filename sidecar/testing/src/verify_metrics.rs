use influxdb2::Client as InfluxClient;
use influxdb2::models::Query;
use log::info;
use std::collections::HashMap;

async fn get_total_requests(client: &InfluxClient, bucket: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let query = Query::new(format!(
        r#"from(bucket:"{}")
          |> range(start: -1h)
          |> filter(fn: (r) => r._measurement == "total_requests")
          |> last()
          |> limit(n: 1)"#,
        bucket
    ));

    let result = client.query_raw(Some(query)).await?;
    
    // Take only the first record
    for row in result.into_iter().take(1) {
        if let Some(value) = row.values.get("_value") {
            return Ok(value.to_string().parse::<i64>().unwrap_or(0));
        }
    }
    
    Ok(0)
}

async fn get_product_distribution(client: &InfluxClient, bucket: &str) -> Result<HashMap<String, i64>, Box<dyn std::error::Error>> {
    let query = Query::new(format!(
        r#"from(bucket:"{}")
          |> range(start: -1h)
          |> filter(fn: (r) => r._measurement == "order_processed")
          |> group(columns: ["product_id"])
          |> count()
          |> limit(n: 1000)"#,
        bucket
    ));

    let result = client.query_raw(Some(query)).await?;
    let mut distribution: HashMap<String, i64> = HashMap::new();

    for row in result.into_iter() {
        if let (Some(product_id), Some(count)) = (
            row.values.get("product_id").map(|v| v.to_string()),
            row.values.get("_value").map(|v| v.to_string().parse::<i64>().unwrap_or(0))
        ) {
            distribution.insert(product_id, count);
        }
    }

    Ok(distribution)
}

async fn verify_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let url = "http://localhost:8086";
    let token = "secret_token";
    let org = "myorg";
    let bucket = "metrics";

    let client = InfluxClient::new(url, org, token);
    
    info!("\n=== Metrics Verification Report ===\n");

    // Get and display total requests
    match get_total_requests(&client, bucket).await {
        Ok(total) => {
            info!("Request Analysis:");
            info!("Total Requests: {}", total);
        },
        Err(e) => info!("Error getting total requests: {}", e),
    }

    // Get and display product distribution
    match get_product_distribution(&client, bucket).await {
        Ok(distribution) => {
            info!("\nProduct Distribution:");
            for (product_id, count) in distribution {
                info!("{}: {} orders", product_id, count);
            }
        },
        Err(e) => info!("Error getting product distribution: {}", e),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    verify_metrics().await
}