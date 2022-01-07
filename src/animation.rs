use std::time::Duration;

pub trait Lerp {
    fn lerp(a: Self, b: Self, p: f32) -> Self;
}

pub struct Animated<V> {
    state: State<V>,
    speed: f32,
}

enum State<V> {
    Idle(V),
    Running {
        source: V,
        target: V,
        position: f32,
    },
}

impl<V> Animated<V>
    where
        V: Lerp + Copy,
{
    pub fn new(initial: V, speed: f32) -> Self {
        return Self {
            state: State::Idle(initial),
            speed,
        };
    }

    pub fn update(&mut self, duration: Duration) {
        match self.state {
            State::Idle(_) => {},
            State::Running {
                source, target, position,
            } => {
                let position = position + duration.as_secs_f32() * self.speed;
                if position >= 1.0 {
                    self.state = State::Idle(target);
                } else {
                    self.state = State::Running {
                        source,
                        target,
                        position,
                    };
                }
            }
        };
    }

    pub fn set(&mut self, value: V) {
        self.state = State::Running {
            source: self.value(),
            target: value,
            position: 0.0,
        };
    }

    pub fn value(&self) -> V {
        return match self.state {
            State::Idle(value) => value,
            State::Running { source, target, position } => V::lerp(source, target, position),
        };
    }
}

impl Lerp for f32 {
    fn lerp(a: Self, b: Self, p: f32) -> Self {
        if p <= 0.0 {
            return a;
        }

        if p >= 1.0 {
            return b;
        }

        return a + (b - a) * p;
    }
}
