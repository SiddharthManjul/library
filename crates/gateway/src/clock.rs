use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use time::OffsetDateTime;

/// Time as the gateway sees it, so tests can move it.
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// A clock that only moves when told to.
#[derive(Debug)]
pub struct ManualClock(AtomicI64);

impl ManualClock {
    pub fn new(start: OffsetDateTime) -> Self {
        Self(AtomicI64::new(
            i64::try_from(start.unix_timestamp_nanos() / 1_000_000).unwrap_or(0),
        ))
    }

    pub fn advance_ms(&self, ms: i64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(0));
        Self(AtomicI64::new(now))
    }
}

impl Clock for ManualClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp_nanos(
            i128::from(self.0.load(Ordering::SeqCst)) * 1_000_000,
        )
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
    }
}
