//! Rate limiting controller for API calls.
//!
//! Corresponds to `crewai/utilities/rpm_controller.py`.
//!
//! Manages requests-per-minute (RPM) limiting to respect API rate limits.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::utilities::logger::Logger;

/// Manages requests per minute limiting.
///
/// When `max_rpm` is set, the controller tracks the number of requests
/// made in the current minute and blocks when the limit is reached.
/// A background timer resets the counter every 60 seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPMController {
    /// Maximum requests per minute. If `None`, no limit is applied.
    pub max_rpm: Option<i32>,
    /// Logger instance for status messages.
    #[serde(skip)]
    pub logger: Logger,

    // ---- Internal state (not serialized) ----
    /// Current request count in this minute window.
    #[serde(skip)]
    current_rpm: Arc<AtomicI32>,
    /// Flag to signal shutdown of the background timer.
    #[serde(skip)]
    shutdown_flag: Arc<AtomicBool>,
}

impl Default for RPMController {
    fn default() -> Self {
        Self {
            max_rpm: None,
            logger: Logger::new(false),
            current_rpm: Arc::new(AtomicI32::new(0)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl RPMController {
    /// Create a new `RPMController` with the given RPM limit.
    ///
    /// If `max_rpm` is `Some`, a background timer is started to reset
    /// the request counter every 60 seconds.
    pub fn new(max_rpm: Option<i32>) -> Self {
        let controller = Self {
            max_rpm,
            logger: Logger::new(false),
            current_rpm: Arc::new(AtomicI32::new(0)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        };

        if max_rpm.is_some() {
            controller.start_reset_timer();
        }

        controller
    }

    /// Check if a new request can be made, waiting if the RPM limit is reached.
    ///
    /// Returns `true` if the request was counted successfully.
    /// If the limit is reached, this method blocks for 60 seconds until the
    /// next minute window, then resets the counter and allows the request.
    pub fn check_or_wait(&self) -> bool {
        let max = match self.max_rpm {
            Some(max) => max,
            None => return true,
        };

        let current = self.current_rpm.fetch_add(1, Ordering::SeqCst);
        if current < max {
            return true;
        }

        // Max RPM reached, wait for next minute
        self.logger.log(
            "info",
            "Max RPM reached, waiting for next minute to start.",
            None,
        );
        self.wait_for_next_minute();
        self.current_rpm.store(1, Ordering::SeqCst);
        true
    }

    /// Stop the RPM counter and signal background timer to shut down.
    pub fn stop_rpm_counter(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }

    /// Get the current request count.
    pub fn current_rpm(&self) -> i32 {
        self.current_rpm.load(Ordering::SeqCst)
    }

    /// Wait for the next minute window (blocks for 60 seconds).
    fn wait_for_next_minute(&self) {
        thread::sleep(Duration::from_secs(60));
        self.current_rpm.store(0, Ordering::SeqCst);
    }

    /// Start a background daemon thread that resets the request counter
    /// every 60 seconds.
    fn start_reset_timer(&self) {
        let current_rpm = Arc::clone(&self.current_rpm);
        let shutdown_flag = Arc::clone(&self.shutdown_flag);

        thread::Builder::new()
            .name("rpm-controller-timer".to_string())
            .spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(60));
                    if shutdown_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    current_rpm.store(0, Ordering::SeqCst);
                }
            })
            .expect("Failed to spawn RPM controller timer thread");
    }
}

impl Drop for RPMController {
    fn drop(&mut self) {
        self.stop_rpm_counter();
    }
}
