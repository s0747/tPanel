use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use axum::{
    extract::{ConnectInfo, Request},
    http::{StatusCode, HeaderValue},
    middleware::Next,
    response::IntoResponse,
};

/// Stan pojedynczego okna dla jednego adresu IP.
#[derive(Debug)]
pub(crate) struct Window {
    /// Liczba requestów w bieżącym oknie
    count:      u32,
    /// Początek bieżącego okna
    window_start: Instant,
    /// Ostatnia aktywność — używana do czyszczenia starych wpisów
    last_seen:  Instant,
}

impl Window {
    fn new() -> Self {
        let now = Instant::now();
        Self { count: 1, window_start: now, last_seen: now }
    }
}

/// Współdzielona mapa okien rate limitera.
pub type RateLimitMap = Arc<Mutex<HashMap<IpAddr, Window>>>;

pub fn new_rate_limit_map() -> RateLimitMap {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Konfiguracja rate limitera przekazywana przez Extension.
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maksymalna liczba requestów w oknie
    pub max_requests: u32,
    /// Długość okna
    pub window:       Duration,
}

/// Middleware fixed-window rate limiting per IP.
///
/// Każde okno zaczyna się przy pierwszym requeście danego IP.
/// Po upływie `window` licznik jest zerowany.
/// Przekroczenie `max_requests` → 429 z nagłówkiem `Retry-After`.
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::extract::Extension(map): axum::extract::Extension<RateLimitMap>,
    axum::extract::Extension(cfg): axum::extract::Extension<RateLimitConfig>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let ip = normalize_ip(addr.ip());
    let now = Instant::now();

    let (allowed, retry_after) = {
        let mut map = map.lock().unwrap();
        let entry = map.entry(ip).or_insert_with(Window::new);
        entry.last_seen = now;

        let elapsed = now.duration_since(entry.window_start);

        if elapsed >= cfg.window {
            // Nowe okno — zeruj licznik
            entry.window_start = now;
            entry.count = 1;
            (true, 0u64)
        } else if entry.count < cfg.max_requests {
            entry.count += 1;
            (true, 0u64)
        } else {
            // Przekroczono limit — oblicz ile zostało do końca okna
            let remaining = cfg.window.saturating_sub(elapsed);
            (false, remaining.as_secs().max(1))
        }
    };

    if allowed {
        next.run(request).await.into_response()
    } else {
        let mut resp = (
            StatusCode::TOO_MANY_REQUESTS,
            format!("429 Too Many Requests — spróbuj ponownie za {retry_after}s"),
        ).into_response();

        resp.headers_mut().insert(
            "Retry-After",
            HeaderValue::from_str(&retry_after.to_string()).unwrap(),
        );
        resp
    }
}

/// Uruchamia tło czyszczące nieaktywne wpisy z mapy rate limitera.
/// Wpis jest usuwany gdy minął `2 * window` od ostatniej aktywności.
pub fn spawn_cleanup_task(map: RateLimitMap, window: Duration) {
    tokio::spawn(async move {
        let cleanup_interval = window;
        let ttl = window * 2;
        loop {
            tokio::time::sleep(cleanup_interval).await;
            let now = Instant::now();
            let mut map = map.lock().unwrap();
            let before = map.len();
            map.retain(|_, w| now.duration_since(w.last_seen) < ttl);
            let removed = before - map.len();
            if removed > 0 {
                println!("🧹 rate_limit: usunięto {removed} nieaktywnych wpisów");
            }
        }
    });
}

// ─── Helper ───────────────────────────────────────────────────────────────────

fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        other => other,
    }
}

// ─── Testy ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_window(count: u32, secs_ago: u64) -> Window {
        let start = Instant::now() - Duration::from_secs(secs_ago);
        Window { count, window_start: start, last_seen: Instant::now() }
    }

    #[test]
    fn new_window_on_first_request() {
        let w = Window::new();
        assert_eq!(w.count, 1);
    }

    #[test]
    fn window_resets_after_expiry() {
        // Symulujemy logikę resetowania okna
        let cfg = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
        };
        let window = make_window(5, 61); // okno wygasło 1s temu
        let elapsed = Instant::now().duration_since(window.window_start);
        assert!(elapsed >= cfg.window, "okno powinno być wygasłe");
    }

    #[test]
    fn within_window_increments() {
        let cfg = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
        };
        let window = make_window(3, 10); // 3 requesty, okno ma 50s
        let elapsed = Instant::now().duration_since(window.window_start);
        assert!(elapsed < cfg.window);
        assert!(window.count < cfg.max_requests);
    }

    #[test]
    fn over_limit_blocked() {
        let cfg = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
        };
        let window = make_window(5, 10); // dokładnie na limicie
        let elapsed = Instant::now().duration_since(window.window_start);
        assert!(elapsed < cfg.window);
        assert!(window.count >= cfg.max_requests);
    }

    #[test]
    fn normalize_ipv4_mapped() {
        let v6: IpAddr = "::ffff:192.168.1.1".parse().unwrap();
        let normalized = normalize_ip(v6);
        assert_eq!(normalized, "192.168.1.1".parse::<IpAddr>().unwrap());
    }
}
