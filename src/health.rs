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
/// Add new fields here to extend the healthcheck response.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub status: Status,
    pub message: String,
}

/// The full healthcheck response.
/// Add new components here to extend in the future.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: Status,           // overall status
    pub api: ComponentHealth,     // AwesomeAPI connectivity
    pub influxdb: ComponentHealth, // InfluxDB connectivity
}

/// Shared application state — updated by the polling loop
/// and read by the healthcheck endpoint.
#[derive(Debug, Clone)]
pub struct AppState {
    pub health: Arc<RwLock<HealthResponse>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            health: Arc::new(RwLock::new(HealthResponse {
                status: Status::Degraded,
                api: ComponentHealth {
                    status: Status::Degraded,
                    message: "Not yet checked".to_string(),
                },
                influxdb: ComponentHealth {
                    status: Status::Degraded,
                    message: "Not yet checked".to_string(),
                },
            })),
        }
    }

    /// Updates the health state after each polling cycle.
    pub async fn update(
        &self,
        api_ok: bool,
        api_msg: String,
        influx_ok: bool,
        influx_msg: String,
    ) {
        let overall = match (api_ok, influx_ok) {
            (true, true)  => Status::Ok,
            (false, false) => Status::Down,
            _              => Status::Degraded,
        };

        let mut health = self.health.write().await;
        health.status = overall;
        health.api = ComponentHealth {
            status: if api_ok { Status::Ok } else { Status::Down },
            message: api_msg,
        };
        health.influxdb = ComponentHealth {
            status: if influx_ok { Status::Ok } else { Status::Down },
            message: influx_msg,
        };
    }
}