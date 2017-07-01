# Crius

Crius is a simple hystrix-like circuit breaker for rust.

_"In the midst of chaos, there is also opportunity"_

## Usage

### Simple command
```rust
extern crate crius;

use crius::command::Command;

fn run_command() {
    let receiver = Command::define(|| {
        return Ok("Ok Result")
    }).create().run();

    assert_eq!("Ok Result", receiver.recv().unwrap().unwrap());
}
```

### Command with fallback
```rust
extern crate crius;

use crius::command::Command;
use std::error::Error;
use std::fmt;

//Define Error Type
#[derive(Debug)]
struct MyError;
impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An error happened")
    }
}
impl Error for MyError {
    fn description(&self) -> &str {
        return "My Error"
    }
}
unsafe impl Send for MyError {}
unsafe impl Sync for MyError {}

fn run_command_with_fallback() {
    let receiver = Command::define_with_fallback(|| {
        return Err(Box::new(MyError {}))
    }, |e| {
        return "Fallback result if an error occurred";
    }).create().run();

    assert_eq!("Fallback result if an error occurred", receiver.recv().unwrap().unwrap());
}
```

### Command with custom configuration
```rust
extern crate crius;

use crius::command::Command;
use crius::command::Config;

fn run_command_with_fallback() {
    let config = *Config::new()
        .circuit_open_ms(5000)
        .error_threshold(10)
        .error_threshold_percentage(50)
        .buckets_in_window(100)
        .bucket_size_in_ms(1000);

    let receiver = Command::define(|| {
        return Ok("Ok Result")
    }).config(config).create().run();

    assert_eq!("Ok Result", receiver.recv().unwrap().unwrap());
}
```

### Error types and handling

The error types that are provided by trychis can be found in `crius::command::error`. 
Currently it only contains one error `RejectError`, which is returned when the circuit is open and therefore the command rejected.
 
If an error is occurring it will be returned in the receiver, except when a fallback is provided where it is than passed as a param to the fallback function.
The returned / passed error is of type `Error + Send + Sync + 'static` - with this we can check and downcast to the original error, for example:

```rust
fn command_with_error_handlong() {

    //Note: we explicitly need to define the success type here, as it is not in the command function returned nor is there a fallback to provide it.
    let receiver = Command::<i32, _>::define(|| {
        return Err(Box::new(MyError { my_error_code: 1234 }))
    }).create().run();

    let err = receiver.recv().unwrap().unwrap_err();
    if err.is::<MyError>() {
        let my_error = err.downcast_ref::<MyError>().unwrap();
        assert_eq!(1234, my_error.my_error_code);
    } else {
        let reject_error = err.downcast_ref::<RejectError>().unwrap();
        //...
    }
}
```

## Configuration

`circuit_open_ms` - Time in ms commands are rejected after the circuit opened - Default 5000

`error_threshold` - Minimum amount of errors for the circuit to break - Default 10

`error_threshold_percentage` - Minimum error percentage for the circuit to break - Default 50

`buckets_in_window` - Trychis is using a rolling window to track success/error calls, this property defines the amount of buckets in a window (buckets_in_window * bucket_size_in_ms is the overall length in ms of the window) - Default 10

`bucket_size_in_ms` - This property defines the ms a bucket is long, i.e. each x ms a new bucket will be created (buckets_in_window * bucket_size_in_ms is the overall length in ms of the window) - Default 1000