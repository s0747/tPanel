use crate::config::Config;
use maud::{html, Markup, PreEscaped};

/// Dwa kafelki z bieżącymi wartościami temperatury i wilgotności
pub fn metric_cards() -> Markup {
    html! {
        div.metrics-row {
            div.metric-card.temp {
                div.metric-label { "Temperatura" }
                div.metric-value id="val-temp" {
                    span id="disp-temp" { "--" }
                    span.metric-unit { "°C" }
                }
                div.metric-trend id="trend-temp" { "oczekiwanie na dane..." }
            }
            div.metric-card.humid {
                div.metric-label { "Wilgotność" }
                div.metric-value id="val-humid" {
                    span id="disp-humid" { "--" }
                    span.metric-unit { "%" }
                }
                div.metric-trend id="trend-humid" { "oczekiwanie na dane..." }
            }
        }
    }
}

/// Karta z wykresem uPlot
pub fn chart_card(cfg: &Config) -> Markup {
    html! {
        div.chart-card {
            div.chart-header {
                span.chart-title {
                    "Historia pomiarów (ostatnie "
                    (cfg.chart_max_points)
                    " próbek)"
                }
                div.legend {
                    div.legend-item {
                        div.legend-dot style="background: var(--color-temp)" {}
                        span { "Temp. °C" }
                    }
                    div.legend-item {
                        div.legend-dot style="background: var(--color-humid)" {}
                        span { "Wilg. %" }
                    }
                }
            }
            div #"chart-container" {}
        }
    }
}

/// Mały inline <script> przekazujący dane z serwera do dashboard.js,
/// a następnie ładuje sam dashboard.js jako zewnętrzny plik statyczny.
///
/// Wzorzec: serwer wstrzykuje tylko dane (JSON), logika jest w pliku .js.
pub fn scripts(data_json: &str, cfg: &Config) -> Markup {
    // Budujemy tylko dwa przypisania; reszta logiki jest w static/dashboard.js
    let inline = format!(
        "window.INIT_DATA  = {};\nwindow.MAX_POINTS = {};\nwindow.SSE_URL = '/{}/sse';",
        data_json, cfg.chart_max_points, cfg.uuid,
    );

    html! {
        // 1. Dane z serwera – muszą być dostępne zanim dashboard.js się wykona
        script { (PreEscaped(inline)) }

        // 2. Logika aplikacji – plik statyczny serwowany przez tower-http
        script src={ "/" (cfg.uuid) "/static/dashboard.js" } defer {}
    }
}
