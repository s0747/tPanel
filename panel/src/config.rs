use std::env;

/// Nazwa i wersja aplikacji pobierane z Cargo.toml w czasie kompilacji
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Konfiguracja aplikacji wczytywana ze zmiennych środowiskowych / pliku .env
#[derive(Debug, Clone)]
pub struct Config {
    /// Adres nasłuchiwania serwera, np. "0.0.0.0:3000"
    pub bind_addr: String,

    /// Interwał między próbkami SSE w sekundach
    pub sse_interval_secs: u64,

    /// Liczba próbek historii trzymanych w pamięci (max)
    pub history_max: usize,

    /// Liczba próbek historii ładowanych z bazy przy starcie
    pub history_seed: usize,

    /// Maksymalna liczba punktów wyświetlanych na wykresie
    pub chart_max_points: usize,

    /// UUID izolujący URL aplikacji, np. "a1b2c3d4-..."
    pub uuid: String,

    /// Zakres temperatury: min
    pub temp_min: f64,

    /// Zakres temperatury: max
    pub temp_max: f64,

    /// Zakres wilgotności: min
    pub humidity_min: f64,

    /// Zakres wilgotności: max
    pub humidity_max: f64,

    /// Włącz persystencję danych w SQLite
    pub db_enabled: bool,

    /// Ścieżka do pliku bazy danych SQLite
    pub db_path: String,

    /// Maksymalna liczba połączeń w puli SQLx
    pub db_max_connections: u32,

    /// Whitelist CIDR dla POST — jeśli niepusta, przepuszcza tylko wymienione adresy
    /// Format: "192.168.1.0/24,10.0.0.1" — przecinek jako separator
    pub sensor_ip_whitelist: String,

    /// Blacklist CIDR dla POST — blokuje wymienione adresy (ignorowana gdy whitelist niepusta)
    pub sensor_ip_blacklist: String,

    /// Maksymalna liczba requestów POST w oknie czasowym (per IP)
    pub rate_limit_max: u32,

    /// Okno czasowe rate limitera w sekundach
    pub rate_limit_window_secs: u64,
}

impl Config {
    /// Wczytuje konfigurację ze zmiennych środowiskowych.
    /// Najpierw ładuje plik `.env` (jeśli istnieje), potem czyta `std::env`.
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        Self {
            bind_addr: env_str("BIND_ADDR", "0.0.0.0:3000"),
            sse_interval_secs: env_u64("SSE_INTERVAL_SECS", 2),
            history_max: env_usize("HISTORY_MAX", 200),
            history_seed: env_usize("HISTORY_SEED", 30),
            chart_max_points: env_usize("CHART_MAX_POINTS", 60),
            uuid: env_str("APP_UUID", "default"),
            temp_min: env_f64("TEMP_MIN", 15.0),
            temp_max: env_f64("TEMP_MAX", 35.0),
            humidity_min: env_f64("HUMIDITY_MIN", 20.0),
            humidity_max: env_f64("HUMIDITY_MAX", 90.0),
            db_enabled: env_bool("DB_ENABLED", false),
            db_path: env_str("DB_PATH", "./data/readings.db"),
            db_max_connections: env_u32("DB_MAX_CONNECTIONS", 5),
            sensor_ip_whitelist: env_str("SENSOR_IP_WHITELIST", ""),
            sensor_ip_blacklist: env_str("SENSOR_IP_BLACKLIST", ""),
            rate_limit_max: env_u32("RATE_LIMIT_MAX", 60),
            rate_limit_window_secs: env_u64("RATE_LIMIT_WINDOW_SECS", 60),
        }
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

fn env_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
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

fn env_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
