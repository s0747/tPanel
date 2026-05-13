use mqtt5::MqttClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::db;
use crate::models::{sensor_id_from_topic, Reading, ReadingRecord};

/// Łączy się z brokerem i subskrybuje topic czujników.
/// Przy każdej wiadomości parsuje payload i zapisuje do SQLite.
/// Automatyczny retry przy utracie połączenia.
pub async fn run(cfg: Arc<Config>, pool: Arc<SqlitePool>) {
    loop {
        let client = MqttClient::new(&cfg.mqtt_client_id);

        // ── Połączenie ────────────────────────────────────────────────────
        match client.connect(&cfg.mqtt_broker_url).await {
            Ok(_) => info!("📡 MQTT połączono: {}", cfg.mqtt_broker_url),
            Err(e) => {
                warn!("⚠️  MQTT błąd połączenia: {e} — retry za 3s");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
        }

        // ── Subskrypcja ───────────────────────────────────────────────────
        let pool_sub = pool.clone();
        let result = client
            .subscribe(&cfg.mqtt_topic, move |msg| {
                let pool = pool_sub.clone();
                let topic = msg.topic.clone();
                let payload = msg.payload.clone();
                tokio::spawn(async move {
                    handle_message(&pool, &topic, &payload).await;
                });
            })
            .await;

        match result {
            Ok(_) => info!("📋 Subskrybuję: {}", cfg.mqtt_topic),
            Err(e) => {
                error!("❌ Błąd subskrypcji: {e} — retry za 3s");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
        }

        // ── Trzymaj połączenie — odpytuj is_connected() co sekundę ───────
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if !client.is_connected().await {
                warn!("⚠️  MQTT rozłączono — retry za 3s");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                break; // wróć do zewnętrznej pętli → nowe połączenie
            }
        }
    }
}

/// Parsuje wiadomość MQTT i zapisuje do SQLite.
async fn handle_message(pool: &SqlitePool, topic: &str, payload: &[u8]) {
    let reading: Reading = match serde_json::from_slice(payload) {
        Ok(r) => r,
        Err(e) => {
            warn!("Pominięto wiadomość z topic '{}' — błąd JSON: {e}", topic);
            return;
        }
    };

    if !reading.ts.is_finite() || !reading.temp.is_finite() || !reading.humidity.is_finite() {
        warn!(
            "Pominięto wiadomość z topic '{}' — wartości nie są skończone",
            topic
        );
        return;
    }

    let rec = ReadingRecord {
        sensor_id: sensor_id_from_topic(topic),
        ts: reading.ts,
        temp: reading.temp,
        humidity: reading.humidity,
    };

    match db::insert(pool, &rec).await {
        Ok(_) => info!(
            "💾 Zapisano: sensor={} ts={:.0} temp={:.1}°C humidity={:.1}%",
            rec.sensor_id, rec.ts, rec.temp, rec.humidity
        ),
        Err(e) => error!("❌ Błąd zapisu do SQLite: {e}"),
    }
}
