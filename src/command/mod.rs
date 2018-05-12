mod circuit_breaker;
mod circuit_breaker_stats;
mod window;

use error::CriusError;
use self::circuit_breaker::CircuitBreaker;
use std::marker::PhantomData;
use std::sync::mpsc::Receiver;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

const DEFAULT_ERROR_THRESHOLD: i32 = 10;
const DEFAULT_ERROR_THRESHOLD_PERCENTAGE: i32 = 50;
const DEFAULT_BUCKETS_IN_WINDOW: i32 = 10;
const DEFAULT_BUCKET_SIZE_IN_MS: u64 = 1000;
const DEFAULT_CIRCUIT_OPEN_MS: u64 = 5000;
const DEFAULT_THREADPOOL_SIZE: i32 = 10;
const DEFAULT_CIRCUIT_BREAKER_ENABLED: bool = true;

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub error_threshold: i32,
    pub error_threshold_percentage: i32,
    pub buckets_in_window: i32,
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

    pub fn buckets_in_window(&mut self, buckets_in_window: i32) -> &mut Self {
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
    O: Send,
    E: From<CriusError>,
    F: Fn(I) -> Result<O, E> + Sync + Send,
    FB: Fn(E) -> O + Sync + Send,
{
    pub config: Option<Config>,
    pub cmd: F,
    pub fallback: Option<FB>,
    phantom_data: PhantomData<I>,
}

impl<I, O, E, F, FB> Command<I, O, E, F, FB>
where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + From<CriusError> + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send,
    FB: Fn(E) -> O + Sync + Send,
{
    pub fn define(cmd: F) -> Command<I, O, E, F, FB> {
        return Command {
            cmd: cmd,
            config: None,
            fallback: None,
            phantom_data: PhantomData,
        };
    }

    pub fn define_with_fallback(cmd: F, fallback: FB) -> Command<I, O, E, F, FB> {
        return Command {
            cmd: cmd,
            fallback: Some(fallback),
            config: None,
            phantom_data: PhantomData,
        };
    }

    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        return self;
    }

    pub fn create(self) -> RunnableCommand<I, O, E, F, FB> {
        return RunnableCommand::new(self.cmd, self.fallback, self.config);
    }
}

pub struct RunnableCommand<I, O, E, F, FB>
where
    O: Send + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send + 'static,
    FB: Fn(E) -> O + Sync + Send + 'static,
{
    command_params: Arc<Mutex<CommandParams<I, O, E, F, FB>>>,
    pool: ThreadPool,
}

impl<I, O, E, F, FB> RunnableCommand<I, O, E, F, FB>
where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + From<CriusError> + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send + 'static,
    FB: Fn(E) -> O + Sync + Send + 'static,
{
    fn new(cmd: F, fb: Option<FB>, config: Option<Config>)
           -> RunnableCommand<I, O, E, F, FB> {
        let config = config.unwrap_or(Config::default());
        RunnableCommand {
            command_params: Arc::new(Mutex::new(CommandParams {
                config,
                cmd,
                fb,
                circuit_breaker: CircuitBreaker::new(config),
                phantom_data: PhantomData,
            })),
            pool: ThreadPool::new(1),
        }
    }

    pub fn run(&mut self, param: I) -> Receiver<Result<O, E>> {
        let command = self.command_params.clone();
        let (tx, rx) = mpsc::channel();

        self.pool.execute(move || {
            let is_allowed = command
                .lock()
                .unwrap()
                .circuit_breaker
                .check_command_allowed();
            if !command
                .lock()
                .unwrap()
                .config
                .circuit_breaker_enabled
            {
                let res = (command.lock().unwrap().cmd)(param);
                tx.send(res).unwrap()
            } else if is_allowed {
                let res = (command.lock().unwrap().cmd)(param);
                command.lock().unwrap().circuit_breaker.register_result(
                    &res,
                );

                if command.lock().unwrap().fb.is_some() && res.is_err() {
                    let final_res = Ok(res.unwrap_or_else(
                        command.lock().unwrap().fb.as_ref().unwrap(),
                    ));
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

        return rx;
    }
}

struct CommandParams<I, O, E, F, FB>
where
    O: Send + 'static,
    F: Fn(I) -> Result<O, E> + Sync + Send + 'static,
    FB: Fn(E) -> O + Sync + Send + 'static,
{
    config: Config,
    cmd: F,
    fb: Option<FB>,
    circuit_breaker: CircuitBreaker,
    phantom_data: PhantomData<I>,
}
