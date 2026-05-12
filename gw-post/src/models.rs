use serde::{Deserialize, Serialize};

/// Payload przychodzący przez POST /sensors/{uuid}/
#[derive(Debug, Clone, Deserialize)]
pub struct SensorReading {
    /// Timestamp Unix — opcjonalny, gateway uzupełnia now()
    pub ts:       Option<f64>,
    /// Temperatura w °C
    pub temp:     f64,
    /// Wilgotność w %
    pub humidity: f64,
}

/// Payload publikowany na MQTT
#[derive(Debug, Serialize)]
pub struct MqttPayload {
    pub ts:       f64,
    pub temp:     f64,
    pub humidity: f64,
}

impl SensorReading {
    pub fn into_mqtt_payload(self) -> MqttPayload {
        MqttPayload {
            ts:       self.ts.filter(|t| t.is_finite()).unwrap_or_else(now_secs),
            temp:     round1(self.temp),
            humidity: round1(self.humidity),
        }
    }
}

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
