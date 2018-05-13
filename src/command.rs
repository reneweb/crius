use error::CriusError;
use circuit_breaker::CircuitBreaker;
use std::marker::PhantomData;

const DEFAULT_ERROR_THRESHOLD: i32 = 10;
const DEFAULT_ERROR_THRESHOLD_PERCENTAGE: i32 = 50;
const DEFAULT_BUCKETS_IN_WINDOW: u32 = 10;
const DEFAULT_BUCKET_SIZE_IN_MS: u64 = 1000;
const DEFAULT_CIRCUIT_OPEN_MS: u64 = 5000;
const DEFAULT_THREADPOOL_SIZE: i32 = 10;
const DEFAULT_CIRCUIT_BREAKER_ENABLED: bool = true;

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub error_threshold: i32,
    pub error_threshold_percentage: i32,
    pub buckets_in_window: u32,
    pub bucket_size_in_ms: u64,
    pub circuit_open_ms: u64,
    pub threadpool_size: i32,
    pub circuit_breaker_enabled: bool,
}

impl Config {
    pub fn default() -> Config {
        Config {
            error_threshold: DEFAULT_ERROR_THRESHOLD,
            error_threshold_percentage: DEFAULT_ERROR_THRESHOLD_PERCENTAGE,
            buckets_in_window: DEFAULT_BUCKETS_IN_WINDOW,
            bucket_size_in_ms: DEFAULT_BUCKET_SIZE_IN_MS,
            circuit_open_ms: DEFAULT_CIRCUIT_OPEN_MS,
            threadpool_size: DEFAULT_THREADPOOL_SIZE,
            circuit_breaker_enabled: DEFAULT_CIRCUIT_BREAKER_ENABLED,
        }
    }

    pub fn error_threshold(&mut self, error_threshold: i32) -> &mut Self {
        self.error_threshold = error_threshold;
        return self;
    }

    pub fn error_threshold_percentage(&mut self, error_threshold_percentage: i32) -> &mut Self {
        self.error_threshold_percentage = error_threshold_percentage;
        return self;
    }

    pub fn buckets_in_window(&mut self, buckets_in_window: u32) -> &mut Self {
        self.buckets_in_window = buckets_in_window;
        return self;
    }

    pub fn bucket_size_in_ms(&mut self, bucket_size_in_ms: u64) -> &mut Self {
        self.bucket_size_in_ms = bucket_size_in_ms;
        return self;
    }

    pub fn circuit_open_ms(&mut self, circuit_open_ms: u64) -> &mut Self {
        self.circuit_open_ms = circuit_open_ms;
        return self;
    }

    pub fn circuit_breaker_enabled(&mut self, circuit_breaker_enabled: bool) -> &mut Self {
        self.circuit_breaker_enabled = circuit_breaker_enabled;
        return self;
    }
}

pub struct Command<I, O, E, F, FB>
where
    E: From<CriusError>,
    F: Fn(I) -> Result<O, E>,
    FB: Fn(E) -> O,
{
    pub cmd: F,
    pub fallback: Option<FB>,
    phantom_data: PhantomData<I>,
    circuit_breaker: CircuitBreaker,
}

impl<I, O, E, F, FB> Command<I, O, E, F, FB>
where
    E: From<CriusError>,
    F: Fn(I) -> Result<O, E>,
    FB: Fn(E) -> O,
{
    pub fn define(cfg: Config, cmd: F) -> Result<Command<I, O, E, F, FB>, CriusError> {
        Ok(Command {
            cmd: cmd,
            fallback: None,
            phantom_data: PhantomData,
            circuit_breaker: CircuitBreaker::new(cfg)?,
        })
    }

    pub fn define_with_fallback(
        cfg: Config,
        cmd: F,
        fallback: FB,
    ) -> Result<Command<I, O, E, F, FB>, CriusError> {
        Ok(Command {
            cmd: cmd,
            fallback: Some(fallback),
            phantom_data: PhantomData,
            circuit_breaker: CircuitBreaker::new(cfg)?,
        })
    }

    pub fn run(&mut self, param: I) -> Result<O, E> {
        // Run the command if the breaker is disabled:
        let enabled = self.circuit_breaker.config.circuit_breaker_enabled;
        if !enabled {
            return (self.cmd)(param);
        }

        // Execute the command if the breaker is enabled and execution
        // is allowed.
        let is_allowed = self.circuit_breaker.check_command_allowed();
        if is_allowed {
            let result = (self.cmd)(param);
            self.circuit_breaker.register_result(&result);

            return match result {
                Ok(result) => Ok(result),
                Err(err) => {
                    // If a fallback is configured, use it on error:
                    if let Some(ref fallback) = self.fallback {
                        Ok(fallback(err))
                    } else {
                        Err(err)
                    }
                }
            };
        }

        // If execution is rejected, either run the configured
        // fallback (if present) or propagate the rejection as an
        // error:
        let err = E::from(CriusError::ExecutionRejected);
        if let Some(ref fallback) = self.fallback {
            return Ok(fallback(err));
        } else {
            return Err(err);
        }
    }
}
