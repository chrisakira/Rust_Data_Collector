mod api;
mod influx;
mod health;

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

const POLL_INTERVAL_SECS: u64 = 10;
const HEALTH_PORT: u16 = 3000;

// ─── Healthcheck Handler ────────────────────────────────────────────────────

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

    let response = Response::builder()
        .status(status_code)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap();

    Ok(response)
}

// ─── Polling Loop ───────────────────────────────────────────────────────────

async fn polling_loop(config: InfluxConfig, state: AppState) {
    let mut interval = time::interval(Duration::from_secs(POLL_INTERVAL_SECS));

    loop {
        interval.tick().await;
        println!("📡 Fetching USD → BRL exchange rate...");

        // ── Check API ──
        let (api_ok, api_msg, rate_opt) = match fetch_usd_brl().await {
            Ok(rate) => {
                let msg = format!("R$ {} (ask: R$ {})", rate.bid, rate.ask);
                println!("💵 USD → BRL : {}", msg);
                (true, msg, Some(rate))
            }
            Err(e) => {
                eprintln!("❌ API error: {e}");
                (false, e.to_string(), None)
            }
        };

        // ── Check InfluxDB ──
        let (influx_ok, influx_msg) = match &rate_opt {
            Some(rate) => match insert_rate(&config, rate).await {
                Ok(_) => (true, "Write successful".to_string()),
                Err(e) => {
                    eprintln!("❌ InfluxDB error: {e}");
                    (false, e.to_string())
                }
            },
            None => (false, "Skipped — no data from API".to_string()),
        };

        // ── Update shared health state ──
        state.update(api_ok, api_msg, influx_ok, influx_msg).await;

        println!("⏳ Next fetch in {POLL_INTERVAL_SECS}s...\n");
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InfluxConfig {
        host: std::env::var("INFLUX_HOST")
            .unwrap_or_else(|_| "http://localhost:8181".to_string()),
        token: std::env::var("INFLUX_TOKEN")
            .expect("INFLUX_TOKEN env var must be set"),
        database: std::env::var("INFLUX_DATABASE")
            .unwrap_or_else(|_| "exchange-rates".to_string()),
    };

    let state = AppState::new();

    // ── Start healthcheck HTTP server ──
    let health_state = state.clone();
    let addr = SocketAddr::from(([0, 0, 0, 0], HEALTH_PORT));
    let listener = TcpListener::bind(addr).await?;
    println!("🩺 Healthcheck listening on http://0.0.0.0:{HEALTH_PORT}/healthcheck");

    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let io = TokioIo::new(stream);
            let state = health_state.clone();

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

    // ── Start polling loop ──
    polling_loop(config, state).await;

    Ok(())
}