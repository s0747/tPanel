use reqwest::Client;
use serde::Serialize;
use tracing::{info, warn, error};

/// Payload JSON wysyłany do weather-dashboard — identyczny z HTTP POST sendera
#[derive(Serialize)]
struct SensorReading {
    sensor_id: String,
    ts:        f64,
    temp:      f64,
    humidity:  f64,
}

/// Typ aliasu dla nadawcy kanału forward
//pub type ForwardSender = tokio::sync::mpsc::Sender<mqtt5::broker::ClientPublishEvent>;

/// Wyciąga sensor_id z nazwy topicu.
/// Oczekiwany format: sensors/{sensor_id}/data lub sensors/{sensor_id}
/// Zwraca None jeśli format nie pasuje.
fn sensor_id_from_topic(topic: &str) -> Option<String> {
    let parts: Vec<&str> = topic.split('/').collect();
    // sensors/{sensor_id}/data → parts[1]
    // sensors/{sensor_id}     → parts[1]
    if parts.len() >= 2 && parts[0] == "sensors" {
        Some(parts[1].to_owned())
    } else {
        None
    }
}

/// Parsuje payload JSON z wiadomości MQTT.
/// Payload może zawierać sensor_id lub nie — jeśli brak, używamy sensor_id z topicu.
fn parse_payload(
    payload: &[u8],
    topic: &str,
    sensor_id_from_topic_flag: bool,
    default_sensor_id: &str,
) -> Option<SensorReading> {
    // Parsuj jako ogólny JSON object
    let value: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| warn!("Błąd parsowania JSON z topic '{}': {e}", topic))
        .ok()?;

    let temp     = value["temp"].as_f64()
        .or_else(|| value["temperature"].as_f64())?;
    let humidity = value["humidity"].as_f64()
        .or_else(|| value["hum"].as_f64())?;

    // ts: z payloadu lub now()
    let ts = value["ts"].as_f64()
        .or_else(|| value["timestamp"].as_f64())
        .unwrap_or_else(now_secs);

    // sensor_id: z topicu lub z payloadu lub domyślny
    let sensor_id = if sensor_id_from_topic_flag {
        sensor_id_from_topic(topic)
            .unwrap_or_else(|| {
                value["sensor_id"].as_str()
                    .unwrap_or(default_sensor_id)
                    .to_owned()
            })
    } else {
        value["sensor_id"].as_str()
            .unwrap_or(default_sensor_id)
            .to_owned()
    };

    Some(SensorReading { sensor_id, ts, temp, humidity })
}

/// Wysyła odczyt do weather-dashboard przez HTTP POST.
async fn forward_reading(
    client: &Client,
    target_url: &str,
    reading: &SensorReading,
) {
    match client.post(target_url).json(reading).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                info!(
                    "[bridge] ✓ {}  sensor={}  temp={:.1}°C  humidity={:.1}%",
                    status, reading.sensor_id, reading.temp, reading.humidity
                );
            } else {
                let body = resp.text().await.unwrap_or_default();
                warn!("[bridge] ✗ {}  {}", status, body.trim());
            }
        }
        Err(e) => error!("[bridge] ✗ HTTP error: {e}"),
    }
}

/// Pętla bridge'a — odbiera zdarzenia z kanału i forwarduje do weather-dashboard.
/// Działa jako osobny tokio task.
pub async fn run_bridge(
    mut rx: tokio::sync::mpsc::Receiver<mqtt5::broker::ClientPublishEvent>,
    target_url: String,
    sensor_id_from_topic_flag: bool,
    default_sensor_id: String,
) {
    let client = Client::new();
    info!("[bridge] Uruchomiony → {}", target_url);

    while let Some(event) = rx.recv().await {
        let topic   = &event.topic;
        let payload = &event.payload;

        match parse_payload(payload, topic, sensor_id_from_topic_flag, &default_sensor_id) {
            Some(reading) => {
                forward_reading(&client, &target_url, &reading).await;
            }
            None => {
                warn!(
                    "[bridge] Pominięto wiadomość z topic '{}' — nie można sparsować payloadu",
                    topic
                );
            }
        }
    }

    warn!("[bridge] Kanał zamknięty — bridge kończy działanie");
}

// ─── Helper ───────────────────────────────────────────────────────────────────

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

// ─── Testy ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_sensor_id_from_topic() {
        assert_eq!(
            sensor_id_from_topic("sensors/sensor-01/data"),
            Some("sensor-01".to_owned())
        );
        assert_eq!(
            sensor_id_from_topic("sensors/room-temp"),
            Some("room-temp".to_owned())
        );
        assert_eq!(sensor_id_from_topic("other/topic"), None);
        assert_eq!(sensor_id_from_topic("sensors"), None);
    }

    #[test]
    fn parses_standard_payload() {
        let json = br#"{"temp": 22.4, "humidity": 58.1}"#;
        let r = parse_payload(json, "sensors/s1/data", true, "default").unwrap();
        assert_eq!(r.temp, 22.4);
        assert_eq!(r.humidity, 58.1);
        assert_eq!(r.sensor_id, "s1");
    }

    #[test]
    fn parses_payload_with_sensor_id() {
        let json = br#"{"sensor_id": "ext-01", "temp": 20.0, "humidity": 50.0}"#;
        // sensor_id_from_topic=false → bierz z payloadu
        let r = parse_payload(json, "sensors/ignored/data", false, "default").unwrap();
        assert_eq!(r.sensor_id, "ext-01");
    }

    #[test]
    fn parses_alternative_field_names() {
        let json = br#"{"temperature": 19.5, "hum": 60.0}"#;
        let r = parse_payload(json, "sensors/s2/data", true, "default").unwrap();
        assert_eq!(r.temp, 19.5);
        assert_eq!(r.humidity, 60.0);
    }

    #[test]
    fn rejects_missing_temp() {
        let json = br#"{"humidity": 50.0}"#;
        assert!(parse_payload(json, "sensors/s1/data", true, "default").is_none());
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_payload(b"not json", "sensors/s1/data", true, "default").is_none());
    }
}
