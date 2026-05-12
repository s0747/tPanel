-- Tabela odczytów z czujników
CREATE TABLE IF NOT EXISTS readings (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_id  TEXT    NOT NULL DEFAULT 'default',
    ts         REAL    NOT NULL,
    temp       REAL    NOT NULL,
    humidity   REAL    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_readings_ts     ON readings (ts DESC);
CREATE INDEX IF NOT EXISTS idx_readings_sensor ON readings (sensor_id, ts DESC);
