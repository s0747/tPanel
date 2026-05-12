mod blacklist;
mod config;
mod handler;
mod models;
mod mqtt;

use std::sync::Arc;
use axum::{Router, routing::{get, post}};
use tracing::info;
use tracing_subscriber::EnvFilter;

use blacklist::Blacklist;
use config::Config;
use handler::GwState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    let cfg = Arc::new(Config::from_env());

    info!("🔧 gw-post konfiguracja:");
    info!("   GW_BIND_ADDR              = {}", cfg.bind_addr);
    info!("   GW_MQTT_BROKER_URL        = {}", cfg.mqtt_broker_url);
    info!("   GW_MQTT_TOPIC_PREFIX      = {}", cfg.mqtt_topic_prefix);
    info!("   TEMP                      = {}–{}°C", cfg.temp_min, cfg.temp_max);
    info!("   HUMIDITY                  = {}–{}%", cfg.humidity_min, cfg.humidity_max);
    info!("   GW_BLACKLIST_THRESHOLD    = {}", cfg.blacklist.threshold);
    info!("   GW_BLACKLIST_WINDOW       = {}s", cfg.blacklist.window.as_secs());
    info!("   GW_BLACKLIST_BAN_DURATION = {}s", cfg.blacklist.ban_dur.as_secs());

    let blacklist = Arc::new(Blacklist::new(cfg.blacklist.clone()));

    // ── Task czyszczący blacklistę ─────────────────────────────────────────
    {
        let bl  = blacklist.clone();
        let win = cfg.blacklist.window;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(win).await;
                bl.cleanup();
            }
        });
    }

    let state = GwState { cfg: cfg.clone(), blacklist };

    let app = Router::new()
        .route("/sensors/:uuid/", post(handler::ingest))
        .route("/health",         get(handler::health))
        .with_state(state)
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    let bind_addr = cfg.bind_addr.clone();
    let listener  = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();

    info!("\n✅ gw-post uruchomiony → http://{}", bind_addr);
    info!("   POST /sensors/{{uuid}}/  — przyjmuje odczyty");
    info!("   GET  /health            — health check");

    axum::serve(listener, app).await.unwrap();
}
