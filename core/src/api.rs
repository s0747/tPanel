use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::warn;

use crate::db;
use crate::models::RangeResponse;

/// Współdzielony stan Axum
#[derive(Clone)]
pub struct ApiState {
    pub pool: Arc<SqlitePool>,
    pub max_points: usize,
}

/// Parametry zapytania dla endpointów range
#[derive(Deserialize)]
pub struct RangeParams {
    pub from: f64,
    pub to: Option<f64>,
}

// ─── Sensory ──────────────────────────────────────────────────────────────────

/// GET /api/sensors
///
/// Zwraca listę wszystkich czujników zarejestrowanych w bazie.
pub async fn sensors(State(state): State<ApiState>) -> impl IntoResponse {
    match db::list_sensors(&state.pool).await {
        Ok(ids) => Json(ids).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/sensors/:sensor_uuid
///
/// Zwraca info o czujniku: liczba rekordów, ostatni odczyt.
pub async fn sensor_info(
    State(state): State<ApiState>,
    Path(sensor_uuid): Path<String>,
) -> impl IntoResponse {
    match db::sensor_info(&state.pool, &sensor_uuid).await {
        Ok(Some(info)) => Json(info).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/sensors/:sensor_uuid/range?from=X&to=Y
///
/// Zwraca punkty pomiarowe z zakresu [from, to] dla danego czujnika.
///
/// Kody odpowiedzi:
///   200 OK               — lista punktów (może być pusta)
///   400 Bad Request      — nieprawidłowe parametry
///   500 Internal Error   — błąd bazy
pub async fn sensor_range(
    State(state): State<ApiState>,
    Path(sensor_uuid): Path<String>,
    Query(params): Query<RangeParams>,
) -> impl IntoResponse {
    let to = params.to.unwrap_or_else(now_secs);

    if !params.from.is_finite() || !to.is_finite() {
        return (
            StatusCode::BAD_REQUEST,
            "from i to muszą być skończonymi liczbami",
        )
            .into_response();
    }
    if params.from > to {
        return (
            StatusCode::BAD_REQUEST,
            "from musi być mniejsze lub równe to",
        )
            .into_response();
    }

    let limit = state.max_points as i64;

    match db::load_range(&state.pool, params.from, to, limit, Some(&sensor_uuid)).await {
        Ok(points) => {
            let count = points.len();
            if count == limit as usize {
                warn!(
                    "Wynik przycięty do {} punktów — rozważ zmniejszenie zakresu",
                    limit
                );
            }
            Json(RangeResponse {
                from: params.from,
                to,
                sensor_id: Some(sensor_uuid.clone()),
                count,
                points,
            })
            .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/sensors/:sensor_uuid/latest
///
/// Zwraca ostatni odczyt dla danego czujnika.
pub async fn sensor_latest(
    State(state): State<ApiState>,
    Path(sensor_uuid): Path<String>,
) -> impl IntoResponse {
    match db::load_range(&state.pool, 0.0, now_secs(), 1, Some(&sensor_uuid)).await {
        Ok(points) if !points.is_empty() => Json(points.into_iter().last()).into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ─── Panele ───────────────────────────────────────────────────────────────────

/// GET /api/panels/:panel_uuid/sensors
///
/// Zwraca listę czujników przypisanych do panelu.
pub async fn panel_sensors(
    State(state): State<ApiState>,
    Path(panel_uuid): Path<String>,
) -> impl IntoResponse {
    match db::list_panel_sensors(&state.pool, &panel_uuid).await {
        Ok(sensors) => {
            let body: Vec<serde_json::Value> = sensors
                .into_iter()
                .map(|s| {
                    serde_json::json!({
                        "sensor_uuid": s.sensor_uuid,
                        "name":        s.name,
                    })
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// POST /api/panels/:panel_uuid/sensors
///
/// Dodaje czujnik do panelu.
/// Body: { "sensor_uuid": "abc-123", "name": "Salon" }
pub async fn add_panel_sensor(
    State(state): State<ApiState>,
    Path(panel_uuid): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let sensor_uuid = match body["sensor_uuid"].as_str() {
        Some(s) if !s.is_empty() => s.to_owned(),
        _ => return (StatusCode::BAD_REQUEST, "brak sensor_uuid").into_response(),
    };
    let name = body["name"].as_str().map(str::to_owned);

    match db::add_panel_sensor(&state.pool, &panel_uuid, &sensor_uuid, name.as_deref()).await {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// DELETE /api/panels/:panel_uuid/sensors/:sensor_uuid
///
/// Usuwa czujnik z panelu.
pub async fn remove_panel_sensor(
    State(state): State<ApiState>,
    Path((panel_uuid, sensor_uuid)): Path<(String, String)>,
) -> impl IntoResponse {
    match db::remove_panel_sensor(&state.pool, &panel_uuid, &sensor_uuid).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ─── Health ───────────────────────────────────────────────────────────────────

/// GET /health
pub async fn health() -> impl IntoResponse {
    StatusCode::OK
}

// ─── Helper ───────────────────────────────────────────────────────────────────

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}
