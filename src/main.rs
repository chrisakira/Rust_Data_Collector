mod api;
mod influx;

use api::fetch_usd_brl;
use influx::{InfluxConfig, insert_rate};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- InfluxDB 3 Configuration ---
    // Note: No `org` field needed in v3!
    let config = InfluxConfig {
        host:     std::env::var("INFLUX_HOST")
                    .unwrap_or_else(|_| "http://localhost:8181".to_string()),
        token:    std::env::var("INFLUX_TOKEN")
                    .expect("INFLUX_TOKEN env var must be set"),
        database: std::env::var("INFLUX_DATABASE")
                    .unwrap_or_else(|_| "exchange-rates".to_string()),
    };

    // --- Fetch Exchange Rate ---
    println!("📡 Fetching USD → BRL exchange rate...");
    let rate = fetch_usd_brl().await?;

    println!("💵 {} → {} : R$ {} (ask: R$ {})", rate.code, rate.codein, rate.bid, rate.ask);
    println!("📈 High: R$ {} | 📉 Low: R$ {}", rate.high, rate.low);
    println!("📊 Change: {} ({}%)", rate.varBid, rate.pctChange);
    println!("🕒 Updated at: {}", rate.create_date);

    // --- Insert into InfluxDB 3 ---
    println!("\n📦 Inserting data into InfluxDB 3...");
    insert_rate(&config, &rate).await?;

    Ok(())
}