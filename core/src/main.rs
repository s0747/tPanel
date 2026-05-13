mod api;
mod config;
mod db;
mod models;
mod subscriber;

use axum::{routing::get, Router};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use api::ApiState;
use config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Arc::new(Config::from_env());

    info!("🔧 core konfiguracja:");
    info!("   CORE_BIND_ADDR        = {}", cfg.bind_addr);
    info!("   CORE_MQTT_BROKER_URL  = {}", cfg.mqtt_broker_url);
    info!("   CORE_MQTT_CLIENT_ID   = {}", cfg.mqtt_client_id);
    info!("   CORE_MQTT_TOPIC       = {}", cfg.mqtt_topic);
    info!("   CORE_DB_PATH          = {}", cfg.db_path);
    info!("   CORE_API_MAX_POINTS   = {}", cfg.api_max_points);

    // ── Inicjalizacja SQLite ──────────────────────────────────────────────
    let pool = Arc::new(db::init_pool(&cfg).await.unwrap_or_else(|e| {
        eprintln!("❌ Błąd inicjalizacji SQLite: {e}");
        std::process::exit(1);
    }));

    // ── MQTT subscriber jako osobny task ──────────────────────────────────
    {
        let cfg_sub = cfg.clone();
        let pool_sub = pool.clone();
        tokio::spawn(async move {
            subscriber::run(cfg_sub, pool_sub).await;
        });
    }

    // ── HTTP API ──────────────────────────────────────────────────────────
    let api_state = ApiState {
        pool: pool.clone(),
        max_points: cfg.api_max_points,
    };

    let app = Router::new()
        // Sensory
        .route("/api/sensors", get(api::sensors))
        .route("/api/sensors/:sensor_uuid", get(api::sensor_info))
        .route("/api/sensors/:sensor_uuid/range", get(api::sensor_range))
        .route("/api/sensors/:sensor_uuid/latest", get(api::sensor_latest))
        // Panele
        .route(
            "/api/panels/:panel_uuid/sensors",
            get(api::panel_sensors).post(api::add_panel_sensor),
        )
        .route(
            "/api/panels/:panel_uuid/sensors/:sensor_uuid",
            axum::routing::delete(api::remove_panel_sensor),
        )
        // Health
        .route("/health", get(api::health))
        .with_state(api_state);

    let bind_addr = cfg.bind_addr.clone();
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();

    info!("\n✅ core uruchomiony → http://{}", bind_addr);
    info!("   GET    /api/sensors                              — lista czujników");
    info!("   GET    /api/sensors/{{uuid}}                     — info o czujniku");
    info!("   GET    /api/sensors/{{uuid}}/range?from=X[&to=Y] — historia");
    info!("   GET    /api/sensors/{{uuid}}/latest              — ostatni odczyt");
    info!("   GET    /api/panels/{{uuid}}/sensors              — czujniki panelu");
    info!("   POST   /api/panels/{{uuid}}/sensors              — dodaj czujnik");
    info!("   DELETE /api/panels/{{uuid}}/sensors/{{s_uuid}}   — usuń czujnik");
    info!("   GET    /health                                   — health check");

    axum::serve(listener, app).await.unwrap();
}
