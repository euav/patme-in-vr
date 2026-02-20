use bluest::{Adapter, Characteristic, Device, Uuid, btuuid};
use futures_lite::stream::StreamExt;
use log::{error, info, warn};
use std::{error::Error, time::Duration};
use tokio::sync::watch::Receiver;

const SERVICE_UUID: Uuid = Uuid::from_u128(0xab96bc38_67c5_44a5_94bf_3146bf493198u128);
const HAPTIC_UUID: Uuid = Uuid::from_u128(0x5db0ca73_7963_492d_8a9c_40bb6b84c2f0u128);
const RECONNECT_BACKOFF: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, PartialEq)]
pub struct HapticState {
    pub strength: Vec<f32>,
}

impl HapticState {
    pub fn new(size: usize) -> Self {
        Self {
            strength: vec![0f32; size],
        }
    }

    pub fn as_le_bytes(&self) -> Vec<u8> {
        self.strength.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
}

#[derive(Clone, Debug)]
pub enum Status {
    Connection(bool),
    Battery(u8),
}

struct Connection {
    device: Device,
    haptic: Characteristic,
    battery: Option<Characteristic>,
}

pub struct Client {
    adapter: Adapter,
    receiver: Receiver<HapticState>,
    status_tx: Option<tokio::sync::mpsc::UnboundedSender<Status>>,
    connection: Option<Connection>,
}

impl Client {
    pub async fn new(
        receiver: Receiver<HapticState>,
        status_tx: Option<tokio::sync::mpsc::UnboundedSender<Status>>,
    ) -> std::io::Result<Client> {
        let adapter = Adapter::default().await.expect("No BLE adapter");
        adapter
            .wait_available()
            .await
            .expect("Cannot acquire adapter rights");
        Ok(Self {
            adapter,
            receiver,
            status_tx,
            connection: None,
        })
    }

    pub async fn start(&mut self) {
        loop {
            if self.receiver.changed().await.is_err() {
                return;
            }
            let state = self.receiver.borrow().clone();

            if !self.is_connected().await {
                match self.find_device().await {
                    Ok(conn) => {
                        self.notify(Status::Connection(true));
                        self.connection = Some(conn);
                    }
                    Err(_) => {
                        tokio::time::sleep(RECONNECT_BACKOFF).await;
                        continue;
                    }
                }
            }

            if let Err(e) = self.write_state(&state).await {
                error!("BLE write failed: {}", e);
                self.notify(Status::Connection(false));
                self.connection = None;
                tokio::time::sleep(RECONNECT_BACKOFF).await;
                continue;
            }

            if let Some(pct) = self.read_battery().await {
                self.notify(Status::Battery(pct));
            }
        }
    }

    fn notify(&self, status: Status) {
        if let Some(tx) = &self.status_tx {
            let _ = tx.send(status);
        }
    }

    async fn read_battery(&self) -> Option<u8> {
        let conn = self.connection.as_ref()?;
        let bat = conn.battery.as_ref()?;
        let level = bat.read().await.ok()?;
        level.first().copied()
    }

    async fn is_connected(&self) -> bool {
        match &self.connection {
            Some(Connection { device, .. }) => device.is_connected().await,
            None => false,
        }
    }

    async fn write_state(&self, state: &HapticState) -> Result<(), Box<dyn Error + Send + Sync>> {
        let Connection { haptic, .. } = self
            .connection
            .as_ref()
            .ok_or("Not connected to haptic characteristic")?;
        haptic.write(&state.as_le_bytes()).await?;
        Ok(())
    }

    async fn find_device(&self) -> Result<Connection, Box<dyn Error + Send + Sync>> {
        info!("Searching for PatMe-in-VR device...");
        let device = self
            .adapter
            .discover_devices(&[SERVICE_UUID])
            .await?
            .next()
            .await
            .ok_or("PatMe-in-VR device not found")??;

        self.adapter.connect_device(&device).await?;
        info!("Connected to device {}", device.id());

        let haptic = device
            .services()
            .await?
            .iter()
            .find(|x| x.uuid() == SERVICE_UUID)
            .ok_or("Haptic service not found on the device")?
            .characteristics()
            .await?
            .iter()
            .find(|x| x.uuid() == HAPTIC_UUID)
            .ok_or("Haptic characteristic not found on the device")?
            .clone();

        let battery = match Self::find_battery(&device).await {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("{:?}", e);
                None
            }
        };

        Ok(Connection {
            device,
            haptic,
            battery,
        })
    }

    async fn find_battery(device: &Device) -> Result<Characteristic, Box<dyn Error + Send + Sync>> {
        Ok(device
            .services()
            .await?
            .iter()
            .find(|x| x.uuid() == btuuid::services::BATTERY)
            .ok_or("Battery service not found on the device")?
            .characteristics()
            .await?
            .iter()
            .find(|x| x.uuid() == btuuid::characteristics::BATTERY_LEVEL)
            .ok_or("Battery level characteristic not found on the device")?
            .clone())
    }
}
