mod broker;
mod config;
mod forward;

use config::Config;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Inicjalizacja logowania — poziom kontrolowany przez RUST_LOG
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env();

    info!("🔧 mqtt-broker konfiguracja:");
    info!("   MQTT_TCP_ADDR              = {}", cfg.mqtt_tcp_addr);
    info!(
        "   MQTT_TLS_ADDR              = {}",
        if cfg.tls_enabled() {
            &cfg.mqtt_tls_addr
        } else {
            "(wyłączony)"
        }
    );
    info!(
        "   MQTT_QUIC_ADDR             = {}",
        if cfg.quic_enabled() {
            &cfg.mqtt_quic_addr
        } else {
            "(wyłączony)"
        }
    );
    info!("   BRIDGE_TOPIC               = {}", cfg.bridge_topic);
    info!("   BRIDGE_TARGET_URL          = {}", cfg.bridge_target_url);
    info!(
        "   BRIDGE_SENSOR_ID_FROM_TOPIC= {}",
        cfg.bridge_sensor_id_from_topic
    );
    info!("   BRIDGE_CLIENT_ID           = {}", cfg.bridge_client_id);

    // ── Kanał broker → bridge (bufor 256 wiadomości) ──────────────────────────
    let (forward_tx, forward_rx) = tokio::sync::mpsc::channel(256);

    // ── Uruchom broker ────────────────────────────────────────────────────────
    let mut mqtt_broker = match broker::start_broker(&cfg, forward_tx).await {
        Ok(b) => b,
        Err(e) => {
            error!("❌ Błąd uruchamiania brokera: {e}");
            std::process::exit(1);
        }
    };

    // ── Uruchom bridge jako osobny task ───────────────────────────────────────
    let bridge_target = cfg.bridge_target_url.clone();
    let bridge_flag = cfg.bridge_sensor_id_from_topic;
    let bridge_default = cfg.bridge_default_sensor_id.clone();

    tokio::spawn(async move {
        forward::run_bridge(forward_rx, bridge_target, bridge_flag, bridge_default).await;
    });

    // ── Bridge subskrybuje lokalny broker ─────────────────────────────────────
    // Subskrybent wewnętrzny — łączy się z własnym brokerem przez TCP
    let bridge_topic = cfg.bridge_topic.clone();
    let bridge_addr = cfg.mqtt_tcp_addr.clone();
    let bridge_client_id = cfg.bridge_client_id.clone();

    tokio::spawn(async move {
        // Krótkie opóźnienie — broker musi być gotowy przed połączeniem klienta
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let client = mqtt5::MqttClient::new(&bridge_client_id);

        let url = format!("mqtt://{bridge_addr}");

        loop {
            match client.connect(&url).await {
                Ok(_) => {
                    info!("[bridge-sub] Połączono z brokerem, subskrybuję '{bridge_topic}'");

                    // Subskrypcja — callback tylko loguje; realne przetwarzanie
                    // odbywa się w BrokerEventHandler (SensorPublishHandler)
                    // który ma bezpośredni dostęp do kanału forward_tx.
                    // Subskrypcja bridge klienta jest potrzebna żeby broker
                    // dostarczał wiadomości do handlera.
                    if let Err(e) = client
                        .subscribe(&bridge_topic, |msg| {
                            tracing::debug!(
                                "[bridge-sub] temat={} payload={} bajtów",
                                msg.topic,
                                msg.payload.len()
                            );
                        })
                        .await
                    {
                        tracing::warn!("[bridge-sub] Błąd subskrypcji: {e}");
                    }
                    break;
                }
                Err(e) => {
                    tracing::warn!("[bridge-sub] Błąd połączenia: {e} — retry za 2s");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    });

    // ── Główna pętla brokera — blokuje do sygnału shutdown ───────────────────
    info!("\n✅ mqtt-broker gotowy");
    info!("   📡 TCP:  mqtt://{}", cfg.mqtt_tcp_addr);
    if cfg.tls_enabled() {
        info!("   🔒 TLS:  mqtts://{}", cfg.mqtt_tls_addr);
    }
    if cfg.quic_enabled() {
        info!("   ⚡ QUIC: quic://{}", cfg.mqtt_quic_addr);
    }
    info!("   🌉 Bridge → {}", cfg.bridge_target_url);

    tokio::select! {
        result = mqtt_broker.run() => {
            if let Err(e) = result {
                error!("❌ Broker zakończył się błędem: {e}");
                std::process::exit(1);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("⏹  Otrzymano Ctrl+C — zamykanie...");
            if let Err(e) = mqtt_broker.shutdown().await {
                error!("Błąd podczas zamykania: {e}");
            }
        }
    }
}
