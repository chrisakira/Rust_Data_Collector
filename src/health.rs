use std::sync::Arc;
use serde::Serialize;
use tokio::sync::RwLock;

/// Represents the status of a single component.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Degraded,
    Down,
}

/// A single component health entry.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub status: Status,
    pub message: String,
}

/// The full healthcheck response.
/// Add new components here to extend in the future.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: Status,
    pub api: ComponentHealth,           // AwesomeAPI (USD/BRL)
    pub influxdb: ComponentHealth,      // InfluxDB write (USD/BRL)
    pub weather_api: ComponentHealth,   // Open-Meteo API
    pub weather_influxdb: ComponentHealth, // InfluxDB write (weather)
}

/// Shared application state updated by polling loops
/// and read by the healthcheck endpoint.
#[derive(Debug, Clone)]
pub struct AppState {
    pub health: Arc<RwLock<HealthResponse>>,
}

impl AppState {
    pub fn new() -> Self {
        let degraded = || ComponentHealth {
            status: Status::Degraded,
            message: "Not yet checked".to_string(),
        };

        Self {
            health: Arc::new(RwLock::new(HealthResponse {
                status:           Status::Degraded,
                api:              degraded(),
                influxdb:         degraded(),
                weather_api:      degraded(),
                weather_influxdb: degraded(),
            })),
        }
    }

    /// Updates USD/BRL exchange rate health.
    pub async fn update_exchange(
        &self,
        api_ok:     bool,
        api_msg:    String,
        influx_ok:  bool,
        influx_msg: String,
    ) {
        let mut health = self.health.write().await;
        health.api = ComponentHealth {
            status:  if api_ok    { Status::Ok } else { Status::Down },
            message: api_msg,
        };
        health.influxdb = ComponentHealth {
            status:  if influx_ok { Status::Ok } else { Status::Down },
            message: influx_msg,
        };
        health.status = compute_overall(&health);
    }

    /// Updates weather forecast health.
    pub async fn update_weather(
        &self,
        api_ok:     bool,
        api_msg:    String,
        influx_ok:  bool,
        influx_msg: String,
    ) {
        let mut health = self.health.write().await;
        health.weather_api = ComponentHealth {
            status:  if api_ok    { Status::Ok } else { Status::Down },
            message: api_msg,
        };
        health.weather_influxdb = ComponentHealth {
            status:  if influx_ok { Status::Ok } else { Status::Down },
            message: influx_msg,
        };
        health.status = compute_overall(&health);
    }
}

/// Computes the overall status from all components.
/// Add new components here when extending the healthcheck.
fn compute_overall(h: &HealthResponse) -> Status {
    let all = [
        matches!(h.api.status,              Status::Ok),
        matches!(h.influxdb.status,         Status::Ok),
        matches!(h.weather_api.status,      Status::Ok),
        matches!(h.weather_influxdb.status, Status::Ok),
    ];

    if all.iter().all(|&ok| ok)  { return Status::Ok;       }
    if all.iter().all(|&ok| !ok) { return Status::Down;     }
    Status::Degraded
}