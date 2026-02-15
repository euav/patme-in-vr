use crate::ble::HapticState;
use crate::osc::HapticParam;
use std::collections::VecDeque;
use tokio::sync::{mpsc::UnboundedReceiver, watch::Sender};
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
    fn from_decay_filters(filters: &[DecayFilter]) -> Self {
        Self {
            force: filters.iter().map(|filter| filter.estimate()).collect(),
        }
    }
}

pub struct Compactor {
    params: UnboundedReceiver<HapticParam>,
    states: Sender<HapticState>,
    filters: Vec<DecayFilter>,
    send_interval: Duration,
}

impl Compactor {
    pub fn new(
        params: UnboundedReceiver<HapticParam>,
        states: Sender<HapticState>,
        haptics_count: usize,
        send_interval: Duration,
    ) -> Self {
        let filters = vec![DecayFilter::new(); haptics_count];

        Self {
            params,
            states,
            filters,
            send_interval,
        }
    }

    pub async fn start(mut self) {
        let mut tick = tokio::time::interval(self.send_interval);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                Some(HapticParam(index, value)) = self.params.recv() => {
                    if index <  self.filters.len() {
                        self.filters[index].append(value);
                    }
                }
                _ = tick.tick() => {
                    let _ = self.states.send(HapticState::from_decay_filters(&self.filters));
                }
            }
        }
    }
}
