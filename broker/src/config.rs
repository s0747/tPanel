use std::env;

/// Konfiguracja mqtt-broker wczytywana z .env
#[derive(Debug, Clone)]
pub struct Config {
    // ── Broker ────────────────────────────────────────────────────────────────

    /// Adres TCP brokera MQTT
    pub mqtt_tcp_addr: String,

    /// Adres TLS brokera MQTT (pusty = wyłączony)
    pub mqtt_tls_addr: String,

    /// Ścieżka do certyfikatu TLS (PEM)
    pub mqtt_tls_cert: String,

    /// Ścieżka do klucza TLS (PEM)
    pub mqtt_tls_key: String,

    /// Adres QUIC brokera MQTT (pusty = wyłączony)
    pub mqtt_quic_addr: String,

    // ── Bridge ────────────────────────────────────────────────────────────────

    /// Topic MQTT na który subskrybuje bridge (wildcard)
    /// Format: sensors/# lub sensors/+/data
    pub bridge_topic: String,

    /// URL endpointu POST weather-dashboard
    pub bridge_target_url: String,

    /// Czy wyciągać sensor_id z nazwy topicu
    /// true  → sensors/{sensor_id}/data → sensor_id z topicu
    /// false → sensor_id z pola JSON payloadu
    pub bridge_sensor_id_from_topic: bool,

    /// Domyślny sensor_id gdy nie można wyciągnąć z topicu
    pub bridge_default_sensor_id: String,

    /// Client ID bridge'a w brokerze
    pub bridge_client_id: String,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        Self {
            mqtt_tcp_addr:  env_str("MQTT_TCP_ADDR",  "0.0.0.0:1883"),
            mqtt_tls_addr:  env_str("MQTT_TLS_ADDR",  ""),
            mqtt_tls_cert:  env_str("MQTT_TLS_CERT",  "certs/server.crt"),
            mqtt_tls_key:   env_str("MQTT_TLS_KEY",   "certs/server.key"),
            mqtt_quic_addr: env_str("MQTT_QUIC_ADDR", ""),
            bridge_topic:                env_str("BRIDGE_TOPIC",       "sensors/#"),
            bridge_target_url:           env_str("BRIDGE_TARGET_URL",  "http://localhost:3000/default/"),
            bridge_sensor_id_from_topic: env_bool("BRIDGE_SENSOR_ID_FROM_TOPIC", true),
            bridge_default_sensor_id:    env_str("BRIDGE_DEFAULT_SENSOR_ID", "mqtt-sensor"),
            bridge_client_id:            env_str("BRIDGE_CLIENT_ID",  "mqtt-bridge-01"),
        }
    }

    pub fn tls_enabled(&self) -> bool {
        !self.mqtt_tls_addr.is_empty()
    }

    pub fn quic_enabled(&self) -> bool {
        !self.mqtt_quic_addr.is_empty()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn env_str(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .and_then(|v| match v.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}
