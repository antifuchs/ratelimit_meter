use {Decider, Decision, Limiter, ErrorKind, Result};

use std::time::{Instant, Duration};
use std::cmp;

#[derive(Debug)]
/// Implements the virtual scheduling description of the Generic Cell
/// Rate Algorithm, attributed to ITU-T in recommendation I.371
/// Traffic control and congestion control in B-ISDN; from
/// [Wikipedia](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm).
///
/// # Example
/// In this example, we construct a rate-limiter with the GCR
/// algorithm that can accomodate 20 requests per second. This
/// translates to the GCRA parameters τ=1s, T=50ms.
///
/// ```
/// # use ratelimit_meter::{Limiter, Decider, GCRA, Decision};
/// # use std::time::{Instant, Duration};
/// let mut limiter = Limiter::new().capacity(20).weight(1).build::<GCRA>().unwrap();
/// let now = Instant::now();
/// let ms = Duration::from_millis(1);
/// assert_eq!(Decision::Yes, limiter.test_and_update(now)); // the first cell is free
/// for i in 0..20 {
///     // Spam a lot:
///     assert_eq!(Decision::Yes, limiter.test_and_update(now), "at {}", i);
/// }
/// // We have exceeded the bucket capacity:
/// assert!(!limiter.test_and_update(now).is_compliant());
///
/// // After a sufficient time period, cells are allowed again:
/// assert_eq!(Decision::Yes, limiter.test_and_update(now + ms*50));
pub struct GCRA {
    // The "weight" of a single packet in units of time.
    t: Duration,

    // The "capacity" of the bucket.
    tau: Duration,

    // The theoretical arrival time of the next packet.
    tat: Option<Instant>,
}

impl Decider for GCRA {
    /// In GCRA, negative decisions come with the time at which the
    /// next cell was expected to arrive; client code of GCRA can use
    /// this to decide what to do with the non-conforming cell.
    type T = Instant;

    fn test_and_update(&mut self, t0: Instant) -> Decision<Instant> {
        let tat = self.tat.unwrap_or(t0);
        if t0 < tat - self.tau {
            return Decision::No(tat)
        }
        self.tat = Some(cmp::max(tat, t0) + self.t);
        Decision::Yes
    }

    fn build_with(l: &Limiter) -> Result<Self> {
        let capacity = l.capacity.ok_or(ErrorKind::CapacityRequired)?;
        let weight = l.weight.ok_or(ErrorKind::WeightRequired)?;
        if l.time_unit <= Duration::new(0, 0) {
            return Err(ErrorKind::InvalidTimeUnit(l.time_unit).into());
        }
        Ok(GCRA{
            t: (l.time_unit / capacity) * weight,
            tau: l.time_unit,
            tat: None,
        })
    }
}
