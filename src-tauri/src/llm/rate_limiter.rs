use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Leaky bucket rate limiter for API calls
pub struct RateLimiter {
    /// Maximum requests allowed
    capacity: u64,
    /// Current tokens in bucket
    tokens: AtomicU64,
    /// Leak rate (tokens per second)
    leak_rate: f64,
    /// Last update time
    last_update: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u64) -> Self {
        Self {
            capacity: requests_per_minute,
            tokens: AtomicU64::new(requests_per_minute),
            leak_rate: requests_per_minute as f64 / 60.0,
            last_update: Mutex::new(Instant::now()),
        }
    }

    /// Try to acquire a token, returns wait time if rate limited
    pub async fn acquire(&self) -> Option<Duration> {
        let mut last_update = self.last_update.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_update);

        // Replenish tokens based on elapsed time
        let replenished = (elapsed.as_secs_f64() * self.leak_rate) as u64;
        // Use Acquire/Release ordering for proper synchronization across threads
        let current = self.tokens.load(Ordering::Acquire);
        let new_tokens = (current + replenished).min(self.capacity);
        self.tokens.store(new_tokens, Ordering::Release);
        *last_update = now;

        // Try to consume a token
        if new_tokens > 0 {
            self.tokens.fetch_sub(1, Ordering::AcqRel);
            None
        } else {
            // Calculate wait time for next token
            let wait_secs = 1.0 / self.leak_rate;
            Some(Duration::from_secs_f64(wait_secs))
        }
    }

    /// Wait until a token is available
    pub async fn wait(&self) {
        loop {
            if let Some(wait_time) = self.acquire().await {
                tokio::time::sleep(wait_time).await;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(60); // 60 RPM = 1 RPS

        // First request should succeed immediately
        assert!(limiter.acquire().await.is_none());

        // Rapid requests should eventually be rate limited
        let mut rate_limited = false;
        for _ in 0..100 {
            if limiter.acquire().await.is_some() {
                rate_limited = true;
                break;
            }
        }
        assert!(rate_limited);
    }
}
