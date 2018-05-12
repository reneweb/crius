mod circuit_breaker;
mod circuit_breaker_stats;
mod window;
pub mod error;

use self::error::CriusError;
use self::circuit_breaker::CircuitBreaker;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::sync::mpsc;
use threadpool::ThreadPool;
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub error_threshold: Option<i32>,
    pub error_threshold_percentage: Option<i32>,
    pub buckets_in_window: Option<i32>,
    pub bucket_size_in_ms:  Option<u64>,
    pub circuit_open_ms: Option<u64>,
    pub threadpool_size: Option<i32>,
    pub circuit_breaker_enabled: Option<bool>
}

impl Config {
    pub fn new() -> Config {
        return Config {
            error_threshold: None,
            error_threshold_percentage: None,
            buckets_in_window: None,
            bucket_size_in_ms: None,
            circuit_open_ms: None,
            threadpool_size: None,
            circuit_breaker_enabled: None
        }
    }

    pub fn error_threshold(&mut self, error_threshold: i32) -> &mut Self {
        self.error_threshold = Some(error_threshold);
        return self;
    }

    pub fn error_threshold_percentage(&mut self, error_threshold_percentage: i32) -> &mut Self {
        self.error_threshold_percentage = Some(error_threshold_percentage);
        return self;
    }

    pub fn buckets_in_window(&mut self, buckets_in_window: i32) -> &mut Self {
        self.buckets_in_window = Some(buckets_in_window);
        return self;
    }

    pub fn bucket_size_in_ms(&mut self, bucket_size_in_ms: u64) -> &mut Self {
        self.bucket_size_in_ms = Some(bucket_size_in_ms);
        return self;
    }

    pub fn circuit_open_ms(&mut self, circuit_open_ms: u64) -> &mut Self {
        self.circuit_open_ms = Some(circuit_open_ms);
        return self;
    }

    pub fn circuit_breaker_enabled(&mut self, circuit_breaker_enabled: bool) -> &mut Self {
        self.circuit_breaker_enabled = Some(circuit_breaker_enabled);
        return self;
    }
}

pub struct Command<I, O, E, F> where
    O: Send,
    E: From<CriusError>,
    F: Fn(I) -> Result<O, E> + Sync + Send {
    pub config: Option<Config>,
    pub cmd: F,
    phantom_data: PhantomData<I>
}

pub struct CommandWithFallback<I, O, E, F, FB> where
    O: Send,
    E: From<CriusError>,
    F: Fn(I) -> Result<O, E> + Sync + Send,
    FB: Fn(E) -> O + Sync + Send {
    pub fb: FB, // TODO: rename to fallback
    pub config: Option<Config>,
    pub cmd: F,
    phantom_data: PhantomData<I>
}

impl <I, O, E, F> Command<I, O, E, F> where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + From<CriusError> + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send, {
    pub fn define(cmd: F) -> Command<I, O, E, F> {
        return Command {
            cmd: cmd,
            config: None,
            phantom_data: PhantomData
        }
    }

    pub fn define_with_fallback<FB>(cmd: F, fallback: FB)
                                    -> CommandWithFallback<I, O, E, F, FB>
        where FB: Fn(E) -> O + Sync + Send {
        return CommandWithFallback {
            cmd: cmd,
            fb: fallback,
            config: None,
            phantom_data: PhantomData
        }
    }

    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        return self
    }

    pub fn create(self) -> RunnableCommand<I, O, E, F, fn(E) -> O> {
        return RunnableCommand::new(self.cmd, None, self.config)
    }

}

impl <I, O, E, F, FB> CommandWithFallback<I, O, E, F, FB> where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + From<CriusError> + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send,
    FB: Fn(E) -> O + Sync + Send + 'static {
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        return self
    }

    pub fn create(self) -> RunnableCommand<I, O, E, F, FB> {
        return RunnableCommand::new(self.cmd, Some(self.fb), self.config)
    }
}

const DEFAULT_ERROR_THRESHOLD: i32 = 10;
const DEFAULT_ERROR_THRESHOLD_PERCENTAGE: i32 = 50;
const DEFAULT_BUCKETS_IN_WINDOW: i32 = 10;
const DEFAULT_BUCKET_SIZE_IN_MS: u64 = 1000;
const DEFAULT_CIRCUIT_OPEN_MS: u64 = 5000;
const DEFAULT_THREADPOOL_SIZE: i32 = 10;
const DEFAULT_CIRCUIT_BREAKER_ENABLED: bool = true;

pub struct RunnableCommand<I, O, E, F, FB> where
    O: Send + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send + 'static,
    FB: Fn(E) -> O + Sync + Send + 'static {
    command_params: Arc<Mutex<CommandParams<I, O, E, F, FB>>>,
    pool: ThreadPool
}

impl <I, O, E, F, FB> RunnableCommand<I, O, E, F, FB> where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + From<CriusError> + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send + 'static,
    FB: Fn(E) -> O + Sync + Send + 'static {

    fn new(cmd: F,
           fb: Option<FB>,
           config: Option<Config>) -> RunnableCommand<I, O, E, F, FB> {
        let final_config = Config {
            error_threshold: config.and_then(|c| c.error_threshold).or(Some(DEFAULT_ERROR_THRESHOLD)),
            error_threshold_percentage: config.and_then(|c| c.error_threshold_percentage).or(Some(DEFAULT_ERROR_THRESHOLD_PERCENTAGE)),
            buckets_in_window: config.and_then(|c| c.buckets_in_window).or(Some(DEFAULT_BUCKETS_IN_WINDOW)),
            bucket_size_in_ms: config.and_then(|c| c.bucket_size_in_ms).or(Some(DEFAULT_BUCKET_SIZE_IN_MS)),
            circuit_open_ms: config.and_then(|c| c.circuit_open_ms).or(Some(DEFAULT_CIRCUIT_OPEN_MS)),
            threadpool_size: config.and_then(|c| c.threadpool_size).or(Some(DEFAULT_THREADPOOL_SIZE)),
            circuit_breaker_enabled: config.and_then(|c| c.circuit_breaker_enabled).or(Some(DEFAULT_CIRCUIT_BREAKER_ENABLED))
        };

        return RunnableCommand {
            command_params: Arc::new(Mutex::new(CommandParams {
                config: final_config,
                cmd: cmd,
                fb: fb,
                circuit_breaker:CircuitBreaker::new(final_config),
                phantom_data: PhantomData
            })),
            pool: ThreadPool::new(1)
        }
    }

    pub fn run(&mut self, param: I) -> Receiver<Result<O, E>> {
        let command = self.command_params.clone();
        let (tx, rx) = mpsc::channel();

        self.pool.execute(move || {
            let is_allowed = command.lock().unwrap().circuit_breaker.check_command_allowed();
            if !command.lock().unwrap().config.circuit_breaker_enabled.unwrap_or(true) {
                let res = (command.lock().unwrap().cmd)(param);
                tx.send(res).unwrap()
            } else if is_allowed {
                let res = (command.lock().unwrap().cmd)(param);
                command.lock().unwrap().circuit_breaker.register_result(&res);

                if command.lock().unwrap().fb.is_some() && res.is_err() {
                    let final_res = Ok(res.unwrap_or_else(command.lock().unwrap().fb.as_ref().unwrap()));
                    tx.send(final_res).unwrap()
                } else {
                    tx.send(res).unwrap()
                }
            } else if command.lock().unwrap().fb.is_some() {
                let err = E::from(CriusError::ExecutionRejected);
                let result = (command.lock().unwrap().fb.as_ref().unwrap())(err);
                tx.send(Ok(result)).ok();
            } else {
                let err = E::from(CriusError::ExecutionRejected);
                tx.send(Err(err)).ok();
            }
        });

        return rx
    }
}

struct CommandParams<I, O, E, F, FB> where
    O: Send + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send + 'static,
    FB: Fn(E) -> O + Sync + Send + 'static {
    config: Config,
    cmd: F,
    fb: Option<FB>,
    circuit_breaker: CircuitBreaker,
    phantom_data: PhantomData<I>,
}
