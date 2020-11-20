use std::time::{Instant, Duration};

pub struct FPSCounterConfig {
    pub seconds_per_print: Duration,
}


pub struct FPSCounter {
    ticks: u32,
    last_print: Instant,
    config: FPSCounterConfig,
}

impl FPSCounter {
    pub fn new(config: FPSCounterConfig) -> Self {
        Self {
            ticks: 0,
            last_print: Instant::now(),
            config,
        }
    }

    #[inline(always)]
    pub fn tick(&mut self) {
        self.ticks += 1;
    }

    #[inline(always)]
    pub fn print(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_print;
        if delta > self.config.seconds_per_print {
            self.last_print = now;
            println!("FPS: {}", self.ticks as f32 / delta.as_secs_f32());
            self.ticks = 0
        }
    }
}

