pub mod dashboard;
pub mod layout;

use maud::{DOCTYPE, html, Markup};
use crate::config::{Config, APP_NAME, APP_VERSION};
use crate::models::DataPoint;

/// Renderuje pełną stronę HTML z danymi historycznymi i konfiguracją
pub fn page(history: &[DataPoint], cfg: &Config) -> Markup {
    let data_json = serde_json::to_string(history).unwrap_or_else(|_| "[]".to_owned());

    html! {
        (DOCTYPE)
        html lang="pl" {
            (layout::head())
            body {
                div.dashboard {
                    (header_section())
                    (dashboard::metric_cards())
                    (dashboard::chart_card(cfg))
                    (footer_section())
                }
                (dashboard::scripts(&data_json, cfg))
            }
        }
    }
}

fn header_section() -> Markup {
    html! {
        header.dash-header {
            div.logo-area {
                h1 { (app_name_spans()) }
                p { "Live environmental sensor data" }
            }
            // Wskaźnik live — ukryty podczas fazy historii, pojawia się po history_end
            div.status-pill #"live-indicator" style="display:none" {
                div.status-dot {}
                span { "LIVE · SSE" }
            }
        }
    }
}

/// Renderuje nazwę z Cargo.toml jako dwa kolorowe spany (pierwsza połowa / druga)
fn app_name_spans() -> Markup {
    let name = APP_NAME.to_uppercase();
    let mid = (name.len() + 1) / 2;
    let (first, second) = name.split_at(mid);
    html! {
        span.accent-temp  { (first)  }
        span.accent-humid { (second) }
    }
}

fn footer_section() -> Markup {
    html! {
        footer.dash-footer {
            // Pasek sterowania: typ wykresu + motyw
            div.controls-bar {
                // Wybór typu wykresu
                div.control-group {
                    label.control-label for="chart-type-select" { "Wykres" }
                    select.control-select #"chart-type-select" onchange="onChartTypeChange(this.value)" {
                        option value="line"    selected { "Liniowy" }
                        option value="area"    { "Obszarowy" }
                        option value="stepped" { "Schodkowy" }
                        option value="bars"    { "Słupkowy" }
                        option value="points"  { "Punktowy" }
                    }
                }

                // Widok — próbki lub zakres czasu
                div.control-group {
                    label.control-label for="view-select" { "Widok" }
                    select.control-select #"view-select" onchange="onViewChange(this.value)" {
                        optgroup label="Próbki" {
                            option value="samples:60"    selected { "60 próbek" }
                            option value="samples:3600"  { "3 600 próbek" }
                            option value="samples:14400" { "14 400 próbek" }
                            option value="samples:43200" { "43 200 próbek" }
                        }
                        optgroup label="Czas" {
                            option value="time:3600"   { "Ostatnia godzina" }
                            option value="time:21600"  { "Ostatnie 6 godzin" }
                            option value="time:86400"  { "Ostatnie 24 godziny" }
                            option value="time:604800" { "Ostatnie 7 dni" }
                        }
                    }
                }

                // Info o aktualizacjach
                div.control-info {
                    span id="update-count" { "0" }
                    " aktualizacji · "
                    (APP_NAME) " v" (APP_VERSION)
                }

                // Przełącznik motywu — piktogramy zamiast toggle
                div.control-group {
                    label.control-label { "Motyw" }
                    div.theme-switcher {
                        button.theme-btn #"btn-light" onclick="setTheme('light')" title="Jasny" { "☀" }
                        button.theme-btn #"btn-dark"  onclick="setTheme('dark')"  title="Ciemny" { "◗" }
                    }
                }
            }
        }
    }
}
