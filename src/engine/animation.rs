use std::collections::VecDeque;
use std::time::Duration;

use scarlet::color::RGBColor;
use scarlet::colorpoint::ColorPoint;

pub type Interpolation = fn(f64) -> f64;

pub trait Lerp {
    fn lerp(a: Self, b: Self, i: f64) -> Self;
}

pub struct Keyframe<V> {
    pub duration: Duration,
    pub value: V,

    pub interpolation: Interpolation,
}

impl<V, I> From<(Duration, I, Interpolation)> for Keyframe<V>
    where I: Into<V> {
    fn from((duration, value, interpolation): (Duration, I, Interpolation)) -> Self {
        return Keyframe {
            duration,
            value: value.into(),
            interpolation,
        };
    }
}

impl<V, I> From<(Duration, I)> for Keyframe<V>
    where I: Into<V> {
    fn from((duration, value): (Duration, I)) -> Self {
        return Keyframe {
            duration,
            value: value.into(),
            interpolation: interpolations::end,
        };
    }
}

impl<V, I> From<(f64, I, Interpolation)> for Keyframe<V>
    where I: Into<V> {
    fn from((duration, value, interpolation): (f64, I, Interpolation)) -> Self {
        return Keyframe {
            duration: Duration::from_secs_f64(duration),
            value: value.into(),
            interpolation,
        };
    }
}

impl<V, I> From<(f64, I)> for Keyframe<V>
    where I: Into<V> {
    fn from((duration, value): (f64, I)) -> Self {
        return Keyframe {
            duration: Duration::from_secs_f64(duration),
            value: value.into(),
            interpolation: interpolations::end,
        };
    }
}

impl<V> Keyframe<V> {
    pub fn new(duration: Duration, value: V, interpolation: Interpolation) -> Self {
        return Self {
            duration,
            value,
            interpolation,
        };
    }
}


#[macro_export]
macro_rules! keyframe {
    ($duration:literal => { $value:expr } @ $interpolation:ident) => {
        (($duration, $value, $crate::engine::animation::interpolations::$interpolation as $crate::engine::animation::Interpolation).into())
    };
}

#[macro_export]
macro_rules! keyframes {
    (@expr $e:expr) => { $e };

    (@rec [ ] -> ($($body:tt)*)) => {
        $crate::keyframes!(@expr [ $($body)* ])
    };

    (@rec [ $duration:literal => { $value:expr } @ $interpolation:ident, $($r:tt)* ] -> ($($body:tt)*)) => {
        $crate::keyframes!(@rec [ $($r)* ] -> ($($body)* $crate::keyframe!($duration => { $value } @ $interpolation),))
    };

    (@rec [ $duration:literal => $value:literal @ $interpolation:ident, $($r:tt)* ] -> ($($body:tt)*)) => {
        $crate::keyframes!(@rec [ $($r)* ] -> ($($body)* $crate::keyframe!($duration => { $value } @ $interpolation),))
    };

    (@rec [ $duration:literal => { $value:expr }, $($r:tt)* ] -> ($($body:tt)*)) => {
        $crate::keyframes!(@rec [ $($r)* ] -> ($($body)* $crate::keyframe!($duration => { $value } @ end),))
    };

    (@rec [ $duration:literal => $value:literal, $($r:tt)* ] -> ($($body:tt)*)) => {
        $crate::keyframes!(@rec [ $($r)* ] -> ($($body)* $crate::keyframe!($duration => { $value } @ end),))
    };

    ($($frame:tt)*) => {
        $crate::keyframes!(@rec [ $($frame)* ] -> ())
    };
}

enum State<V> {
    Running {
        // Sequence of keyframes in this animation
        timeline: VecDeque<Keyframe<V>>,

        // Time already spent in the current keyframe
        elapsed: Duration,
    },

    Idle,
}

pub struct Animated<V> {
    state: State<V>,
    value: V,
}


impl<V> Animated<V>
    where
        V: Lerp + Copy,
{
    pub fn idle(value: impl Into<V>) -> Self {
        return Self {
            state: State::Idle,
            value: value.into(),
        };
    }

    pub fn set(&mut self, value: V) {
        self.state = State::Idle;
        self.value = value;
    }

    pub fn animate(&mut self, keyframes: impl IntoIterator<Item=Keyframe<V>>) {
        match self.state {
            State::Running { ref mut timeline, .. } => {
                timeline.extend(keyframes);
            }
            State::Idle => {
                self.state = State::Running {
                    elapsed: Duration::ZERO,
                    timeline: keyframes.into_iter().collect(),
                };
            }
        }
    }

    pub fn set_and_animate(&mut self, value: V, keyframes: impl IntoIterator<Item=Keyframe<V>>) {
        self.set(value);
        self.animate(keyframes);
    }

    pub fn update(&mut self, duration: Duration) {
        if let State::Running { ref mut elapsed, ref mut timeline } = self.state {
            // Continue in time
            *elapsed += duration;

            while let Some(keyframe) = timeline.front() {
                if *elapsed >= keyframe.duration {
                    // The current keyframe has completed
                    self.value = keyframe.value;

                    *elapsed -= keyframe.duration;
                    timeline.pop_front();
                } else {
                    // Stay in the current frame
                    break;
                }
            }

            if timeline.is_empty() {
                // Timeline depleted - idling
                self.state = State::Idle;
            }
        }
    }

    pub fn value(&self) -> V {
        match &self.state {
            State::Running { elapsed, timeline } => {
                if let Some(keyframe) = timeline.front() {
                    let delta = elapsed.as_secs_f64() / keyframe.duration.as_secs_f64();
                    let delta = (keyframe.interpolation)(delta);
                    return V::lerp(self.value, keyframe.value, delta);
                } else {
                    return self.value;
                }
            }
            State::Idle => {
                return self.value;
            }
        };
    }

    pub fn is_idle(&self) -> bool {
        return match self.state {
            State::Idle => true,
            State::Running { .. } => false,
        };
    }
}

impl Lerp for u8 {
    fn lerp(a: Self, b: Self, i: f64) -> Self {
        if a < b {
            return a + ((b - a) as f64 * i) as u8;
        } else {
            return a - ((a - b) as f64 * i) as u8;
        }
    }
}

impl Lerp for f32 {
    fn lerp(a: Self, b: Self, i: f64) -> Self {
        return a + ((b - a) as f64 * i) as f32;
    }
}

impl Lerp for RGBColor {
    fn lerp(a: Self, b: Self, i: f64) -> Self {
        return RGBColor::weighted_midpoint(b, a, i);
    }
}

pub mod interpolations {
    pub use easings::*;

    pub fn end(i: f64) -> f64 {
        if i < 1.0 { 0.0 } else { 1.0 }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_idle_value() {
        let animation: Animated<u8> = Animated::idle(7);
        assert!(animation.is_idle());
        assert_eq!(animation.value(), 7);
    }

    #[test]
    fn test_animation_value() {
        let mut animation: Animated<u8> = Animated::idle(7);
        assert_eq!(animation.value(), 7);

        animation.animate(keyframes!(
            1.0 => 9 @ linear,
            2.0 => 8 @ end,
            3.0 => 5 @ linear,
        ));

        assert!(!animation.is_idle());

        assert_eq!(animation.value(), 7);

        // 1. Frame: 1.0 => 9 @ linear
        animation.update(Duration::from_millis(500));
        assert_eq!(animation.value(), 8);
        animation.update(Duration::from_millis(500));
        assert_eq!(animation.value(), 9);

        // 2. Frame: 2.0 => 8 @ end
        animation.update(Duration::from_millis(1500));
        assert_eq!(animation.value(), 9);
        animation.update(Duration::from_millis(400));
        assert_eq!(animation.value(), 9);
        animation.update(Duration::from_millis(100));
        assert_eq!(animation.value(), 8);

        // 3. Frame: 3.0 => 4 @ linear
        animation.update(Duration::from_millis(1000));
        assert_eq!(animation.value(), 7);
        animation.update(Duration::from_millis(1000));
        assert_eq!(animation.value(), 6);
        animation.update(Duration::from_millis(1000));
        assert_eq!(animation.value(), 5);

        assert!(animation.is_idle());
    }
}