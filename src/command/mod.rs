mod circuit_breaker;
mod circuit_breaker_stats;
mod window;
pub mod error;

use self::error::reject_error::RejectError;
use self::circuit_breaker::CircuitBreaker;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::sync::mpsc;
use std::thread;

pub type CommandError = Error + Send + Sync + 'static;

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub error_threshold: Option<i32>,
    pub error_threshold_percentage: Option<i32>,
    pub buckets_in_window: Option<i32>,
    pub bucket_size_in_ms:  Option<u64>,
    pub circuit_open_ms: Option<u64>
}

impl Config {
    pub fn new() -> Config {
        return Config {
            error_threshold: None,
            error_threshold_percentage: None,
            buckets_in_window: None,
            bucket_size_in_ms: None,
            circuit_open_ms: None
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
}

pub struct Command<T, CMD> where T: Send, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send {
    pub config: Option<Config>,
    pub cmd: CMD
}

pub struct CommandWithFallback<T, CMD, FB> where T: Send, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send, FB: Fn(Box<CommandError>) -> T + Sync + Send {
    pub fb: FB,
    pub config: Option<Config>,
    pub cmd: CMD
}

impl <T, CMD> Command<T, CMD> where T: Send + 'static, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send {
    pub fn define(cmd: CMD) -> Command<T, CMD> {
        return Command {
            cmd: cmd,
            config: None
        }
    }

    pub fn define_with_fallback<FB>(cmd: CMD, fallback: FB) -> CommandWithFallback<T, CMD, FB> where FB: Fn(Box<CommandError>) -> T + Sync + Send {
        return CommandWithFallback {
            cmd: cmd,
            fb: fallback,
            config: None
        }
    }
}

impl <T, CMD> Command<T, CMD> where T: Send + 'static, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send {

    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        return self
    }

    pub fn create(self) -> RunnableCommand<T, CMD, fn(Box<CommandError>) -> T> {
        return RunnableCommand::new(self.cmd, None, self.config)
    }
}

impl <T, CMD, FB> CommandWithFallback<T, CMD, FB> where T: Send + 'static, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send, FB: Fn(Box<CommandError>) -> T + Sync + Send + 'static {
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        return self
    }

    pub fn create(self) -> RunnableCommand<T, CMD, FB> {
        return RunnableCommand::new(self.cmd, Some(self.fb), self.config)
    }
}

const DEFAULT_ERROR_THRESHOLD: i32 = 10;
const DEFAULT_ERROR_THRESHOLD_PERCENTAGE: i32 = 50;
const DEFAULT_BUCKETS_IN_WINDOW: i32 = 10;
const DEFAULT_BUCKET_SIZE_IN_MS: u64 = 1000;
const DEFAULT_CIRCUIT_OPEN_MS: u64 = 5000;

pub struct RunnableCommand<T, CMD, FB> where T: Send + 'static, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send + 'static, FB: Fn(Box<CommandError>) -> T + Sync + Send + 'static {
    command_params: Arc<Mutex<CommandParams<T, CMD, FB>>>
}

impl <T, CMD, FB> RunnableCommand<T, CMD, FB> where T: Send + 'static, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send + 'static, FB: Fn(Box<CommandError>) -> T + Sync + Send + 'static {

    fn new(cmd: CMD,
           fb: Option<FB>,
           config: Option<Config>) -> RunnableCommand<T, CMD, FB> {
        let final_config = Config {
            error_threshold: config.and_then(|c| c.error_threshold).or(Some(DEFAULT_ERROR_THRESHOLD)),
            error_threshold_percentage: config.and_then(|c| c.error_threshold_percentage).or(Some(DEFAULT_ERROR_THRESHOLD_PERCENTAGE)),
            buckets_in_window: config.and_then(|c| c.buckets_in_window).or(Some(DEFAULT_BUCKETS_IN_WINDOW)),
            bucket_size_in_ms: config.and_then(|c| c.bucket_size_in_ms).or(Some(DEFAULT_BUCKET_SIZE_IN_MS)),
            circuit_open_ms: config.and_then(|c| c.circuit_open_ms).or(Some(DEFAULT_CIRCUIT_OPEN_MS)),
        };

        return RunnableCommand {
            command_params: Arc::new(Mutex::new(CommandParams {
                cmd: cmd,
                fb: fb,
                circuit_breaker:CircuitBreaker::new(final_config)
            }))
        }
    }

    pub fn run(&mut self) -> Receiver<Result<T, Box<CommandError>>> {
        let command = self.command_params.clone();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let is_allowed = command.lock().unwrap().circuit_breaker.check_command_allowed();
            if is_allowed {
                let res = (command.lock().unwrap().cmd)();
                command.lock().unwrap().circuit_breaker.register_result(&res);

                if command.lock().unwrap().fb.is_some() && res.is_err() {
                    let final_res = Ok(res.unwrap_or_else(command.lock().unwrap().fb.as_ref().unwrap()));
                    tx.send(final_res)
                } else {
                    tx.send(res)
                }
            } else if command.lock().unwrap().fb.is_some() {
                let res = (command.lock().unwrap().fb.as_ref().unwrap())(Box::new(RejectError {}));
                tx.send(Ok(res))
            } else {
                tx.send(Err(Box::new(RejectError {}) as Box<CommandError>))
            }
        });

        return rx
    }
}

struct CommandParams<T, CMD, FB> where T: Send + 'static, CMD: Fn() -> Result<T, Box<CommandError>> + Sync + Send + 'static, FB: Fn(Box<CommandError>) -> T + Sync + Send + 'static {
    cmd: CMD,
    fb: Option<FB>,
    circuit_breaker: CircuitBreaker
}