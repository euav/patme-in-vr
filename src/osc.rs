use log::{debug, error, info};
use rosc::{OscBundle, OscMessage, OscPacket, OscType};
use tokio::{net::UdpSocket, sync::mpsc::UnboundedSender};

const OSC_PATME_PREFIX: &str = "/avatar/parameters/PatMe/";

#[derive(Clone, Debug)]
pub enum PatMeParam {
    Intensity(f32),
    Touch(usize, f32),
}

impl PatMeParam {
    fn try_from_message(msg: OscMessage) -> Option<Self> {
        let value = match msg.args.first()? {
            OscType::Float(value) => value.clamp(0.0, 1.0),
            _ => return None,
        };

        if msg.addr.starts_with(OSC_PATME_PREFIX) && value.is_finite() {
            let param_name = &msg.addr[OSC_PATME_PREFIX.len()..];
            match param_name {
                "Intensity" => Some(Self::Intensity(value)),
                _ => match param_name.parse::<usize>() {
                    Ok(index) => Some(Self::Touch(index, value)),
                    _ => None,
                }
            }
        } else {
            None
        }
    }
}

pub struct Server {
    socket: UdpSocket,
    sender: UnboundedSender<PatMeParam>,
}

impl Server {
    pub async fn new(port: u16, sender: UnboundedSender<PatMeParam>) -> std::io::Result<Server> {
        let socket = UdpSocket::bind(("0.0.0.0", port)).await?;
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

            self.handle_packet(packet);
        }
    }

    fn handle_packet(&self, packet: OscPacket) {
        match packet {
            OscPacket::Message(message) => {
                if let Some(param) = PatMeParam::try_from_message(message) {
                    debug!("osc> {:?}", param);
                    let _ = self.sender.send(param);
                }
            }
            OscPacket::Bundle(OscBundle { content, .. }) => {
                for inner in content {
                    self.handle_packet(inner);
                }
            }
        }
    }
}
