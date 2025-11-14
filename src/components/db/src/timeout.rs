use std::{num::NonZero, time::Instant};

pub trait ShouldUnblock {
    #[inline]
    fn should_unblock(&mut self) -> bool;
}

pub struct TimeoutNever;
pub struct TimeoutAt(pub Instant);
pub struct TimeoutAfterSteps(pub NonZero<usize>);

impl ShouldUnblock for TimeoutNever {
    fn should_unblock(&mut self) -> bool {
        false
    }
}

impl ShouldUnblock for TimeoutAt {
    fn should_unblock(&mut self) -> bool {
        Instant::now() >= self.0
    }
}

impl ShouldUnblock for TimeoutAfterSteps {
    fn should_unblock(&mut self) -> bool {
        if let Some(decremented) = NonZero::new(self.0.get() - 1) {
            self.0 = decremented;
            false
        } else {
            true
        }
    }
}
