mod api;
mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod views;

//use std::time::Duration;
//use axum::{Router, routing::{get, post}, middleware as axum_middleware};
//use axum::extract::Extension;
use axum::{Router, routing::{get, post}};
use tower_http::services::ServeDir;
use config::{Config, APP_NAME, APP_VERSION};
use models::AppState;

/* use middleware::ip_filter::{parse_cidr_list, ip_filter_middleware};
use middleware::rate_limit::{
    new_rate_limit_map, spawn_cleanup_task,
    rate_limit_middleware, RateLimitConfig,
}; */

#[tokio::main]
async fn main() {
    let cfg = Config::from_env();

    println!("🌡️  {} v{}", APP_NAME, APP_VERSION);
    println!("⚙️  Konfiguracja:");
    println!("   BIND_ADDR             = {}", cfg.bind_addr);
    println!("   APP_UUID              = {}", cfg.uuid);
    println!("   SSE_INTERVAL_SECS     = {}", cfg.sse_interval_secs);
    println!("   HISTORY_MAX           = {}", cfg.history_max);
    println!("   HISTORY_SEED          = {}", cfg.history_seed);
    println!("   CHART_MAX_POINTS      = {}", cfg.chart_max_points);
    println!("   TEMP                  = {}–{}°C", cfg.temp_min, cfg.temp_max);
    println!("   HUMIDITY              = {}–{}%", cfg.humidity_min, cfg.humidity_max);
    println!("   DB_ENABLED            = {}", cfg.db_enabled);
    if cfg.db_enabled {
        println!("   DB_PATH               = {}", cfg.db_path);
        println!("   DB_MAX_CONN           = {}", cfg.db_max_connections);
    }
    println!("   SENSOR_IP_WHITELIST   = {}", if cfg.sensor_ip_whitelist.is_empty() { "(brak)" } else { &cfg.sensor_ip_whitelist });
    println!("   SENSOR_IP_BLACKLIST   = {}", if cfg.sensor_ip_blacklist.is_empty() { "(brak)" } else { &cfg.sensor_ip_blacklist });
    println!("   RATE_LIMIT_MAX        = {} req", cfg.rate_limit_max);
    println!("   RATE_LIMIT_WINDOW     = {}s", cfg.rate_limit_window_secs);

    // ── Inicjalizacja bazy danych ──────────────────────────────────────────
    let db_pool = if cfg.db_enabled {
        match db::init_pool(&cfg).await {
            Ok(pool) => {
                println!("✅ SQLite zainicjalizowany: {}", cfg.db_path);
                Some(pool)
            }
            Err(e) => {
                eprintln!("❌ Błąd inicjalizacji SQLite: {e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let state = AppState::new(cfg, db_pool);

    // ── Historia startowa ──────────────────────────────────────────────────
    if state.config.db_enabled {
        if let Some(pool) = &state.db_pool {
            match db::load_recent(pool, state.config.history_seed).await {
                Ok(points) => {
                    let count = points.len();
                    let mut history = state.history.lock().unwrap();
                    *history = points;
                    println!("📂 Załadowano {count} rekordów z bazy do historii");
                }
                Err(e) => eprintln!("⚠️  Błąd ładowania historii z bazy: {e}"),
            }
        }
    } else {
        println!("📂 Historia startowa pusta (DB_ENABLED=false)");
    }

    // ── Middleware — IP filter ─────────────────────────────────────────────
/*     let whitelist = parse_cidr_list(&state.config.sensor_ip_whitelist);
    let blacklist = parse_cidr_list(&state.config.sensor_ip_blacklist);

    if !whitelist.is_empty() {
        println!("🔒 IP whitelist aktywna: {} wpisów", whitelist.len());
    } else if !blacklist.is_empty() {
        println!("🔒 IP blacklist aktywna: {} wpisów", blacklist.len());
    } else {
        println!("🔓 Filtrowanie IP wyłączone");
    } */

    // ── Middleware — Rate limiter ──────────────────────────────────────────
/*     let rate_map = new_rate_limit_map();
    let rate_cfg = RateLimitConfig {
        max_requests: state.config.rate_limit_max,
        window:       Duration::from_secs(state.config.rate_limit_window_secs),
    };

    // Uruchom task czyszczący nieaktywne wpisy
    spawn_cleanup_task(rate_map.clone(), rate_cfg.window); 


    println!(
        "⏱️  Rate limiter: max {} req / {}s per IP",
        rate_cfg.max_requests, state.config.rate_limit_window_secs
    );
*/
    let uuid = state.config.uuid.clone();

    // ── Trasa POST z middleware ────────────────────────────────────────────
    let post_route = Router::new()
        .route(&format!("/{uuid}/"), post(api::ingest_handler))
/*         .layer(axum_middleware::from_fn(rate_limit_middleware))
        .layer(axum_middleware::from_fn(ip_filter_middleware))
        .layer(Extension(whitelist))
        .layer(Extension(blacklist))
        .layer(Extension(rate_map))
        .layer(Extension(rate_cfg)) */
        ;

    let app = Router::new()
        .route(&format!("/{uuid}/"),              get(handlers::index_handler))
        .route(&format!("/{uuid}/sse"),           get(handlers::sse_handler))
        .nest_service(&format!("/{uuid}/static"), ServeDir::new("static"))
        .merge(post_route)
        .with_state(state.clone());

    let addr = state.config.bind_addr.clone();
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("\n✅ {} v{} uruchomiony → http://{}/{}/", APP_NAME, APP_VERSION, addr, uuid);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
