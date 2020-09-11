/// a simple circuit breaker that can retry a task until it fails a certain number of times within
/// a given time period
use chrono::{DateTime, Duration, Utc};

pub struct CircuitBreaker {
    count: usize,
    duration: Duration,
    trips: Vec<DateTime<Utc>>,
}

impl CircuitBreaker {
    /// construct a new circuit breaker which trips if more than `count` retries happen in
    /// a period of less than `duration`
    pub fn new(count: usize, duration: Duration) -> CircuitBreaker {
        CircuitBreaker {
            count,
            duration,
            trips: vec![],
        }
    }

    /// check if the circuit breaker will allow us to retry an operation.
    /// This method compares the current time to the oldest time in the
    /// last `count` retries - if this time is less than `duration` then it returns `false`
    /// otherwise the current time is recorded and it returns `true`
    pub fn retry(&mut self) -> bool {
        if self.trips.len() < self.count {
            // haven't retried enough times to trip yet
            self.trips.push(Utc::now());
            true
        } else {
            debug_assert!(self.trips.len() == self.count);

            let now = Utc::now();
            if now - self.trips[0] < self.duration {
                // earliest trip was too recent - trip the breaker
                false
            } else {
                // remove the oldest retry and record the current one, breaker remains closed
                self.trips.remove(0);
                self.trips.push(now);
                true
            }
        }
    }
}
