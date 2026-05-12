use std::net::IpAddr;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
    extract::ConnectInfo,
};
use ipnet::IpNet;
use std::net::SocketAddr;

/// Sprawdza czy adres IP mieści się w którymkolwiek z podanych sieci CIDR.
fn matches_any(ip: IpAddr, nets: &[IpNet]) -> bool {
    nets.iter().any(|net| net.contains(&ip))
}

/// Middleware filtrowania IP.
///
/// Logika:
///   - whitelist niepusta → przepuść tylko adresy z listy, pozostałe 403
///   - whitelist pusta, blacklist niepusta → zablokuj adresy z listy, pozostałe przepuść
///   - obie puste → przepuść wszystkich
pub async fn ip_filter_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    whitelist: axum::extract::Extension<Vec<IpNet>>,
    blacklist: axum::extract::Extension<Vec<IpNet>>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let ip = addr.ip();

    // Normalizuj IPv4-mapped IPv6 (::ffff:1.2.3.4 → 1.2.3.4)
    let ip = match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        other => other,
    };

    if !whitelist.is_empty() {
        if !matches_any(ip, &whitelist) {
            return (
                StatusCode::FORBIDDEN,
                format!("403 Forbidden — {ip} nie jest na liście dozwolonych adresów"),
            ).into_response();
        }
    } else if !blacklist.is_empty() && matches_any(ip, &blacklist) {
        return (
            StatusCode::FORBIDDEN,
            format!("403 Forbidden — {ip} jest zablokowany"),
        ).into_response();
    }

    next.run(request).await
}

/// Parsuje listę sieci CIDR z łańcucha rozdzielonego przecinkami.
/// Obsługuje zarówno CIDR (192.168.1.0/24) jak i pojedyncze adresy (10.0.0.1 → /32 lub /128).
/// Nieprawidłowe wpisy są logowane na stderr i pomijane.
pub fn parse_cidr_list(raw: &str) -> Vec<IpNet> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            // Spróbuj jako CIDR
            if let Ok(net) = entry.parse::<IpNet>() {
                return Some(net);
            }
            // Spróbuj jako pojedynczy adres IP → zamień na /32 lub /128
            if let Ok(ip) = entry.parse::<IpAddr>() {
                return Some(IpNet::from(ip));
            }
            eprintln!("⚠️  Nieprawidłowy wpis IP/CIDR (pomijam): '{entry}'");
            None
        })
        .collect()
}

// ─── Testy ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_ipv4() {
        let nets = parse_cidr_list("192.168.1.10");
        assert_eq!(nets.len(), 1);
        assert!(nets[0].contains(&"192.168.1.10".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn parses_cidr_ipv4() {
        let nets = parse_cidr_list("10.0.0.0/24");
        assert_eq!(nets.len(), 1);
        assert!(nets[0].contains(&"10.0.0.100".parse::<IpAddr>().unwrap()));
        assert!(!nets[0].contains(&"10.0.1.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn parses_multiple() {
        let nets = parse_cidr_list("192.168.1.10, 10.0.0.0/24, 172.16.0.0/12");
        assert_eq!(nets.len(), 3);
    }

    #[test]
    fn ignores_empty_entries() {
        let nets = parse_cidr_list("192.168.1.1,,, 10.0.0.1");
        assert_eq!(nets.len(), 2);
    }

    #[test]
    fn ignores_invalid() {
        let nets = parse_cidr_list("not-an-ip, 192.168.1.1");
        assert_eq!(nets.len(), 1);
    }

    #[test]
    fn whitelist_logic() {
        let whitelist = parse_cidr_list("192.168.1.0/24");
        let allowed = "192.168.1.50".parse::<IpAddr>().unwrap();
        let denied  = "10.0.0.1".parse::<IpAddr>().unwrap();
        assert!(matches_any(allowed, &whitelist));
        assert!(!matches_any(denied, &whitelist));
    }

    #[test]
    fn blacklist_logic() {
        let blacklist = parse_cidr_list("10.0.0.0/8");
        let blocked   = "10.5.5.5".parse::<IpAddr>().unwrap();
        let allowed   = "192.168.1.1".parse::<IpAddr>().unwrap();
        assert!(matches_any(blocked, &blacklist));
        assert!(!matches_any(allowed, &blacklist));
    }

    #[test]
    fn parses_ipv6_cidr() {
        let nets = parse_cidr_list("2001:db8::/32");
        assert_eq!(nets.len(), 1);
        assert!(nets[0].contains(&"2001:db8::1".parse::<IpAddr>().unwrap()));
    }
}
