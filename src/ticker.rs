use std::time::{Instant, Duration};
use derive_more::AsRef;

pub struct Ticker {
    start_time: Instant,
    now: Instant,
    pub target_delta: TickRate,
}

impl Ticker {
    pub fn new(target_delta: TickRate) -> Self {
        let time = Instant::now();
        Self {
            start_time: time,
            now: time,
            target_delta
        }
    }

    pub fn tick(&mut self) {
        if let Some(dur) = (self.now + self.target_delta.0).checked_duration_since(Instant::now()) {
            std::thread::sleep(dur);
        }
        self.now = Instant::now();
    }
}

#[derive(AsRef)]
pub struct TickRate(Duration);

impl TickRate {
    pub fn ticks_per_second(tps: u32) -> Self {
        Self(Duration::from_secs_f64(1f64 / (tps as f64)))
    }

    pub fn tick_duration(dur: Duration) -> Self {
        Self(dur)
    }
}