//! Token buckets per tenant, per user and per method. A call costs one token from each of its three
//! buckets and is refused, with a retry hint, when any is empty. Nothing is charged for a refused call.

use std::{collections::HashMap, sync::Mutex};

use library_core::{ErrorCode, LibraryError, TenantId, UserId};

use crate::Clock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    /// Tokens a full bucket holds, which is also the largest burst.
    pub burst: f64,
    pub per_second: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub tenant: Limit,
    pub user: Limit,
    pub method: Limit,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            tenant: Limit {
                burst: 300.0,
                per_second: 150.0,
            },
            user: Limit {
                burst: 60.0,
                per_second: 30.0,
            },
            method: Limit {
                burst: 30.0,
                per_second: 10.0,
            },
        }
    }
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    updated_ms: i64,
}

/// Idle buckets are dropped once the map grows past this, so memory stays bounded.
const PRUNE_ABOVE: usize = 50_000;
const IDLE_MS: i64 = 10 * 60 * 1000;

pub struct RateLimiter {
    limits: Limits,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn refill(b: &mut Bucket, now_ms: i64, l: Limit) {
        let elapsed = (now_ms - b.updated_ms).max(0) as f64 / 1000.0;
        b.tokens = (b.tokens + elapsed * l.per_second).min(l.burst);
        b.updated_ms = now_ms;
    }

    pub fn check(
        &self,
        clock: &dyn Clock,
        tenant: TenantId,
        user: UserId,
        method: &str,
    ) -> Result<(), LibraryError> {
        let now_ms = i64::try_from(clock.now().unix_timestamp_nanos() / 1_000_000).unwrap_or(0);
        let keys = [
            (format!("t:{tenant}"), self.limits.tenant),
            (format!("u:{tenant}:{user}"), self.limits.user),
            (format!("m:{tenant}:{user}:{method}"), self.limits.method),
        ];
        let Ok(mut map) = self.buckets.lock() else {
            return Err(LibraryError::internal());
        };
        if map.len() > PRUNE_ABOVE {
            map.retain(|_, b| now_ms - b.updated_ms < IDLE_MS);
        }
        // Look at all three before charging any, so a refusal costs nothing.
        let mut wait_ms: Option<f64> = None;
        for (k, l) in &keys {
            let b = map.entry(k.clone()).or_insert(Bucket {
                tokens: l.burst,
                updated_ms: now_ms,
            });
            Self::refill(b, now_ms, *l);
            if b.tokens < 1.0 {
                let w = (1.0 - b.tokens) / l.per_second * 1000.0;
                wait_ms = Some(wait_ms.map_or(w, |m: f64| m.max(w)));
            }
        }
        if let Some(w) = wait_ms {
            return Err(LibraryError::new(
                ErrorCode::RateLimited,
                format!("Too many requests. Try again in {} ms.", w.ceil() as u64),
            ));
        }
        for (k, _) in &keys {
            if let Some(b) = map.get_mut(k) {
                b.tokens -= 1.0;
            }
        }
        Ok(())
    }

    pub fn tracked_buckets(&self) -> usize {
        self.buckets.lock().map_or(0, |m| m.len())
    }
}
