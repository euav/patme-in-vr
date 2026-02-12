use bluest::{Adapter, Characteristic, Device, Uuid};
use futures_lite::stream::StreamExt;
use log::{debug, error, info};
use std::{error::Error, time::Duration};
use tokio::sync::watch::Receiver;

const SERVICE_UUID: Uuid = Uuid::from_u128(0xab96bc38_67c5_44a5_94bf_3146bf493198u128);
const HAPTICS_UUID: Uuid = Uuid::from_u128(0x5db0ca73_7963_492d_8a9c_40bb6b84c2f0u128);
const RECONNECT_BACKOFF: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, PartialEq)]
pub struct HapticState {
    pub force: Vec<f32>,
}

impl HapticState {
    pub fn new(size: usize) -> Self {
        Self {
            force: vec![0f32; size],
        }
    }

    pub fn as_le_bytes(&self) -> Vec<u8> {
        self.force.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
}

pub struct Client {
    adapter: Adapter,
    receiver: Receiver<HapticState>,
    connection: Option<(Device, Characteristic)>,
}

impl Client {
    pub async fn new(receiver: Receiver<HapticState>) -> std::io::Result<Client> {
        let adapter = Adapter::default().await.expect("No BLE adapter");
        adapter
            .wait_available()
            .await
            .expect("Cannot acquire adapter rights");
        Ok(Self {
            adapter,
            receiver,
            connection: None,
        })
    }

    pub async fn start(&mut self) {
        loop {
            if self.receiver.changed().await.is_err() {
                info!("BLE receiver closed; stopping BLE task");
                return;
            }

            let state = self.receiver.borrow().clone();
            debug!("ble> {:?}", state);

            if !self.is_connected().await {
                self.connection = self.find_device().await.ok();
                if !self.is_connected().await {
                    tokio::time::sleep(RECONNECT_BACKOFF).await;
                    continue;
                }
            }

            if let Err(e) = self.write_state(state).await {
                error!("BLE write failed: {}", e);
                self.connection = None;
                tokio::time::sleep(RECONNECT_BACKOFF).await;
            }
        }
    }

    async fn is_connected(&self) -> bool {
        match &self.connection {
            Some((device, _)) => device.is_connected().await,
            None => false,
        }
    }

    async fn write_state(&self, state: HapticState) -> Result<(), Box<dyn Error>> {
        let (_, characteristic) = self
            .connection
            .as_ref()
            .ok_or("Not connected to haptics characteristic")?;
        let bytes = state.as_le_bytes();
        characteristic.write(&bytes).await?;
        Ok(())
    }

    async fn find_device(&self) -> Result<(Device, Characteristic), Box<dyn Error>> {
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

        let characteristic = device
            .services()
            .await?
            .iter()
            .find(|x| x.uuid() == SERVICE_UUID)
            .ok_or("PatMe-in-VR service not found on the device")?
            .characteristics()
            .await?
            .iter()
            .find(|x| x.uuid() == HAPTICS_UUID)
            .ok_or("PatMe-in-VR characteristic not found on the device")?
            .clone();

        Ok((device, characteristic))
    }
}
