# Crius [![Build Status](https://travis-ci.org/reneweb/crius.svg?branch=master)](https://travis-ci.org/reneweb/crius) [![Cargo Version](https://img.shields.io/crates/v/crius.svg)](https://crates.io/crates/crius)

Crius is a simple hystrix-like circuit breaker for rust.

_"In the midst of chaos, there is also opportunity"_

## Usage

### Simple command
```rust
use crius::{command, Config, CriusError};

#[derive(PartialEq, Debug)]
struct ExampleError;
impl From<CriusError> for ExampleError {
  fn from(_: CriusError) -> Self { ExampleError }
}

// Define a simple circuit breaker command:
let mut cmd = command(Config::default(), |n| {
  if n > 10 {
    Err(ExampleError)
  } else {
    Ok(n * 2)
  }}).unwrap();

// and run it with an example input:
let result = cmd.run(10);
assert_eq!(Ok(20), result);
```

### Command with fallback
```rust
use crius::{command_with_fallback, Config, CriusError};

#[derive(PartialEq, Debug)]
struct ExampleError;
impl From<CriusError> for ExampleError {
  fn from(_: CriusError) -> Self { ExampleError }
}

let double_if_lt_ten = |n| if n > 10 {
  Err(ExampleError)
} else {
  Ok(n * 2)
};

// Define a simple circuit breaker command:
let mut cmd = command_with_fallback(
    Config::default(),
    double_if_lt_ten,

    // Define a fallback:
    |_err| 4, // It's always four.
).unwrap();

// and run it with an example input:
let result = cmd.run(11);
assert_eq!(Ok(4), result);
```

### Command with custom configuration
```rust
use crius::{command, Config, CriusError};

let config = *Config::default()
    .circuit_open_ms(5000)
    .error_threshold(10)
    .error_threshold_percentage(50)
    .buckets_in_window(100)
    .bucket_size_in_ms(1000);

let mut cmd = command(config, |n| {
  if n > 10 {
    Err(ExampleError)
  } else {
    Ok(n * 2)
  }}).unwrap();

// and run it with an example input:
let result = cmd.run(10);
assert_eq!(Ok(20), result);
```

## Configuration

`circuit_open_ms` - Time in ms commands are rejected after the circuit opened - Default 5000

`error_threshold` - Minimum amount of errors for the circuit to break - Default 10

`error_threshold_percentage` - Minimum error percentage for the circuit to break - Default 50

`buckets_in_window` - Rolling window to track success/error calls, this property defines the amount of buckets in a window (buckets_in_window * bucket_size_in_ms is the overall length in ms of the window) - Default 10

`bucket_size_in_ms` - This property defines the ms a bucket is long, i.e. each x ms a new bucket will be created (buckets_in_window * bucket_size_in_ms is the overall length in ms of the window) - Default 1000

`circuit_breaker_enabled` - Defines if the circuit breaker is enabled or not - Default true