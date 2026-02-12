use crate::ble::HapticState;
use crate::osc::HapticParam;
use std::collections::VecDeque;
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::{Duration, Instant, MissedTickBehavior};

const FILTER_DEPTH: usize = 6;

#[derive(Clone)]
struct DecayFilter {
    history: VecDeque<(Instant, f32)>,
}

impl DecayFilter {
    const PERIOD_SECS: f32 = 0.3;

    fn new(depth: usize) -> Self {
        let now = Instant::now();
        Self {
            history: (0..depth).map(|_| (now, 0f32)).collect(),
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
    params: Receiver<HapticParam>,
    states: Sender<HapticState>,
    filters: Vec<DecayFilter>,
    send_interval: Duration,
}

impl Compactor {
    pub fn new(
        params: Receiver<HapticParam>,
        states: Sender<HapticState>,
        haptics_count: usize,
        send_interval: Duration,
    ) -> Self {
        let filters = vec![DecayFilter::new(FILTER_DEPTH); haptics_count];

        Self {
            params,
            states,
            filters,
            send_interval,
        }
    }

    pub async fn start(mut self) {
        let haptics_count = self.filters.len();

        let mut tick = tokio::time::interval(self.send_interval);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = self.params.changed() => {
                    let HapticParam(index, value) = *self.params.borrow();
                    if index < haptics_count {
                        self.filters[index].append(value);
                    }
                }
                _ = tick.tick() => {
                    let state = HapticState::from_decay_filters(&self.filters);
                    let _ = self.states.send(state);
                }
            }
        }
    }
}
