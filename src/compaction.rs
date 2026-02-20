use crate::ble::HapticState;
use crate::osc::PatMeParam;
use std::collections::VecDeque;
use tokio::sync::watch::Sender;
use tokio::sync::{mpsc::UnboundedReceiver, watch::Receiver};
use tokio::time::{Duration, Instant, MissedTickBehavior};

#[derive(Clone)]
struct DecayFilter {
    history: VecDeque<(Instant, f32)>,
}

impl DecayFilter {
    const HISTORY_DEPTH: usize = 6;
    const PERIOD_SECS: f32 = 0.3;

    fn new() -> Self {
        let now = Instant::now();
        Self {
            history: (0..Self::HISTORY_DEPTH).map(|_| (now, 0f32)).collect(),
        }
    }

    fn append(&mut self, sample: f32) {
        let _ = self.history.pop_back();
        self.history.push_front((Instant::now(), sample));
    }

    fn estimate(&self) -> f32 {
        let now = Instant::now();
        let (weighted_sum, weight_sum) = self.history.iter().rev().fold(
            (0f32, 0f32),
            |(acc_value, acc_weight), (instant, value)| {
                let age = (now - *instant).as_secs_f32();
                let weight = 1f32 - age / Self::PERIOD_SECS;

                if weight > 0f32 {
                    (acc_value + value * weight, acc_weight + weight)
                } else {
                    (acc_value, acc_weight)
                }
            },
        );

        if weight_sum == 0f32 {
            0f32
        } else {
            weighted_sum / weight_sum
        }
    }
}

impl HapticState {
    fn from_decay_filters(filters: &[DecayFilter], scale: f32) -> Self {
        Self {
            force: filters.iter().map(|f| f.estimate() * scale).collect(),
        }
    }
}

pub struct Compactor {
    osc_params: UnboundedReceiver<PatMeParam>,
    states: Sender<HapticState>,
    filters: Vec<DecayFilter>,
    max_intensity: f32,
    send_interval: Duration,
    max_intensity_rx: Option<Receiver<u8>>,
}

impl Compactor {
    pub fn new(
        osc_params: UnboundedReceiver<PatMeParam>,
        states: Sender<HapticState>,
        haptics_count: usize,
        send_interval: Duration,
        max_intensity_rx: Option<Receiver<u8>>,
    ) -> Self {
        let filters = vec![DecayFilter::new(); haptics_count];

        Self {
            osc_params,
            states,
            filters,
            max_intensity: 0.7f32,
            send_interval,
            max_intensity_rx,
        }
    }

    fn current_max_intensity(&mut self) -> f32 {
        if let Some(ref rx) = self.max_intensity_rx {
            match rx.has_changed() {
                Ok(true) => {
                    self.max_intensity = (*rx.borrow() as f32 / 100.0).clamp(0.0, 1.0);
                }
                _ => {}
            }
        }
        self.max_intensity
    }

    pub async fn start(mut self) {
        let mut tick = tokio::time::interval(self.send_interval);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                Some(patme_param) = self.osc_params.recv() => {
                    match patme_param {
                        PatMeParam::Touch(index, value) => {
                            if index < self.filters.len() {
                                self.filters[index].append(value);
                            }
                        }
                        PatMeParam::Intensity(value) => {
                            self.max_intensity = value;
                        }
                    }
                }
                _ = tick.tick() => {
                    let scale = self.current_max_intensity();
                    let _ = self.states.send(HapticState::from_decay_filters(&self.filters, scale));
                }
            }
        }
    }
}
