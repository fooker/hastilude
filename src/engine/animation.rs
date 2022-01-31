use std::ops::Deref;
use std::time::Duration;

enum State {
    Running {
        duration: Duration,
        elapsed: Duration,

    },
    Finished,
}

pub struct Animation {
    state: State,
}

impl Animation {
    pub fn idle() -> Self {
        return Self {
            state: State::Finished,
        };
    }

    pub fn new(duration: Duration) -> Self {
        return Self {
            state: State::Running {
                duration,
                elapsed: Duration::ZERO,
            }
        };
    }

    pub fn update(&mut self, t: Duration) -> Option<f32> {
        return match &mut self.state {
            State::Running { duration, elapsed } => {
                *elapsed += t;

                let result = (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0);

                if elapsed > duration {
                    self.state = State::Finished;
                }

                return Some(result);
            }
            State::Finished => None,
        };
    }
}

pub struct Fader {
    speed: Duration,

    current: f32,
    target: f32,
}

impl Fader {
    pub fn new(initial: f32, speed: Duration) -> Self {
        return Self {
            speed,
            current: initial,
            target: initial,
        };
    }

    pub fn update(&mut self, duration: Duration) {
        let x = duration.as_secs_f32() / self.speed.as_secs_f32();
        if self.current < self.target {
                self.current = f32::min(self.target, self.current + x);
        }
        if self.current > self.target {
                self.current = f32::max(self.target, self.current - x);
        }
    }

    pub fn set(&mut self, value: f32) {
        self.target = value;
    }

    pub fn value(&self) -> f32 {
        return self.current;
    }
}

impl Deref for Fader {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        return &self.current;
    }
}
