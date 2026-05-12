use crate::blacklist::BlacklistConfig;
use std::{env, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub mqtt_broker_url: String,
    pub mqtt_topic_prefix: String,
    pub default_sensor_id: String,
    pub temp_min: f64,
    pub temp_max: f64,
    pub humidity_min: f64,
    pub humidity_max: f64,

    /// Blacklista — konfiguracja
    pub blacklist: BlacklistConfig,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        Self {
            bind_addr: env_str("GW_BIND_ADDR", "0.0.0.0:4000"),
            mqtt_broker_url: env_str("GW_MQTT_BROKER_URL", "mqtt://localhost:1883"),
            mqtt_topic_prefix: env_str("GW_MQTT_TOPIC_PREFIX", "sensors"),
            default_sensor_id: env_str("GW_DEFAULT_SENSOR_ID", "default"),
            temp_min: env_f64("TEMP_MIN", 15.0),
            temp_max: env_f64("TEMP_MAX", 35.0),
            humidity_min: env_f64("HUMIDITY_MIN", 20.0),
            humidity_max: env_f64("HUMIDITY_MAX", 90.0),
            blacklist: BlacklistConfig {
                threshold: env_u32("GW_BLACKLIST_THRESHOLD", 5),
                window: Duration::from_secs(env_u64("GW_BLACKLIST_WINDOW", 60)),
                ban_dur: Duration::from_secs(env_u64("GW_BLACKLIST_BAN_DURATION", 300)),
            },
        }
    }

    /// Buduje pełny topic: {prefix}/{uuid}/data
    pub fn topic(&self, uuid: &str) -> String {
        format!("{}/{}/data", self.mqtt_topic_prefix, uuid)
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

fn env_u64(key: &str, default: u64) -> u64 {
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
