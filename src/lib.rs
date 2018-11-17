//! This module is the user-facing entrypoint for circuit breaker
//! usage. The internal structures are available via other modules if
//! required, but are (for now) mostly undocumented.
//!
//! You should usually stick to this module for documentation and
//! examples.
//!
//! **Note**: The type aliases used by this public API require users
//! to supply [function pointers][] instead of closures. This is
//! because the type of a closure can not currently be named and
//! circuit breakers will often have to be stored in context structs
//! and the like without propagating the closure trait constraints all
//! the way up.
//!
//! If you need to store closures or other types that implement the
//! `Fn`-trait, please take a look at the internal modules.
//!
//! [function pointers]: https://doc.rust-lang.org/book/second-edition/ch19-05-advanced-functions-and-closures.html#function-pointers

mod circuit_breaker;
mod circuit_breaker_stats;
mod window;

pub mod command;
pub mod error;

pub use command::Config;
pub use error::CriusError;

/// Convenience type alias for function pointers matching the
/// input/output and error types of a circuit breaker.
pub type CommandFn<I, O, E> = fn(I) -> Result<O, E>;

/// Convenience type alias matching the fallback function pointer of a
/// circuit breaker.
pub type FallbackFn<O, E> = fn(E) -> O;

/// A Command is a runnable circuit breaker. It can be constructed
/// either with or without a fallback method that can provide
/// alternative values if the contained calls fail or if the breaker
/// is open.
///
/// # Type parameters:
///
/// * `I`: *Input* type to the breaker's function.
/// * `O`: *Output* type of the breaker's function.
/// * `E`: *Error* type returned by the breaker's function. This type
///   must implement `From<CriusError>` to propagate internal circuit
///   breaker errors.
pub type Command<I, O, E> = command::Command<I, O, E, CommandFn<I, O, E>, FallbackFn<O, E>>;

/// Use this function to construct a circuit breaker *without* a
/// fallback function.
///
/// # Example:
///
/// ```
/// # use crius::{command, Config, CriusError};
/// # #[derive(PartialEq, Debug)]
/// # struct ExampleError;
/// # impl From<CriusError> for ExampleError {
/// #   fn from(_: CriusError) -> Self { ExampleError }
/// # }
///
/// // Define a simple circuit breaker command:
/// let mut cmd = command(Config::default(), |n| {
///     if n > 10 {
///         Err(ExampleError)
///     } else {
///         Ok(n * 2)
///     }}).unwrap();
///
/// // and run it with an example input:
/// let result = cmd.run(10);
/// assert_eq!(Ok(20), result)
/// ```
pub fn command<I, O, E>(
    config: Config,
    function: CommandFn<I, O, E>,
) -> Result<Command<I, O, E>, CriusError>
where
    E: From<CriusError>,
{
    command::Command::define(config, function)
}

/// Use this function to construct a circuit breaker *with* a fallback
/// function:
/// # Example:
///
/// ```
/// # use crius::{command_with_fallback, Config, CriusError};
/// # #[derive(PartialEq, Debug)]
/// # struct ExampleError;
/// # impl From<CriusError> for ExampleError {
/// #   fn from(_: CriusError) -> Self { ExampleError }
/// # }
/// # let double_if_lt_ten = |n| if n > 10 {
/// #     Err(ExampleError)
/// # } else {
/// #     Ok(n * 2)
/// # };
/// #
/// // Define a simple circuit breaker command:
/// let mut cmd = command_with_fallback(
///     Config::default(),
///
///     // Same function as in the `command`-example
///     double_if_lt_ten,
///
///     // Define a fallback:
///     |_err| 4, // It's always four.
/// ).unwrap();
///
/// // and run it with an example input:
/// let result = cmd.run(11);
/// assert_eq!(Ok(4), result)
/// ```
pub fn command_with_fallback<I, O, E>(
    config: Config,
    function: CommandFn<I, O, E>,
    fallback: FallbackFn<O, E>,
) -> Result<Command<I, O, E>, CriusError>
where
    E: From<CriusError>,
{
    command::Command::define_with_fallback(config, function, fallback)
}
