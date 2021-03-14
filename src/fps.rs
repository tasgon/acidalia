use std::time::{Instant, Duration};

pub enum TimingState {
    Draw,
    Update,
}

pub struct FPSCounter {
    draw_ticks: usize,
    update_ticks: usize,
    index: usize,
    num_elements: usize,
    timing_state: TimingState,
    last_time: Instant,
    start_time: Instant,

    update_time: Box<[f32]>,
    draw_time: Box<[f32]>,
}

impl FPSCounter {
    pub fn new() -> Self {
        Self {
            draw_ticks: 0,
            update_ticks: 0,
            index: 0,
            num_elements: 0,
            timing_state: TimingState::Update,
            last_time: Instant::now(),
            start_time: Instant::now(),
            update_time: Vec::with_capacity(1).into_boxed_slice(),
            draw_time: Vec::with_capacity(1).into_boxed_slice(),
        }
    }

    pub(crate) fn set_elements(&mut self, num_elements: usize) {
        self.num_elements = num_elements;
        self.update_time = vec![0f32; num_elements].into_boxed_slice();
        self.draw_time = vec![0f32; num_elements].into_boxed_slice();
        self.draw_ticks = 0;
        self.update_ticks = 0;
        self.start_time = Instant::now();
    }

    #[inline(always)]
    fn get(&mut self) -> &mut Box<[f32]> {
        match self.timing_state {
            TimingState::Draw => &mut self.draw_time,
            TimingState::Update => &mut self.update_time,
        }
    }

    #[inline(always)]
    fn tick(&mut self) {
        match self.timing_state {
            TimingState::Draw => self.draw_ticks += 1,
            TimingState::Update => self.update_ticks += 1,
        }
    }

    #[inline(always)]
    pub(crate) fn start(&mut self, ts: TimingState) {
        self.index = 0;
        self.timing_state = ts;
        self.last_time = Instant::now();
    }

    #[inline(always)]
    pub(crate) fn stop(&mut self) {
        let idx = self.index;
        self.get()[idx] = (Instant::now() - self.last_time).as_secs_f32();
    }

    #[inline(always)]
    pub(crate) fn advance(&mut self) {
        self.stop();
        self.index += 1;
        self.tick();
        self.last_time = Instant::now();
    }

    #[inline(always)]
    pub fn draw_time(&self) -> &Box<[f32]> {
        &self.draw_time
    }

    #[inline(always)]
    pub fn update_time(&self) -> &Box<[f32]> {
        &self.update_time
    }

    #[inline(always)]
    pub fn elements(&self) -> usize {
        self.num_elements
    }

    #[inline(always)]
    pub fn draw_ticks(&self) -> usize {
        self.draw_ticks
    }

    #[inline(always)]
    pub fn running_time(&self) -> Duration {
        Instant::now() - self.start_time
    }
}

