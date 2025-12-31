//! Retry policy: decides backoff delays.

use std::time::Duration;

/// Retry policy for failed tasks.
///
/// v1: Simple policy with configurable base delay.
/// Future: Could add jitter, exponential backoff variants, etc.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Base delay for the first retry.
    pub base_delay: Duration,

    /// Backoff multiplier for exponential backoff.
    pub multiplier: f64,
}

impl RetryPolicy {
    /// Default policy for v1 (matches requirements: 5 max attempts, reasonable backoff).
    pub fn default_v1() -> Self {
        Self {
            base_delay: Duration::from_secs(2),
            multiplier: 2.0,
        }
    }

    /// Calculate delay for the next retry based on attempt number.
    ///
    /// # Arguments
    /// * `attempts` - Number of attempts already made (1-indexed).
    ///
    /// # Returns
    /// Duration to wait before the next retry.
    ///
    /// # Design note
    /// This is the core "judgment logic" for retry timing.
    /// v1 uses exponential backoff: delay = base_delay * multiplier^(attempts - 1)
    ///
    /// Example with base_delay=2s, multiplier=2.0:
    /// - attempt 1 (first failure): 2s
    /// - attempt 2: 4s
    /// - attempt 3: 8s
    /// - attempt 4: 16s
    /// - attempt 5: 32s
    pub fn next_delay(&self, attempts: u32) -> Duration {
        // TODO(human): Implement exponential backoff logic here.
        // Calculate: base_delay * multiplier^(attempts - 1)
        // Hints:
        // - Use attempts.saturating_sub(1) to get the exponent (0-indexed)
        // - Use f64::powi() for power calculation
        // - Convert Duration to f64 (as_secs_f64), calculate, then from_secs_f64
        // - Handle edge cases: attempts=0 should probably use base_delay
        let base_secs = self.base_delay.as_secs_f64();
        let delay_secs = base_secs * self.multiplier.powi((attempts.saturating_sub(1)) as i32);
        Duration::from_secs_f64(delay_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_has_reasonable_values() {
        let policy = RetryPolicy::default_v1();
        assert_eq!(policy.base_delay, Duration::from_secs(2));
        assert_eq!(policy.multiplier, 2.0);
    }

    #[test]
    fn exponential_backoff_increases() {
        let policy = RetryPolicy::default_v1();

        let d1 = policy.next_delay(1);
        let d2 = policy.next_delay(2);
        let d3 = policy.next_delay(3);

        // Each delay should be larger than the previous
        assert!(d2 > d1);
        assert!(d3 > d2);

        // With base=2s, multiplier=2.0:
        // attempt 1: 2s, attempt 2: 4s, attempt 3: 8s
        assert_eq!(d1, Duration::from_secs(2));
        assert_eq!(d2, Duration::from_secs(4));
        assert_eq!(d3, Duration::from_secs(8));
    }
}
