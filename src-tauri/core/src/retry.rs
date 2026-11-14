//! Another try for calls to outside services that fail for a passing reason: a dropped
//! connection, a timeout, HTTP 408 / 429 / 5xx. The wait doubles with each try, capped, with a
//! random part so several processes do not knock again at the same moment. Only calls that are
//! safe to repeat use it (reading Home Assistant, Edge speech, connecting to IMAP) - never a
//! Home Assistant service call or sending mail, which could happen twice.

use std::future::Future;
use std::time::Duration;

use crate::error::{AppError, Result};

pub struct Policy {
    pub attempts: u32,
    pub base_ms: u64,
    pub max_ms: u64,
}

/// Three tries, waiting about 0.4 s and 0.8 s between them.
pub const NETWORK: Policy = Policy { attempts: 3, base_ms: 400, max_ms: 4000 };
/// A single try (the same code path for calls that must not repeat).
pub const ONCE: Policy = Policy { attempts: 1, base_ms: 0, max_ms: 0 };

pub enum Fail {
    /// Worth another try.
    Transient(AppError),
    /// Final (bad request, wrong password, unknown voice...).
    Fatal(AppError),
}

pub fn transient_status(code: u16) -> bool {
    code == 408 || code == 429 || (500..600).contains(&code)
}

/// The wait after try `attempt` (1-based); `jitter` in 0..1 adds up to half of it.
pub fn delay(p: &Policy, attempt: u32, jitter: f64) -> Duration {
    let exp = p.base_ms.saturating_mul(1u64 << attempt.saturating_sub(1).min(16)).min(p.max_ms);
    Duration::from_millis(exp + (exp as f64 * 0.5 * jitter.clamp(0.0, 1.0)) as u64)
}

pub async fn run<T, F, Fut>(p: &Policy, mut call: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<T, Fail>>,
{
    let mut attempt = 1;
    loop {
        match call().await {
            Ok(v) => return Ok(v),
            Err(Fail::Fatal(e)) => return Err(e),
            Err(Fail::Transient(e)) if attempt >= p.attempts => return Err(e),
            Err(Fail::Transient(_)) => {
                tokio::time::sleep(delay(p, attempt, rand::random::<f64>())).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn backs_off_and_stops() {
        assert_eq!(delay(&NETWORK, 1, 0.0), Duration::from_millis(400));
        assert_eq!(delay(&NETWORK, 2, 1.0), Duration::from_millis(1200));
        assert_eq!(delay(&NETWORK, 9, 0.0), Duration::from_millis(4000));
        assert!(transient_status(503) && transient_status(429) && !transient_status(404));
        let fast = Policy { attempts: 3, base_ms: 1, max_ms: 2 };
        let calls = Cell::new(0);
        let out = crate::runtime::block_on(run(&fast, || {
            calls.set(calls.get() + 1);
            let n = calls.get();
            async move { if n < 3 { Err(Fail::Transient(AppError::msg("x"))) } else { Ok(7) } }
        }));
        assert_eq!((out.ok(), calls.get()), (Some(7), 3));
        calls.set(0);
        let out: Result<i32> = crate::runtime::block_on(run(&fast, || {
            calls.set(calls.get() + 1);
            async { Err(Fail::Fatal(AppError::msg("no"))) }
        }));
        assert_eq!((out.is_err(), calls.get()), (true, 1));
    }
}
