# 🌡️ KlimaMonitor — Weather Dashboard

Aplikacja SPA do monitorowania temperatury i wilgotności w czasie rzeczywistym.

## Stack technologiczny

| Warstwa | Technologia |
|---|---|
| Backend HTTP | **Axum** 0.7 |
| Streaming danych | **SSE** (Server-Sent Events) |
| HTML templating | **Maud** 0.26 |
| UI framework | **Pico.css** 2 |
| Wykresy | **uPlot** 1.6.31 |
| Reaktywność | **Datastar** 1.0.0-RC.11 |

## Uruchomienie

### Wymagania
- Rust (stabilny, 1.75+): https://rustup.rs

### Kroki

```bash
# 1. Klonuj / wypakuj projekt
cd weather-dashboard

# 2. Zbuduj i uruchom
cargo run --release

# 3. Otwórz przeglądarkę
open http://localhost:3000
```

Aplikacja domyślnie nasłuchuje na **porcie 3000**.

## Architektura

```
┌─────────────────────────────────────────────────────┐
│                   Przeglądarka                      │
│                                                     │
│  Pico.css  ←  Maud HTML  ←  Axum /                 │
│                                                     │
│  uPlot chart ← JavaScript ← SSE /sse               │
│  (dual-axis)     EventSource    ↑                   │
└─────────────────────────────────────────────────────┘
                                  │
                          Axum SSE handler
                          (co 2 sekundy)
                          losowe delta:
                          temp  ±0.5°C
                          humid ±1.0%
```

## Funkcje

- 📡 **Real-time SSE** — dane co 2 sekundy bez pollingu
- 📊 **Dual-axis uPlot** — temperatura (lewa oś) + wilgotność (prawa oś)
- 🔄 **Auto-reconnect** — SSE automatycznie wznawia połączenie
- 📜 **Historia** — 30 próbek startowych, max 60 na wykresie
- 📱 **Responsive** — działa na mobile i desktop
- 🌙 **Dark theme** — elegancki ciemny interfejs

## Struktura projektu

```
weather-dashboard/
├── Cargo.toml          # Zależności Rust
├── README.md           # Ten plik
└── src/
    └── main.rs         # Cały serwer + templaty HTML
```

## Rozszerzenia

Aby podłączyć prawdziwe czujniki (np. DHT22 przez GPIO lub MQTT):

1. Zastąp `generate_point()` odczytem z prawdziwego sensora
2. Dodaj kanał `tokio::sync::broadcast` dla wielu klientów SSE
3. Dodaj opcjonalnie bazę danych (SQLite via `sqlx`) dla trwałej historii
