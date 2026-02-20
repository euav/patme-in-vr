use iced::widget::{button, column, progress_bar, row, slider, text};
use iced::{Alignment, Element, Size, Subscription, Task};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Updates sent from the bridge to the GUI.
#[derive(Debug, Clone)]
pub enum GuiUpdate {
    Ble(bool),
    Osc(bool),
    Haptics(Vec<f32>),
    Battery(u8),
    Intensity(u8),
}

#[derive(Clone, Debug)]
pub struct AppFlags {
    pub config: crate::Config,
    pub gui_tx: tokio::sync::mpsc::UnboundedSender<GuiUpdate>,
    pub cmd_tx: tokio::sync::mpsc::UnboundedSender<crate::BridgeCommand>,
    pub cmd_rx: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<crate::BridgeCommand>>>>,
    pub osc_port: u16,
    pub haptics_count: usize,
    pub send_interval_ms: u64,
}

#[derive(Clone, Debug)]
pub struct App {
    pub osc_port: u16,
    pub haptics_count: usize,
    pub send_interval_ms: u64,
    pub osc_listening: bool,
    pub ble_connected: bool,
    pub battery_pct: Option<u8>,
    pub haptics: Vec<f32>,
    pub max_intensity: u8,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<crate::BridgeCommand>,
    rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<GuiUpdate>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Vibrate(usize),
    MaxIntensityChanged(f32), // slider value, converted to u8 in update
    Quit,
    BridgeExited,
}

impl App {
    pub fn new(
        flags: AppFlags,
        rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<GuiUpdate>>>,
    ) -> (Self, Task<Message>) {
        (
            Self {
                osc_port: flags.osc_port,
                haptics_count: flags.haptics_count,
                send_interval_ms: flags.send_interval_ms,
                osc_listening: false,
                ble_connected: false,
                battery_pct: None,
                haptics: Vec::new(),
                max_intensity: 70,
                cmd_tx: flags.cmd_tx,
                rx,
            },
            Task::perform(
                async move {
                    let cmd_rx = flags.cmd_rx.lock().unwrap().take();
                    crate::bridge(flags.config, cmd_rx, Some(flags.gui_tx)).await
                },
                |_: ()| Message::BridgeExited,
            ),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => self.apply_pending_updates(),
            Message::Vibrate(idx) => {
                let _ = self.cmd_tx.send(crate::BridgeCommand::TestPulse(idx));
            }
            Message::MaxIntensityChanged(v) => {
                self.max_intensity = (v.clamp(0.0, 100.0).round() as u8).min(100);
                let _ = self
                    .cmd_tx
                    .send(crate::BridgeCommand::SetMaxIntensity(self.max_intensity));
            }
            Message::Quit => return iced::exit(),
            Message::BridgeExited => {}
        }
        Task::none()
    }

    fn apply_pending_updates(&mut self) {
        let mut rx = self.rx.lock().unwrap();
        while let Ok(update) = rx.try_recv() {
            match update {
                GuiUpdate::Ble(connected) => self.ble_connected = connected,
                GuiUpdate::Osc(listening) => self.osc_listening = listening,
                GuiUpdate::Haptics(strength) => self.haptics = strength,
                GuiUpdate::Battery(pct) => self.battery_pct = Some(pct),
                GuiUpdate::Intensity(intensity) => self.max_intensity = intensity,
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(150)).map(|_| Message::Tick)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let osc_status = match self.osc_listening {
            true => format!("OSC listening on port {}", self.osc_port),
            false => "OSC: not started".to_string(),
        };

        let ble_status = match self.ble_connected {
            true => "BLE: Connected",
            false => "BLE: Searching...",
        };

        let battery_status = match self.battery_pct {
            Some(pct) => format!("Battery: {}%", pct),
            None => "Battery: —".to_string(),
        };

        let status = column![
            text("Status").size(16),
            text(osc_status).size(12),
            text(ble_status).size(12),
            text(battery_status).size(12),
        ]
        .spacing(4)
        .align_x(Alignment::Start);

        let config = column![
            text("Configuration").size(16),
            text(format!("Haptics: {}", self.haptics_count)).size(12),
            text(format!("Send interval: {} ms", self.send_interval_ms)).size(12),
        ]
        .spacing(4)
        .align_x(Alignment::Start);

        let max_intensity_row = row![
            text("Max intensity").width(100),
            slider(
                0.0..=100.0,
                self.max_intensity as f32,
                Message::MaxIntensityChanged
            ),
            text(format!("{}%", self.max_intensity)).width(40),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut haptics = column![text("Haptic values").size(16)].spacing(8);
        for (idx, &val) in self.haptics.iter().enumerate() {
            haptics = haptics.push(
                row![
                    text(format!("Haptic {}", idx)).width(80),
                    progress_bar(0.0..=1.0, val),
                    text(format!("{:.0}%", val * 100.0)).width(40),
                    button("Test").on_press(Message::Vibrate(idx))
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        if self.haptics.is_empty() {
            haptics = haptics.push(text("(no data yet)").size(12));
        }

        let quit_btn = button("Quit").on_press(Message::Quit);

        column![
            text("PatMe in VR, OSC to BLE bridge").size(20),
            row![status, config].spacing(32),
            max_intensity_row,
            haptics,
            quit_btn,
        ]
        .spacing(16)
        .padding(24)
        .align_x(Alignment::Start)
        .into()
    }
}

pub fn run_app(config: crate::Config) -> iced::Result {
    let (gui_tx, gui_rx) = tokio::sync::mpsc::unbounded_channel();
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();

    let rx_arc = Arc::new(Mutex::new(gui_rx));
    let cmd_rx_cell = Arc::new(Mutex::new(Some(cmd_rx)));

    let osc_port = config.osc_port;
    let haptics_count = config.haptics_count;
    let send_interval_ms = config.send_interval_ms;
    let flags = AppFlags {
        config,
        gui_tx,
        cmd_tx,
        cmd_rx: cmd_rx_cell,
        osc_port,
        haptics_count,
        send_interval_ms,
    };

    iced::application(
        move || App::new(flags.clone(), rx_arc.clone()),
        App::update,
        App::view,
    )
    .subscription(App::subscription)
    .title("PatMe in VR")
    .window_size(Size::new(500f32, 400f32))
    .run()
}
