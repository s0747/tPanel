use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, warn};

use crate::blacklist::Blacklist;
use crate::config::Config;
use crate::models::SensorReading;
use crate::mqtt::{self, MqttError};

/// Stan Axum
#[derive(Clone)]
pub struct GwState {
    pub cfg: Arc<Config>,
    pub blacklist: Arc<Blacklist>,
}

/// POST /sensors/{uuid}/
///
/// 1. Sprawdza blacklistę IP
/// 2. Waliduje wartości
/// 3. Otwiera połączenie MQTT z username={uuid}
/// 4. Publikuje payload
/// 5. Aktualizuje blacklistę na podstawie wyniku
///
/// Kody odpowiedzi:
///   201 Created              — sukces
///   403 Forbidden            — IP na blackliście
///   422 Unprocessable Entity — wartości poza zakresem
///   401 Unauthorized         — broker odrzucił UUID (zły credentials)
///   502 Bad Gateway          — błąd publikacji MQTT
pub async fn ingest(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<GwState>,
    Path(uuid): Path<String>,
    Json(reading): Json<SensorReading>,
) -> impl IntoResponse {
    let ip = addr.ip();
    let cfg = &state.cfg;

    // ── Sprawdź blacklistę ────────────────────────────────────────────────
    if state.blacklist.is_banned(ip) {
        warn!("🚫 Zablokowano: {} (blacklista)", ip);
        return StatusCode::FORBIDDEN.into_response();
    }

    // ── Walidacja zakresu ──────────────────────────────────────────────────
    if reading.temp < cfg.temp_min || reading.temp > cfg.temp_max {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "temp {:.1} poza zakresem [{}, {}]",
                reading.temp, cfg.temp_min, cfg.temp_max
            ),
        )
            .into_response();
    }

    if reading.humidity < cfg.humidity_min || reading.humidity > cfg.humidity_max {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "humidity {:.1} poza zakresem [{}, {}]",
                reading.humidity, cfg.humidity_min, cfg.humidity_max
            ),
        )
            .into_response();
    }

    // ── Przygotuj topic i payload ──────────────────────────────────────────
    let topic = cfg.topic(&uuid);
    let payload = reading.into_mqtt_payload();
    let payload_str = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // ── Publikuj przez MQTT (nowe połączenie z UUID jako credentials) ──────
    match mqtt::publish_with_auth(&cfg.mqtt_broker_url, &uuid, &topic, &payload_str).await {
        Ok(_) => {
            state.blacklist.record_success(ip);
            info!(
                "✓ PUBLISH topic='{}' ip={} temp={:.1}°C humidity={:.1}%",
                topic, ip, payload.temp, payload.humidity
            );
            StatusCode::CREATED.into_response()
        }
        Err(MqttError::Auth(e)) => {
            let newly_banned = state.blacklist.record_failure(ip);
            warn!("✗ Auth failure: ip={} uuid={} err={}", ip, uuid, e);
            if newly_banned {
                warn!("🚫 IP {} trafił na blacklistę", ip);
            }
            StatusCode::UNAUTHORIZED.into_response()
        }
        Err(MqttError::Publish(e)) => {
            (StatusCode::BAD_GATEWAY, format!("MQTT publish error: {e}")).into_response()
        }
    }
}

/// GET /health
pub async fn health() -> impl IntoResponse {
    StatusCode::OK
}
