use std::sync::{Arc, Mutex};
use sqlx::SqlitePool;
use crate::config::Config;

/// Pojedynczy punkt pomiarowy (używany w historii w pamięci i SSE)
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DataPoint {
    pub ts:       f64, // timestamp Unix w sekundach
    pub temp:     f64, // temperatura °C
    pub humidity: f64, // wilgotność %
}

/// Payload przychodzący przez POST /{uuid}/
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SensorReading {
    /// Identyfikator czujnika — opcjonalny, domyślnie "default"
    #[serde(default = "default_sensor_id")]
    pub sensor_id: String,

    /// Timestamp Unix w sekundach — opcjonalny, serwer uzupełnia now() gdy brak
    pub ts: Option<f64>,

    /// Temperatura w °C
    pub temp: f64,

    /// Wilgotność w %
    pub humidity: f64,
}

fn default_sensor_id() -> String {
    "default".to_owned()
}

impl SensorReading {
    /// Konwertuje do DataPoint, uzupełniając ts = now() gdy brak
    pub fn into_data_point(self) -> DataPoint {
        let ts = self.ts.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
        });
        DataPoint {
            ts,
            temp:     (self.temp     * 10.0).round() / 10.0,
            humidity: (self.humidity * 10.0).round() / 10.0,
        }
    }
}

/// Współdzielony stan aplikacji przekazywany przez Axum
#[derive(Clone)]
pub struct AppState {
    pub history:  Arc<Mutex<Vec<DataPoint>>>,
    pub config:   Arc<Config>,
    pub db_pool:  Option<SqlitePool>,
}

impl AppState {
    pub fn new(config: Config, db_pool: Option<SqlitePool>) -> Self {
        Self {
            history:  Arc::new(Mutex::new(Vec::new())),
            config:   Arc::new(config),
            db_pool,
        }
    }
}
