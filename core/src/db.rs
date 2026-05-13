use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use tracing::info;

use crate::config::Config;
use crate::models::{Reading, ReadingRecord};

/// Inicjalizuje pulę SQLite i uruchamia migracje.
pub async fn init_pool(cfg: &Config) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = std::path::Path::new(&cfg.db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let url = format!("sqlite://{}?mode=rwc", cfg.db_path);

    let pool = SqlitePoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&url)
        .await?;

    sqlx::query(include_str!("../migrations/001_init.sql"))
        .execute(&pool)
        .await?;

    info!("✅ SQLite zainicjalizowany: {}", cfg.db_path);
    Ok(pool)
}

/// Zapisuje odczyt do bazy.
pub async fn insert(pool: &SqlitePool, rec: &ReadingRecord) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO readings (sensor_id, ts, temp, humidity) VALUES (?, ?, ?, ?)")
        .bind(&rec.sensor_id)
        .bind(rec.ts)
        .bind(rec.temp)
        .bind(rec.humidity)
        .execute(pool)
        .await?;
    Ok(())
}

/// Zwraca punkty z zakresu [from, to].
/// Opcjonalnie filtruje po sensor_id.
/// Wyniki posortowane ASC, max `limit` rekordów.
pub async fn load_range(
    pool: &SqlitePool,
    from: f64,
    to: f64,
    limit: i64,
    sensor_id: Option<&str>,
) -> Result<Vec<Reading>, sqlx::Error> {
    let rows = match sensor_id {
        Some(id) => {
            sqlx::query(
                "SELECT ts, temp, humidity
             FROM readings
             WHERE ts >= ? AND ts <= ? AND sensor_id = ?
             ORDER BY ts ASC
             LIMIT ?",
            )
            .bind(from)
            .bind(to)
            .bind(id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }

        None => {
            sqlx::query(
                "SELECT ts, temp, humidity
             FROM readings
             WHERE ts >= ? AND ts <= ?
             ORDER BY ts ASC
             LIMIT ?",
            )
            .bind(from)
            .bind(to)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows
        .into_iter()
        .map(|r| Reading {
            ts: r.get::<f64, _>("ts"),
            temp: r.get::<f64, _>("temp"),
            humidity: r.get::<f64, _>("humidity"),
        })
        .collect())
}

/// Zwraca listę unikalnych sensor_id z bazy.
pub async fn list_sensors(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT DISTINCT sensor_id FROM readings ORDER BY sensor_id ASC")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| r.get::<String, _>("sensor_id"))
        .collect())
}

/// Zwraca info o czujniku: liczba rekordów, ostatni odczyt.
/// Zwraca None jeśli czujnik nie istnieje w bazie.
pub async fn sensor_info(
    pool: &SqlitePool,
    sensor_uuid: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT
            COUNT(*)  AS cnt,
            MAX(ts)   AS last_ts,
            (SELECT temp     FROM readings WHERE sensor_id = ? ORDER BY ts DESC LIMIT 1) AS last_temp,
            (SELECT humidity FROM readings WHERE sensor_id = ? ORDER BY ts DESC LIMIT 1) AS last_humid
         FROM readings
         WHERE sensor_id = ?",
    )
    .bind(sensor_uuid)
    .bind(sensor_uuid)
    .bind(sensor_uuid)
    .fetch_one(pool)
    .await?;

    let count: i64 = row.get::<i64, _>("cnt");
    if count == 0 {
        return Ok(None);
    }

    Ok(Some(serde_json::json!({
        "sensor_uuid": sensor_uuid,
        "count":       count,
        "last_ts":     row.get::<Option<f64>, _>("last_ts"),
        "last_temp":   row.get::<Option<f64>, _>("last_temp"),
        "last_humid":  row.get::<Option<f64>, _>("last_humid"),
    })))
}

// ─── Panel sensors ────────────────────────────────────────────────────────────

pub struct PanelSensor {
    pub sensor_uuid: String,
    pub name: Option<String>,
}

/// Zwraca listę czujników przypisanych do panelu.
pub async fn list_panel_sensors(
    pool: &SqlitePool,
    panel_uuid: &str,
) -> Result<Vec<PanelSensor>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT sensor_uuid, name
         FROM panel_sensors
         WHERE panel_uuid = ?
         ORDER BY created_at ASC",
    )
    .bind(panel_uuid)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PanelSensor {
            sensor_uuid: r.get::<String, _>("sensor_uuid"),
            name: r.get::<Option<String>, _>("name"),
        })
        .collect())
}

/// Dodaje czujnik do panelu. Ignoruje duplikaty.
pub async fn add_panel_sensor(
    pool: &SqlitePool,
    panel_uuid: &str,
    sensor_uuid: &str,
    name: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO panel_sensors (panel_uuid, sensor_uuid, name)
         VALUES (?, ?, ?)",
    )
    .bind(panel_uuid)
    .bind(sensor_uuid)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(())
}

/// Usuwa czujnik z panelu.
pub async fn remove_panel_sensor(
    pool: &SqlitePool,
    panel_uuid: &str,
    sensor_uuid: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM panel_sensors
         WHERE panel_uuid = ? AND sensor_uuid = ?",
    )
    .bind(panel_uuid)
    .bind(sensor_uuid)
    .execute(pool)
    .await?;
    Ok(())
}
