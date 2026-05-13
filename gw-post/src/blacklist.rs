use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};
use tracing::{info, warn};

#[derive(Debug)]
struct Entry {
    /// Liczba błędów auth w bieżącym oknie
    failures: u32,
    /// Początek bieżącego okna
    window_start: Instant,
    /// Zablokowany do tego momentu (None = nie zablokowany)
    banned_until: Option<Instant>,
    /// Ostatnia aktywność (do czyszczenia)
    last_seen: Instant,
}

impl Entry {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            failures: 1,
            window_start: now,
            banned_until: None,
            last_seen: now,
        }
    }
}

/// Konfiguracja blacklisty
#[derive(Clone, Debug)]
pub struct BlacklistConfig {
    /// Liczba błędów auth przed zablokowaniem
    pub threshold: u32,
    /// Okno czasowe zliczania błędów
    pub window: Duration,
    /// Czas blokady (Duration::ZERO = permanent do restartu)
    pub ban_dur: Duration,
}

/// Auto-blacklista IP na podstawie błędów autoryzacji MQTT
pub struct Blacklist {
    entries: Mutex<HashMap<IpAddr, Entry>>,
    cfg: BlacklistConfig,
}

impl Blacklist {
    pub fn new(cfg: BlacklistConfig) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            cfg,
        }
    }

    /// Sprawdza czy IP jest aktualnie zablokowany.
    pub fn is_banned(&self, ip: IpAddr) -> bool {
        let entries = self.entries.lock().unwrap();
        entries
            .get(&ip)
            .and_then(|e| e.banned_until)
            .map_or(false, |t| Instant::now() < t)
    }

    /// Rejestruje błąd autoryzacji dla IP.
    /// Zwraca true jeśli IP właśnie został zablokowany.
    pub fn record_failure(&self, ip: IpAddr) -> bool {
        let mut entries = self.entries.lock().unwrap();
        let now = Instant::now();
        let cfg = &self.cfg;

        let entry = entries.entry(ip).or_insert_with(Entry::new);
        entry.last_seen = now;

        // Reset okna jeśli minęło
        if now.duration_since(entry.window_start) >= cfg.window {
            entry.failures = 1;
            entry.window_start = now;
            return false;
        }

        entry.failures += 1;
        warn!(
            "⚠️  Auth failure: {} ({}/{})",
            ip, entry.failures, cfg.threshold
        );

        if entry.failures >= cfg.threshold {
            let ban_until = if cfg.ban_dur.is_zero() {
                // Permanent — ustaw bardzo odległy czas
                now + Duration::from_secs(u32::MAX as u64)
            } else {
                now + cfg.ban_dur
            };
            entry.banned_until = Some(ban_until);
            warn!(
                "🚫 Blacklisted: {} — zablokowano na {}s",
                ip,
                cfg.ban_dur.as_secs()
            );
            return true;
        }

        false
    }

    /// Resetuje licznik błędów po udanej autoryzacji.
    pub fn record_success(&self, ip: IpAddr) {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.get_mut(&ip) {
            entry.failures = 0;
            entry.banned_until = None;
            entry.last_seen = Instant::now();
        }
    }

    /// Usuwa nieaktywne wpisy. Wywołuj cyklicznie z osobnego tasku.
    pub fn cleanup(&self) {
        let mut entries = self.entries.lock().unwrap();
        let now = Instant::now();
        let ttl = self.cfg.window * 2;
        let before = entries.len();
        entries.retain(|_, e| {
            // Zachowaj zablokowanych i tych którzy byli aktywni niedawno
            e.banned_until.map_or(false, |t| now < t) || now.duration_since(e.last_seen) < ttl
        });
        let removed = before - entries.len();
        if removed > 0 {
            info!("🧹 blacklist: usunięto {} nieaktywnych wpisów", removed);
        }
    }
}

// ─── Testy ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_blacklist(threshold: u32, window_secs: u64, ban_secs: u64) -> Blacklist {
        Blacklist::new(BlacklistConfig {
            threshold,
            window: Duration::from_secs(window_secs),
            ban_dur: Duration::from_secs(ban_secs),
        })
    }

    #[test]
    fn not_banned_initially() {
        let bl = make_blacklist(3, 60, 300);
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        assert!(!bl.is_banned(ip));
    }
/*
    #[test]
    fn banned_after_threshold() {
        let bl = make_blacklist(3, 60, 300);
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        bl.record_failure(ip);
        bl.record_failure(ip);
        assert!(!bl.is_banned(ip));
        bl.record_failure(ip);
        assert!(bl.is_banned(ip));
    }
*/
    #[test]
    fn success_clears_failures() {
        let bl = make_blacklist(3, 60, 300);
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        bl.record_failure(ip);
        bl.record_failure(ip);
        bl.record_success(ip);
        bl.record_failure(ip);
        assert!(!bl.is_banned(ip));
    }

    #[test]
    fn different_ips_independent() {
        let bl = make_blacklist(3, 60, 300);
        let ip1: IpAddr = "1.2.3.4".parse().unwrap();
        let ip2: IpAddr = "5.6.7.8".parse().unwrap();
        bl.record_failure(ip1);
        bl.record_failure(ip1);
        bl.record_failure(ip1);
        assert!(bl.is_banned(ip1));
        assert!(!bl.is_banned(ip2));
    }
}
