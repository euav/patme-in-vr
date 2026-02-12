use log::{debug, error, info};
use rosc::{OscBundle, OscMessage, OscPacket, OscType};
use tokio::{net::UdpSocket, sync::watch::Sender};

const OSC_HAPTIC_PREFIX: &str = "/avatar/parameters/haptic_";

#[derive(Clone, Debug, Default)]
pub struct HapticParam(pub usize, pub f32);

impl HapticParam {
    fn try_from_message(msg: OscMessage) -> Option<Self> {
        let value = match msg.args.first()? {
            OscType::Float(value) => *value,
            _ => return None,
        };

        if msg.addr.starts_with(OSC_HAPTIC_PREFIX) && value.is_finite() {
            return match msg.addr[OSC_HAPTIC_PREFIX.len()..]
                .to_string()
                .parse::<usize>()
            {
                Ok(index) => Some(Self(index, value.clamp(0.0, 1.0))),
                _ => None,
            };
        }

        None
    }
}

fn handle_packet(packet: OscPacket, sender: &Sender<HapticParam>) {
    match packet {
        OscPacket::Message(message) => {
            if let Some(param) = HapticParam::try_from_message(message) {
                debug!("osc> {:?}", param);
                let _ = sender.send(param);
            }
        }
        OscPacket::Bundle(OscBundle { content, .. }) => {
            for inner in content {
                handle_packet(inner, sender);
            }
        }
    }
}

pub struct Server {
    socket: UdpSocket,
    sender: Sender<HapticParam>,
}

impl Server {
    pub async fn new(sender: Sender<HapticParam>, bind_addr: &str) -> std::io::Result<Server> {
        let socket = UdpSocket::bind(bind_addr).await?;
        info!("UDP socket {} has been bound", socket.local_addr()?);
        Ok(Self { socket, sender })
    }

    pub async fn start(&self) {
        let mut buffer = [0u8; rosc::decoder::MTU];
        loop {
            let size = match self.socket.recv_from(&mut buffer).await {
                Ok((size, _)) => size,
                Err(e) => {
                    error!("UDP receiving error: {}", e);
                    continue;
                }
            };

            let packet = match rosc::decoder::decode_udp(&buffer[..size]) {
                Ok((_, packet)) => packet,
                Err(e) => {
                    error!("OSC decoding error: {}", e);
                    continue;
                }
            };

            handle_packet(packet, &self.sender);
        }
    }
}
