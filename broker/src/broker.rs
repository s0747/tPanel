use mqtt5::broker::config::{QuicConfig, TlsConfig};
use mqtt5::broker::{
    BrokerConfig, BrokerEventHandler, ClientPublishEvent, MqttBroker, PublishAction,
};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{info, warn};

use crate::config::Config;

/// Handler zdarzeń brokera — wywoływany przy każdym PUBLISH od czujnika.
/// Przekazuje zdarzenie do kanału bridge'a i zwraca PublishAction::Continue
/// żeby broker normalnie dostarczył wiadomość subskrybentom.
pub struct SensorPublishHandler {
    pub forward_tx: tokio::sync::mpsc::Sender<ClientPublishEvent>,
}

impl BrokerEventHandler for SensorPublishHandler {
    fn on_client_publish<'a>(
        &'a self,
        event: ClientPublishEvent,
    ) -> Pin<Box<dyn Future<Output = PublishAction> + Send + 'a>> {
        Box::pin(async move {
            if let Err(e) = self.forward_tx.send(event).await {
                warn!("Kanał bridge'a pełny lub zamknięty: {e}");
            }
            // Continue — broker nadal dostarcza wiadomość subskrybentom
            PublishAction::Continue
        })
    }
}

/// Buduje i uruchamia broker MQTT na podstawie konfiguracji.
pub async fn start_broker(
    cfg: &Config,
    forward_tx: tokio::sync::mpsc::Sender<ClientPublishEvent>,
) -> Result<MqttBroker, Box<dyn std::error::Error + Send + Sync>> {
    let tcp_addr: SocketAddr = cfg.mqtt_tcp_addr.parse()?;

    let handler = Arc::new(SensorPublishHandler { forward_tx });

    let mut broker_cfg = BrokerConfig::default()
        .with_bind_address(tcp_addr)
        .with_event_handler(handler);

    // ── TLS (opcjonalne) ──────────────────────────────────────────────────────
    if cfg.tls_enabled() {
        let tls_addr: SocketAddr = cfg.mqtt_tls_addr.parse()?;
        let tls = TlsConfig::new(
            cfg.mqtt_tls_cert.clone().into(),
            cfg.mqtt_tls_key.clone().into(),
        )
        .with_bind_address(tls_addr);

        broker_cfg = broker_cfg.with_tls(tls);
        info!("🔒 TLS włączony: mqtts://{}", cfg.mqtt_tls_addr);
    }

    // ── QUIC (opcjonalne) ─────────────────────────────────────────────────────
    if cfg.quic_enabled() {
        let quic_addr: SocketAddr = cfg.mqtt_quic_addr.parse()?;
        let quic = QuicConfig::new(
            cfg.mqtt_tls_cert.clone().into(),
            cfg.mqtt_tls_key.clone().into(),
        )
        .with_bind_address(quic_addr);

        broker_cfg = broker_cfg.with_quic(quic);
        info!("⚡ QUIC włączony: quic://{}", cfg.mqtt_quic_addr);
    }

    let broker = MqttBroker::with_config(broker_cfg).await?;

    info!("📡 Broker MQTT uruchomiony: mqtt://{}", cfg.mqtt_tcp_addr);

    Ok(broker)
}
