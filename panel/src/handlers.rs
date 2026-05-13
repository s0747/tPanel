use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive},
    response::{Html, Sse},
};
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;

use crate::db;
use crate::models::AppState;
use crate::views;

pub async fn index_handler(State(state): State<AppState>) -> Html<String> {
    let history = state.history.lock().unwrap();
    Html(views::page(&history, &state.config).into_string())
}

/// Parametry zapytania SSE
/// GET /{uuid}/sse                         → tylko live
/// GET /{uuid}/sse?from=1744000000         → historia od from do teraz + live
/// GET /{uuid}/sse?from=1744000000&to=...  → historia w zakresie + live
#[derive(Deserialize)]
pub struct SseParams {
    pub from: Option<f64>,
    pub to: Option<f64>,
}

/// SSE handler — dwufazowy strumień:
///
/// Faza 1 (gdy podano `from`): replay z bazy SQLite w zakresie [from, to]
///   → zdarzenia: "history" (punkt) i "history_end" (koniec fazy)
///
/// Faza 2: live stream — nowe punkty trafiające do AppState.history przez POST/MQTT
///   → zdarzenia: "sensor"
pub async fn sse_handler(
    State(state): State<AppState>,
    Query(params): Query<SseParams>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {

        // ── Faza 1: replay z bazy ─────────────────────────────────────────────
        if let Some(from) = params.from {
            if let Some(pool) = &state.db_pool {
                let to = params.to.unwrap_or_else(now_secs);

                match db::load_range(pool, from, to).await {
                    Ok(points) => {
                        let count = points.len();
                        for point in &points {
                            let data = serde_json::json!({
                                "ts":       point.ts,
                                "temp":     point.temp,
                                "humidity": point.humidity,
                                "source":   "db",
                            });
                            yield Ok(Event::default()
                                .event("history")
                                .data(data.to_string()));

                            // Mała pauza — nie zalewamy klienta
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }

                        // Sygnał końca fazy historii
                        let end_data = serde_json::json!({ "count": count });
                        yield Ok(Event::default()
                            .event("history_end")
                            .data(end_data.to_string()));
                    }
                    Err(e) => {
                        eprintln!("⚠️  db::load_range error: {e}");
                        // Wyślij history_end z count=0 żeby klient wiedział
                        // że faza 1 się skończyła mimo błędu
                        yield Ok(Event::default()
                            .event("history_end")
                            .data(r#"{"count":0,"error":true}"#));
                    }
                }
            } else {
                // DB wyłączone — poinformuj klienta i przejdź od razu do live
                yield Ok(Event::default()
                    .event("history_end")
                    .data(r#"{"count":0,"db_disabled":true}"#));
            }
        }

        // ── Faza 2: live stream ───────────────────────────────────────────────
        // Startujemy od `to` (jeśli podano) żeby nie duplikować punktów
        // które były już w zakresie historii
        let mut last_sent_ts = params.to.unwrap_or(0.0);
        let interval_secs = state.config.sse_interval_secs;

        loop {
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;

            let maybe_event = {
                let history = state.history.lock().unwrap();
                history.last().and_then(|point| {
                    if point.ts > last_sent_ts {
                        last_sent_ts = point.ts;
                        let data = serde_json::json!({
                            "ts":       point.ts,
                            "temp":     point.temp,
                            "humidity": point.humidity,
                            "source":   "live",
                        });
                        Some(Ok::<Event, Infallible>(
                            Event::default()
                                .event("sensor")
                                .data(data.to_string())
                        ))
                    } else {
                        None
                    }
                })
            };

            if let Some(event) = maybe_event {
                yield event;
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ─── Helper ───────────────────────────────────────────────────────────────────

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}
