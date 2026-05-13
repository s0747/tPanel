use mqtt5::MqttClient;
use tracing::error;

/// Otwiera nowe połączenie MQTT z podanym UUID jako username.
/// Jedno połączenie per request — broker autoryzuje per połączenie.
/// Zwraca Err jeśli połączenie lub publikacja nie powiedzie się.
pub async fn publish_with_auth(
    broker_url: &str,
    uuid: &str,
    topic: &str,
    payload: &str,
) -> Result<(), MqttError> {
    // Client ID unikalny per request — UUID + timestamp
    let client_id = format!("{}-{}", uuid, now_millis());
    let client = MqttClient::new(&client_id);

    // URL z credentials: mqtt://uuid@host:port
    // mqtt5 obsługuje username w URL lub przez opcje połączenia
    let url_with_auth = build_url(broker_url, uuid);

    client.connect(&url_with_auth).await.map_err(|e| {
        // Błąd połączenia — najprawdopodobniej auth failure
        MqttError::Auth(e.to_string())
    })?;

    client
        .publish(topic, payload.as_bytes())
        .await
        .map(|_| ())
        .map_err(|e| {
            error!("❌ MQTT publish error: {e}");
            MqttError::Publish(e.to_string())
        })?;

    // Rozłącz po publikacji
    let _ = client.disconnect().await;

    Ok(())
}

/// Błędy MQTT rozróżniane przez handler
#[derive(Debug)]
pub enum MqttError {
    /// Błąd autoryzacji — nieprawidłowy UUID/credentials
    Auth(String),
    /// Błąd publikacji
    Publish(String),
}

/// Buduje URL MQTT z UUID jako username.
/// Format: mqtt://uuid:uuid@host:port
/// (username=uuid, password=uuid — broker weryfikuje parę)
fn build_url(base_url: &str, uuid: &str) -> String {
    // Wstaw credentials do URL
    // mqtt://host:port → mqtt://uuid:uuid@host:port
    if let Some(rest) = base_url.strip_prefix("mqtt://") {
        format!("mqtt://{}:{}@{}", uuid, uuid, rest)
    } else if let Some(rest) = base_url.strip_prefix("mqtts://") {
        format!("mqtts://{}:{}@{}", uuid, uuid, rest)
    } else if let Some(rest) = base_url.strip_prefix("quic://") {
        format!("quic://{}:{}@{}", uuid, uuid, rest)
    } else {
        // Fallback — bez credentials
        base_url.to_owned()
    }
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_url_with_credentials() {
        assert_eq!(
            build_url("mqtt://localhost:1883", "my-uuid"),
            "mqtt://my-uuid:my-uuid@localhost:1883"
        );
    }

    #[test]
    fn builds_mqtts_url() {
        assert_eq!(
            build_url("mqtts://broker.example.com:8883", "abc-123"),
            "mqtts://abc-123:abc-123@broker.example.com:8883"
        );
    }

    #[test]
    fn builds_quic_url() {
        assert_eq!(
            build_url("quic://localhost:14567", "sensor-01"),
            "quic://sensor-01:sensor-01@localhost:14567"
        );
    }
}
