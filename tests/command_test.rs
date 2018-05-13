extern crate crius;

mod circuit_breaker {
    use crius::command::Config;
    use crius::command::Command;
    use crius::error::CriusError;
    use std::error::Error;
    use std::fmt::Display;
    use std::fmt;
    use std::{thread, time};

    #[derive(PartialEq, Eq, Copy, Clone, Debug)]
    enum TestError {
        Internal,
        External,
    }

    impl Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "An error happened")
        }
    }

    impl Error for TestError {
        fn description(&self) -> &str {
            return "An error happened";
        }
    }

    impl From<CriusError> for TestError {
        fn from(_: CriusError) -> Self {
            TestError::External
        }
    }

    type TestCommand<I, O> =
        Command<I, O, TestError, fn(I) -> Result<O, TestError>, fn(TestError) -> O>;

    #[test]
    fn runs_command() {
        let result = TestCommand::<(), u8>::define(Config::default(), |_| Ok(5))
            .unwrap()
            .run(());
        assert_eq!(5, result.unwrap());
    }

    #[test]
    fn runs_command_multiple_times() {
        let mut cmd = TestCommand::<(), u8>::define(Config::default(), |_| return Ok(5)).unwrap();

        for _ in 0..5 {
            let result = cmd.run(());
            assert_eq!(5, result.unwrap());
        }
    }

    #[test]
    fn runs_command_with_param() {
        let result = TestCommand::<u8, u8>::define(Config::default(), |param| Ok(param))
            .unwrap()
            .run(5);

        assert_eq!(5, result.unwrap());
    }

    #[test]
    fn rejects_command_if_circuit_open() {
        let mut cmd = TestCommand::<(), ()>::define(*Config::default().error_threshold(5), |_| {
            Err(TestError::Internal)
        }).unwrap();

        for _ in 0..5 {
            let err = cmd.run(()).expect_err("Expected internal error");
            assert_eq!(TestError::Internal, err); // Fallback by returned error
        }

        let err = cmd.run(()).expect_err("Expected external error");
        assert_eq!(TestError::External, err); // Fallback by reject error
    }

    #[test]
    fn returns_fallback_if_err_result_returned() {
        let mut cmd =
            Command::define_with_fallback(Config::default(), |_| Err(TestError::Internal), |_| 5)
                .unwrap();

        let result = cmd.run(());
        assert_eq!(5, result.unwrap());
    }

    #[test]
    fn returns_fallback_if_circuit_open() {
        let mut cmd = Command::define_with_fallback(
            *Config::default().error_threshold(5),
            |_| Err(TestError::Internal),
            |_| 5,
        ).unwrap();

        for _ in 0..5 {
            let result = cmd.run(());
            assert_eq!(5, result.unwrap()); // Fallback by returned error
        }

        let result = cmd.run(());
        assert_eq!(5, result.unwrap()); // Fallback by reject error
    }

    #[test]
    fn handles_lots_of_calls() {
        let mut cmd = TestCommand::<(), u8>::define(Config::default(), |_| {
            let two_millis = time::Duration::from_millis(2);
            thread::sleep(two_millis);

            return Ok(5);
        }).unwrap();

        let mut results = Vec::new();
        for _ in 0..1000 {
            results.push(cmd.run(()));
        }

        for result in results {
            assert_eq!(5, result.unwrap());
        }
    }
}
