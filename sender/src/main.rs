use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};

// ─── Konfiguracja ─────────────────────────────────────────────────────────────

struct Config {
    url: String,
    sensor_id: String,
}

impl Config {
    fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        Self {
            url: env_str("SENDER_URL", "http://localhost:3000/default/"),
            sensor_id: env_str("SENDER_SENSOR_ID", "sender-01"),
        }
    }
}

// ─── Payload JSON ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Reading {
    sensor_id: String,
    ts: f64,
    temp: f64,
    humidity: f64,
}

// ─── Parsowanie linii ze stdin ────────────────────────────────────────────────

/// Parsuje linię "TEMP HUMIDITY" z pipe od generatora.
/// Zwraca None i loguje ostrzeżenie przy błędzie.
fn parse_line(line: &str) -> Option<(f64, f64)> {
    let mut parts = line.split_whitespace();

    let temp = parts
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())?;

    let humidity = parts
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())?;

    if parts.next().is_some() {
        eprintln!("sender: ostrzeżenie — nadmiarowe pola w linii: '{line}'");
        return None;
    }

    Some((temp, humidity))
}

// ─── HTTP ─────────────────────────────────────────────────────────────────────

async fn send_reading(client: &reqwest::Client, cfg: &Config, reading: &Reading) {
    match client.post(&cfg.url).json(reading).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                eprintln!(
                    "[{:.0}] ✓ {}  temp={:.1}°C  humidity={:.1}%",
                    reading.ts, status, reading.temp, reading.humidity,
                );
            } else {
                let body = resp.text().await.unwrap_or_default();
                eprintln!("[{:.0}] ✗ {}  {}", reading.ts, status, body.trim());
            }
        }
        Err(e) => eprintln!("[{:.0}] ✗ HTTP error: {e}", now_secs()),
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cfg = Config::from_env();
    let client = reqwest::Client::new();

    eprintln!("sender: url={}  sensor_id={}", cfg.url, cfg.sensor_id);
    eprintln!("sender: czekam na dane ze stdin (generator | sender)...");

    // Czytaj stdin linia po linii asynchronicznie
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let line = line.trim().to_owned();
                if line.is_empty() {
                    continue;
                }

                match parse_line(&line) {
                    Some((temp, humidity)) => {
                        let reading = Reading {
                            sensor_id: cfg.sensor_id.clone(),
                            ts: now_secs(),
                            temp,
                            humidity,
                        };
                        send_reading(&client, &cfg, &reading).await;
                    }
                    None => {
                        eprintln!("sender: pominięto nieprawidłową linię: '{line}'");
                    }
                }
            }
            Ok(None) => {
                // EOF — generator zamknął pipe lub zakończył działanie
                eprintln!("sender: stdin zamknięty (EOF) — kończę");
                break;
            }
            Err(e) => {
                eprintln!("sender: błąd odczytu stdin: {e}");
                std::process::exit(1);
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn env_str(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

// ─── Testy ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::parse_line;

    #[test]
    fn parses_valid() {
        let (t, h) = parse_line("22.4 58.1").unwrap();
        assert_eq!(t, 22.4);
        assert_eq!(h, 58.1);
    }

    #[test]
    fn parses_integers() {
        let (t, h) = parse_line("22 58").unwrap();
        assert_eq!(t, 22.0);
        assert_eq!(h, 58.0);
    }

    #[test]
    fn parses_with_extra_whitespace() {
        let (t, h) = parse_line("  22.4   58.1  ").unwrap();
        assert_eq!(t, 22.4);
        assert_eq!(h, 58.1);
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_line("").is_none());
    }

    #[test]
    fn rejects_one_value() {
        assert!(parse_line("22.4").is_none());
    }

    #[test]
    fn rejects_three_values() {
        assert!(parse_line("22.4 58.1 99.0").is_none());
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(parse_line("abc def").is_none());
    }

    #[test]
    fn rejects_nan() {
        assert!(parse_line("NaN 58.1").is_none());
    }

    #[test]
    fn rejects_inf() {
        assert!(parse_line("inf 58.1").is_none());
    }
}
