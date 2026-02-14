mod ble;
mod compaction;
mod osc;

use crate::ble::{Client, HapticState};
use crate::compaction::Compactor;
use crate::osc::{HapticParam, Server};

use clap::Parser;
use tokio::signal;
use tokio::sync::watch;
use tokio::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "PatMe-in-VR",
    version,
    about = "VRChat's OSC to BLE haptics bridge"
)]
struct Config {
    #[arg(long, env = "PATME_OSC_PORT", default_value_t = 9001)]
    osc_port: u16,

    #[arg(long, env = "PATME_HAPTICS_COUNT", default_value_t = 2)]
    haptics_count: usize,

    #[arg(long, env = "PATME_SEND_INTERVAL_MS", default_value_t = 20)]
    send_interval_ms: u64,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = Config::parse();

    let (tx, params) = watch::channel(HapticParam::default());
    let (states, rx) = watch::channel(HapticState::new(config.haptics_count));

    let osc = Server::new(config.osc_port, tx)
        .await
        .expect("Failed to start OSC server");
    let mut ble = Client::new(rx).await.expect("Failed to create BLE client");
    let compactor = Compactor::new(
        params,
        states,
        config.haptics_count,
        Duration::from_millis(config.send_interval_ms),
    );

    tokio::select! {
        _ = osc.start() => log::info!("OSC server task finished"),
        _ = ble.start() => log::info!("BLE client task finished"),
        _ = compactor.start() => log::info!("Compaction task finished"),
        _ = signal::ctrl_c() => log::info!("Ctrl+C received, shutting down"),
    }

    Ok(())
}
