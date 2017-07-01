extern crate crius;

mod circuit_breaker {

    use crius::command::Config;
    use crius::command::Command;
    use crius::command::RunnableCommand;
    use crius::command::error::RejectError;
    use std::error::Error;
    use std::fmt::Display;
    use std::fmt;
    use std::{thread, time};

    #[derive(PartialEq, Eq, Copy, Clone, Debug)]
    struct TestError {}
    impl Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter) ->  fmt::Result {
             write!(f, "An error happened")
         }
    }
    impl Error for TestError {
        fn description(&self) -> &str {
            return "An error happened"
        }
    }
    unsafe impl Send for TestError {}
    unsafe impl Sync for TestError {}

    #[test]
    fn runs_command() {
        let rx = Command::define(|| {
                return Ok(5)
            }).create().run();

        assert_eq!(5, rx.recv().unwrap().unwrap());
    }

    #[test]
    fn runs_command_multiple_times() {
        let mut cmd = Command::define(|| {
            return Ok(5)
        }).create();

        for _ in 0..5 {
            let rx = cmd.run();
            assert_eq!(5, rx.recv().unwrap().unwrap());
        }
    }

    #[test]
    fn rejects_command_if_circuit_open() {
        let mut cmd: RunnableCommand<i32, _, _> = Command::define(|| {
            return Err(Box::new(TestError {}));
        }).config(*Config::new().error_threshold(5)).create();

        for _ in 0..5 {
            let rx = cmd.run();
            assert_eq!(true, rx.recv().unwrap().unwrap_err().is::<TestError>()); // Fallback by returned error
        }

        let rx = cmd.run();
        assert_eq!(true, rx.recv().unwrap().unwrap_err().is::<RejectError>()); // Fallback by reject error
    }

    #[test]
    fn returns_fallback_if_err_result_returned() {
        let mut cmd = Command::define_with_fallback(|| {
            return Err(Box::new(TestError {}))
        }, |_| {
            return 5;
        }).create();

        let rx = cmd.run();
        assert_eq!(5, rx.recv().unwrap().unwrap());
    }

    #[test]
    fn returns_fallback_if_circuit_open() {
        let mut cmd = Command::define_with_fallback(|| {
            return Err(Box::new(TestError {}))
        }, |_| {
            return 5;
        }).config(*Config::new().error_threshold(5)).create();

        for _ in 0..5 {
            let rx = cmd.run();
            assert_eq!(5, rx.recv().unwrap().unwrap()); // Fallback by returned error
        }

        let rx = cmd.run();
        assert_eq!(5, rx.recv().unwrap().unwrap()); // Fallback by reject error
    }

    #[test]
    fn handles_lots_of_calls() {
        let mut cmd = Command::define(|| {
            let ten_millis = time::Duration::from_millis(10);
            thread::sleep(ten_millis);

            return Ok(5)
        }).create();

        let mut rxs = Vec::new();
        for _ in 0..1000 {
            rxs.push(cmd.run());
        }

        for rx in rxs {
            assert_eq!(5, rx.recv().unwrap().unwrap());
        }
    }
}