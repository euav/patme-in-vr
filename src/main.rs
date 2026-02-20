mod ble;
mod compaction;
mod gui;
mod osc;

use clap::Parser;
use log::info;
use tokio::signal;
use tokio::sync::{mpsc, watch};
use tokio::time::Duration;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "PatMe-in-VR",
    version,
    about = "VRChat's OSC to BLE haptics bridge"
)]
pub struct Config {
    #[arg(long, env = "PATME_OSC_PORT", default_value_t = 9001)]
    osc_port: u16,

    #[arg(long, env = "PATME_HAPTICS_COUNT", default_value_t = 2)]
    haptics_count: usize,

    #[arg(long, env = "PATME_SEND_INTERVAL_MS", default_value_t = 20)]
    send_interval_ms: u64,

    #[arg(long)]
    headless: bool,
}

/// Commands from the GUI to the bridge.
#[derive(Debug, Clone)]
pub enum BridgeCommand {
    TestPulse(usize),
    SetMaxIntensity(u8),
}

/// If GUI is used, spawns tasks that forward BLE status and haptic state to the GUI.
/// Returns the sender for BLE status.
fn setup_gui_forwarding(
    gui_tx: Option<mpsc::UnboundedSender<gui::GuiUpdate>>,
    ble_tx: &watch::Sender<ble::HapticState>,
) -> Option<mpsc::UnboundedSender<ble::Status>> {
    let gui_tx = gui_tx?;
    let (status_tx, mut status_rx) = mpsc::unbounded_channel::<ble::Status>();

    // Forward BLE status (connection, battery) to GUI
    let tx = gui_tx.clone();
    tokio::spawn(async move {
        while let Some(s) = status_rx.recv().await {
            match s {
                ble::Status::Connection(connected) => _ = tx.send(gui::GuiUpdate::Ble(connected)),
                ble::Status::Battery(pct) => _ = tx.send(gui::GuiUpdate::Battery(pct)),
            }
        }
    });

    // Forward haptic state to GUI (for progress bars)
    let mut haptics_rx = ble_tx.subscribe();
    let tx = gui_tx.clone();
    tokio::spawn(async move {
        while haptics_rx.changed().await.is_ok() {
            let _ = tx.send(gui::GuiUpdate::Haptics(haptics_rx.borrow().force.clone()));
        }
    });

    Some(status_tx)
}

/// Spawns a task that handles GUI commands: test pulse and max intensity updates.
/// Returns the compactor's max_intensity receiver when commands are enabled.
fn spawn_command_handler(
    mut cmd_rx: mpsc::UnboundedReceiver<BridgeCommand>,
    ble_tx: watch::Sender<ble::HapticState>,
) -> watch::Receiver<u8> {
    let (max_intensity_tx, max_intensity_rx) = watch::channel(100u8);

    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                BridgeCommand::TestPulse(idx) => {
                    let state_rx = ble_tx.subscribe();
                    let current = state_rx.borrow().clone();
                    if idx < current.force.len() {
                        let mut pulse = current.clone();
                        pulse.force[idx] = 1.0;
                        let _ = ble_tx.send(pulse);
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        let _ = ble_tx.send(current);
                    }
                }
                BridgeCommand::SetMaxIntensity(v) => {
                    let _ = max_intensity_tx.send(v.min(100));
                }
            }
        }
    });

    max_intensity_rx
}

pub async fn bridge(
    config: Config,
    cmd_rx: Option<mpsc::UnboundedReceiver<BridgeCommand>>,
    gui_tx: Option<mpsc::UnboundedSender<gui::GuiUpdate>>,
) {
    let (osc_tx, osc_rx) = mpsc::unbounded_channel();
    let (ble_tx, ble_rx) = watch::channel(ble::HapticState::new(config.haptics_count));

    let status_tx = setup_gui_forwarding(gui_tx.clone(), &ble_tx);

    let max_intensity_rx = cmd_rx.map(|rx| spawn_command_handler(rx, ble_tx.clone()));

    let osc = osc::Server::new(config.osc_port, osc_tx)
        .await
        .expect("Failed to start OSC server");
    if let Some(ref tx) = gui_tx {
        let _ = tx.send(gui::GuiUpdate::Osc(true));
    }

    let mut ble = ble::Client::new(ble_rx, status_tx)
        .await
        .expect("Failed to create BLE client");
    let compactor = compaction::Compactor::new(
        osc_rx,
        ble_tx,
        config.haptics_count,
        Duration::from_millis(config.send_interval_ms),
        max_intensity_rx,
    );

    tokio::select! {
        _ = osc.start() => info!("OSC server task finished"),
        _ = ble.start() => info!("BLE client task finished"),
        _ = compactor.start() => info!("Compaction task finished"),
        _ = signal::ctrl_c() => info!("Ctrl+C received, shutting down"),
    }
}

fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = Config::parse();

    if config.headless {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(bridge(config, None, None));
    } else {
        gui::run_app(config).expect("Cannot start GUI app, try headless mode");
    }

    Ok(())
}
