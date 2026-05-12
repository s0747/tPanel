//use rand::Rng;
use rand::RngExt;
use std::io::{self, Write};
use std::time::Duration;

// ─── Konfiguracja ─────────────────────────────────────────────────────────────

struct Config {
    interval:      Duration,
    temp_min:      f64,
    temp_max:      f64,
    humidity_min:  f64,
    humidity_max:  f64,
    temp_step:     f64,
    humidity_step: f64,
}

impl Config {
    fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        let interval_secs = env_f64("GENERATOR_INTERVAL", 2.0);
        Self {
            interval:      Duration::from_secs_f64(interval_secs),
            temp_min:      env_f64("TEMP_MIN",      15.0),
            temp_max:      env_f64("TEMP_MAX",      35.0),
            humidity_min:  env_f64("HUMIDITY_MIN",  20.0),
            humidity_max:  env_f64("HUMIDITY_MAX",  90.0),
            temp_step:     env_f64("TEMP_STEP",      0.5),
            humidity_step: env_f64("HUMIDITY_STEP",  1.0),
        }
    }

    fn temp_mid(&self)     -> f64 { (self.temp_min     + self.temp_max)     / 2.0 }
    fn humidity_mid(&self) -> f64 { (self.humidity_min + self.humidity_max) / 2.0 }
}

// ─── Generowanie wartości ─────────────────────────────────────────────────────

fn walk(prev: f64, step: f64, min: f64, max: f64) -> f64 {
    let delta = rand::rng().random_range(-step..=step);
    (prev + delta).clamp(min, max)
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cfg    = Config::from_env();
    let stdout = io::stdout();

    let mut temp     = cfg.temp_mid();
    let mut humidity = cfg.humidity_mid();

    eprintln!(
        "generator: interval={:.1}s  temp={:.1}–{:.1}°C  humidity={:.1}–{:.1}%",
        cfg.interval.as_secs_f64(),
        cfg.temp_min, cfg.temp_max,
        cfg.humidity_min, cfg.humidity_max,
    );

    loop {
        temp     = round1(walk(temp,     cfg.temp_step,     cfg.temp_min,     cfg.temp_max));
        humidity = round1(walk(humidity, cfg.humidity_step, cfg.humidity_min, cfg.humidity_max));

        {
            let mut out = stdout.lock();
            if writeln!(out, "{temp} {humidity}").is_err() {
                std::process::exit(0);
            }
        }

        tokio::time::sleep(cfg.interval).await;
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

// ─── Testy ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_stays_in_range() {
        for _ in 0..1000 {
            let t = walk(25.0, 0.5, 15.0, 35.0);
            let h = walk(55.0, 1.0, 20.0, 90.0);
            assert!(t >= 15.0 && t <= 35.0, "temp {t} poza zakresem");
            assert!(h >= 20.0 && h <= 90.0, "humidity {h} poza zakresem");
        }
    }

    #[test]
    fn walk_clamps_at_min() {
        let t = walk(15.0, 0.5, 15.0, 35.0);
        assert!(t >= 15.0);
    }

    #[test]
    fn walk_clamps_at_max() {
        let t = walk(35.0, 0.5, 15.0, 35.0);
        assert!(t <= 35.0);
    }

    #[test]
    fn round1_works() {
        assert_eq!(round1(22.45), 22.5);
        assert_eq!(round1(22.44), 22.4);
    }
}
