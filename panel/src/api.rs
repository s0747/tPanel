use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::db;
use crate::models::{AppState, SensorReading};

/// POST /{uuid}/
///
/// Przyjmuje odczyt z czujnika, zapisuje do historii w pamięci
/// i opcjonalnie do bazy SQLite.
///
/// Kody odpowiedzi:
///   201 Created             — sukces
///   409 Conflict            — ts starszy niż ostatni rekord w historii
///   422 Unprocessable Entity — błędny JSON lub wartości poza zakresem
pub async fn ingest_handler(
    State(state): State<AppState>,
    Json(reading): Json<SensorReading>,
) -> impl IntoResponse {
    let cfg = &state.config;

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

    let sensor_id = reading.sensor_id.clone();
    let point = reading.into_data_point();

    // ── Sprawdzenie kolejności czasowej ────────────────────────────────────
    {
        let history = state.history.lock().unwrap();
        if let Some(last) = history.last() {
            if point.ts < last.ts {
                return (
                    StatusCode::CONFLICT,
                    format!(
                        "ts {:.3} starszy niż ostatni rekord {:.3}",
                        point.ts, last.ts
                    ),
                )
                    .into_response();
            }
        }
    }

    // ── Zapis do bazy SQLite (opcjonalny) ──────────────────────────────────
    if let Some(pool) = &state.db_pool {
        if let Err(e) = db::insert_reading(pool, &sensor_id, &point).await {
            eprintln!("❌ db::insert_reading error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // ── Aktualizacja historii w pamięci ────────────────────────────────────
    {
        let mut history = state.history.lock().unwrap();
        history.push(point);
        let len = history.len();
        if len > cfg.history_max {
            history.drain(..len - cfg.history_max);
        }
    }

    StatusCode::CREATED.into_response()
}
