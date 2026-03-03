mod api;
mod influx;
mod health;
mod weather;
mod stock;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::time;

use api::fetch_usd_brl;
use health::AppState;
use influx::{InfluxConfig, insert_rate};
use weather::{fetch_weather, insert_weather};
use stock::{fetch_stocks, insert_stocks};

const EXCHANGE_POLL_SECS: u64     = 60;
const WEATHER_POLL_SECS:  u64     = 60 * 60 * 24; // 24 hours
const STOCK_POLL_SECS:    u64     = 60 * 60;      // 1 hour
const HEALTH_PORT:        u16     = 3000;

// ─── Healthcheck Handler ─────────────────────────────────────────────────────

async fn healthcheck_handler(
    state: AppState,
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let health = state.health.read().await;
    let body = serde_json::to_string(&*health).unwrap_or_else(|_| "{}".to_string());

    let status_code = match health.status {
        health::Status::Ok       => StatusCode::OK,
        health::Status::Degraded => StatusCode::SERVICE_UNAVAILABLE,
        health::Status::Down     => StatusCode::SERVICE_UNAVAILABLE,
    };

    Ok(Response::builder()
        .status(status_code)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

// ─── Exchange Rate Polling Loop (every 10s) ───────────────────────────────────

async fn exchange_polling_loop(config: InfluxConfig, state: AppState) {
    let mut interval = time::interval(Duration::from_secs(EXCHANGE_POLL_SECS));

    loop {
        println!("📡 [Exchange] Fetching USD → BRL...");

        let (api_ok, api_msg, rate_opt) = match fetch_usd_brl().await {
            Ok(rate) => {
                let msg = format!("R$ {} (ask: R$ {})", rate.bid, rate.ask);
                println!("💵 USD → BRL : {}", msg);
                (true, msg, Some(rate))
            }
            Err(e) => {
                eprintln!("❌ [Exchange] API error: {e}");
                (false, e.to_string(), None)
            }
        };

        let (influx_ok, influx_msg) = match &rate_opt {
            Some(rate) => match insert_rate(&config, rate).await {
                Ok(_)  => (true,  "Write successful".to_string()),
                Err(e) => {
                    eprintln!("❌ [Exchange] InfluxDB error: {e}");
                    (false, e.to_string())
                }
            },
            None => (false, "Skipped — no data from API".to_string()),
        };

        state.update_exchange(api_ok, api_msg, influx_ok, influx_msg).await;
        println!("⏳ [Exchange] Next fetch in {EXCHANGE_POLL_SECS}s\n");
        interval.tick().await;
    }
}

// ─── Weather Polling Loop (once per day) ─────────────────────────────────────

async fn weather_polling_loop(config: InfluxConfig, state: AppState) {
    let mut interval = time::interval(Duration::from_secs(WEATHER_POLL_SECS));

    loop {
        println!("🌤️  [Weather] Fetching 7-day forecast...");

        let (api_ok, api_msg, weather_opt) = match fetch_weather().await {
            Ok(w) => {
                let msg = format!(
                    "{}°C feels like {}°C, cloud cover {}%, rain {}%, precipitation {}%.",
                    w.current.temperature_2m,
                    w.current.apparent_temperature,
                    w.current.cloud_cover,
                    w.current.rain,
                    w.current.precipitation,
                );
                println!("🌡️  Current: {}", msg);
                (true, msg, Some(w))
            }
            Err(e) => {
                eprintln!("❌ [Weather] API error: {e}");
                (false, e.to_string(), None)
            }
        };

        let (influx_ok, influx_msg) = match &weather_opt {
            Some(w) => match insert_weather(&config, w).await {
                Ok(_)  => (true,  "Write successful".to_string()),
                Err(e) => {
                    eprintln!("❌ [Weather] InfluxDB error: {e}");
                    (false, e.to_string())
                }
            },
            None => (false, "Skipped — no data from API".to_string()),
        };

        state.update_weather(api_ok, api_msg, influx_ok, influx_msg).await;
        println!("⏳ [Weather] Next fetch in 24h\n");
        interval.tick().await;
    }
}

// ─── Stock Polling Loop (every hour) ─────────────────────────────────────────

async fn stock_polling_loop(config: InfluxConfig, state: AppState, brapi_token: Option<String>) {
    let mut interval = time::interval(Duration::from_secs(STOCK_POLL_SECS));

    loop {
        println!("📊 [Stocks] Fetching Brazilian stock data...");

        let (api_ok, api_msg, stocks_opt) = match fetch_stocks(brapi_token.as_ref()).await {
            Ok(stocks) => {
                let msg = format!("Fetched {} stocks successfully", stocks.len());
                (true, msg, Some(stocks))
            }
            Err(e) => {
                eprintln!("❌ [Stocks] API error: {e}");
                (false, e.to_string(), None)
            }
        };

        let (influx_ok, influx_msg) = match &stocks_opt {
            Some(stocks) => match insert_stocks(&config, stocks).await {
                Ok(_)  => (true,  "Write successful".to_string()),
                Err(e) => {
                    eprintln!("❌ [Stocks] InfluxDB error: {e}");
                    (false, e.to_string())
                }
            },
            None => (false, "Skipped — no data from API".to_string()),
        };

        state.update_stocks(api_ok, api_msg, influx_ok, influx_msg).await;
        println!("⏳ [Stocks] Next fetch in 1h\n");
        interval.tick().await;
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InfluxConfig {
        host:     std::env::var("INFLUX_HOST")
                    .unwrap_or_else(|_| "http://localhost:8181".to_string()),
        token:    std::env::var("INFLUX_TOKEN")
                    .expect("INFLUX_TOKEN env var must be set"),
        database: std::env::var("INFLUX_DATABASE")
                    .unwrap_or_else(|_| "exchange-rates".to_string()),
    };

    // Optional BRAPI token (some endpoints may not require it)
    let brapi_token = std::env::var("BRAPI_TOKEN").ok();
    
    if brapi_token.is_some() {
        println!("🔑 BRAPI token configured");
    } else {
        println!("ℹ️  BRAPI token not set (using free tier)");
    }

    let state = AppState::new();

    // ── Healthcheck HTTP server ──
    let health_state = state.clone();
    let addr         = SocketAddr::from(([0, 0, 0, 0], HEALTH_PORT));
    let listener     = TcpListener::bind(addr).await?;
    println!("🩺 Healthcheck listening on http://0.0.0.0:{HEALTH_PORT}/healthcheck");

    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let io          = TokioIo::new(stream);
            let state       = health_state.clone();

            tokio::spawn(async move {
                let svc = service_fn(move |req| {
                    let state = state.clone();
                    healthcheck_handler(state, req)
                });
                if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                    eprintln!("❌ HTTP server error: {e}");
                }
            });
        }
    });

    // ── Exchange rate loop (every 10s) ──
    let exchange_config = config.clone();
    let exchange_state  = state.clone();
    tokio::spawn(async move {
        exchange_polling_loop(exchange_config, exchange_state).await;
    });

    // ── Stock loop (every hour) ──
    let stock_config = config.clone();
    let stock_state  = state.clone();
    let stock_token  = brapi_token.clone();
    tokio::spawn(async move {
        stock_polling_loop(stock_config, stock_state, stock_token).await;
    });

    // ── Weather loop (once per day) ──
    weather_polling_loop(config, state).await;

    Ok(())
}