use circuit_breaker_stats::CircuitBreakerStats;
use command::Config;
use error::CriusError;
use std::time::{Duration, Instant};
use window::Point;
use window::Window;

#[derive(Clone, Debug)]
pub struct CircuitBreaker {
    circuit_breaker_stats: CircuitBreakerStats,
    circuit_open_time: Option<Instant>,
    pub(crate) config: Config,
}

impl CircuitBreaker {
    /// Attempt to create a circuit breaker from the given
    /// configuration. This may return `None` if the configuration is
    /// invalid (e.g. if the configured durations overflow).
    pub fn new(config: Config) -> Result<CircuitBreaker, CriusError> {
        Window::new(config)
            .map(|window| CircuitBreaker {
                circuit_breaker_stats: CircuitBreakerStats { window },
                circuit_open_time: None,
                config: config,
            })
            .ok_or(CriusError::InvalidConfig)
    }

    pub fn check_command_allowed(&mut self) -> bool {
        if self.should_close_open_circuit() {
            self.circuit_open_time = None;
            true
        } else if self.should_keep_circuit_open() {
            false
        } else if self.should_open_circuit() {
            self.circuit_open_time = Some(Instant::now());
            self.circuit_breaker_stats.clear();
            false
        } else {
            true
        }
    }

    pub fn register_result<T, E>(&mut self, res: &Result<T, E>) {
        match *res {
            Ok(_) => self.circuit_breaker_stats.add_point(Point::SUCCESS),
            Err(_) => self.circuit_breaker_stats.add_point(Point::FAILURE),
        }
    }

    fn should_close_open_circuit(&mut self) -> bool {
        if let Some(open_time) = self.circuit_open_time {
            open_time <= self.time_to_close_circuit()
        } else {
            false
        }
    }

    fn should_keep_circuit_open(&mut self) -> bool {
        if let Some(open_time) = self.circuit_open_time {
            open_time > self.time_to_close_circuit()
        } else {
            false
        }
    }

    fn should_open_circuit(&mut self) -> bool {
        let pct_above_threshold =
            self.circuit_breaker_stats.error_percentage() >= self.config.error_threshold_percentage;

        let count_above_threshold =
            self.circuit_breaker_stats.error_nr() >= self.config.error_threshold;

        pct_above_threshold && count_above_threshold
    }

    fn time_to_close_circuit(&self) -> Instant {
        Instant::now() - Duration::from_millis(self.config.circuit_open_ms)
    }
}
