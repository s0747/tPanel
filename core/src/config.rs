use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    /// Adres HTTP API archiverа
    pub bind_addr: String,

    /// URL brokera MQTT
    pub mqtt_broker_url: String,

    /// Client ID w brokerze
    pub mqtt_client_id: String,

    /// Topic MQTT do subskrypcji (wildcard)
    pub mqtt_topic: String,

    /// Ścieżka do pliku SQLite
    pub db_path: String,

    /// Maksymalna liczba połączeń w puli SQLx
    pub db_max_connections: u32,

    /// Maksymalna liczba punktów zwracanych przez /api/range
    pub api_max_points: usize,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        Self {
            bind_addr: env_str("CORE_BIND_ADDR", "0.0.0.0:4001"),
            mqtt_broker_url: env_str("CORE_MQTT_BROKER_URL", "mqtt://localhost:1883"),
            mqtt_client_id: env_str("CORE_MQTT_CLIENT_ID", "archiver-01"),
            mqtt_topic: env_str("CORE_MQTT_TOPIC", "sensors/#"),
            db_path: env_str("CORE_DB_PATH", "./data/readings.db"),
            db_max_connections: env_u32("CORE_DB_MAX_CONN", 5),
            api_max_points: env_usize("CORE_API_MAX_POINTS", 86400),
        }
    }
}

fn env_str(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn env_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
