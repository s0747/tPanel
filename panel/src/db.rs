use crate::config::Config;
use crate::models::DataPoint;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};

/// Inicjalizuje pulę połączeń i uruchamia migracje SQL.
pub async fn init_pool(cfg: &Config) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = std::path::Path::new(&cfg.db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let url = format!("sqlite://{}?mode=rwc", cfg.db_path);

    let pool = SqlitePoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&url)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(include_str!("../migrations/001_init.sql"))
        .execute(pool)
        .await?;
    Ok(())
}

/// Zapisuje pojedynczy odczyt do bazy danych.
/// Używa query() zamiast query!() — nie wymaga DATABASE_URL ani sqlx-data.json.
pub async fn insert_reading(
    pool: &SqlitePool,
    sensor_id: &str,
    point: &DataPoint,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO readings (sensor_id, ts, temp, humidity) VALUES (?, ?, ?, ?)")
        .bind(sensor_id)
        .bind(point.ts)
        .bind(point.temp)
        .bind(point.humidity)
        .execute(pool)
        .await?;
    Ok(())
}

/// Ładuje ostatnie `limit` odczytów z bazy, posortowanych od najstarszego.
pub async fn load_recent(pool: &SqlitePool, limit: usize) -> Result<Vec<DataPoint>, sqlx::Error> {
    let limit = limit as i64;

    let rows = sqlx::query(
        r#"
        SELECT ts, temp, humidity
        FROM (
            SELECT ts, temp, humidity
            FROM readings
            ORDER BY ts DESC
            LIMIT ?
        )
        ORDER BY ts ASC
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| DataPoint {
            ts: r.get::<f64, _>("ts"),
            temp: r.get::<f64, _>("temp"),
            humidity: r.get::<f64, _>("humidity"),
        })
        .collect())
}

/// Ładuje punkty z bazy w zakresie czasowym [from, to].
/// Wyniki posortowane od najstarszego (ASC).
pub async fn load_range(
    pool: &SqlitePool,
    from: f64,
    to: f64,
) -> Result<Vec<DataPoint>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT ts, temp, humidity
         FROM readings
         WHERE ts >= ? AND ts <= ?
         ORDER BY ts ASC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| DataPoint {
            ts: r.get::<f64, _>("ts"),
            temp: r.get::<f64, _>("temp"),
            humidity: r.get::<f64, _>("humidity"),
        })
        .collect())
}
