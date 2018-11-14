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
    enum TestError { Internal, External }
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

    impl From<CriusError> for TestError {
        fn from(_: CriusError) -> Self { TestError::External }
    }

    type TestCommand<I, O> = Command<I, O, TestError,
                                     fn(I) -> Result<O, TestError>,
                                     fn(TestError) -> O>;

    #[test]
    fn runs_command() {
        let rx = TestCommand::<(), u8>::define(|_| Ok(5)).create().run(());
        assert_eq!(5, rx.recv().unwrap().unwrap());
    }

    #[test]
    fn runs_command_multiple_times() {
        let mut cmd = TestCommand::<(), u8>::define(|_| {
            return Ok(5)
        }).create();

        for _ in 0..5 {
            let rx = cmd.run(());
            assert_eq!(5, rx.recv().unwrap().unwrap());
        }
    }

    #[test]
    fn runs_command_with_param() {
        let rx = TestCommand::<u8, u8>::define(|param| Ok(param))
            .create().run(5);

        assert_eq!(5, rx.recv().unwrap().unwrap());
    }

    #[test]
    fn rejects_command_if_circuit_open() {
        let mut cmd = TestCommand::<(), ()>::define(|_| {
            Err(TestError::Internal)
        }).config(*Config::new().error_threshold(5)).create();

        for _ in 0..5 {
            let rx = cmd.run(());
            let err = rx.recv().unwrap().unwrap_err();
            assert_eq!(TestError::Internal, err); // Fallback by returned error
        }

        let rx = cmd.run(());
        let err = rx.recv().unwrap().unwrap_err();
        assert_eq!(TestError::External, err); // Fallback by reject error
    }

    #[test]
    fn returns_fallback_if_err_result_returned() {
        let mut cmd = Command::define_with_fallback(|_| {
            return Err(TestError::Internal)
        }, |_| {
            return 5;
        }).create();

        let rx = cmd.run(());
        assert_eq!(5, rx.recv().unwrap().unwrap());
    }

    #[test]
    fn returns_fallback_if_circuit_open() {
        let mut cmd = Command::define_with_fallback(|_| {
            return Err(TestError::Internal)
        }, |_| {
            return 5;
        }).config(*Config::new().error_threshold(5)).create();

        for _ in 0..5 {
            let rx = cmd.run(());
            assert_eq!(5, rx.recv().unwrap().unwrap()); // Fallback by returned error
        }

        let rx = cmd.run(());
        assert_eq!(5, rx.recv().unwrap().unwrap()); // Fallback by reject error
    }

    #[test]
    fn handles_lots_of_calls() {
        let mut cmd = TestCommand::<(), u8>::define(|_| {
            let ten_millis = time::Duration::from_millis(10);
            thread::sleep(ten_millis);

            return Ok(5)
        }).create();

        let mut rxs = Vec::new();
        for _ in 0..1000 {
            rxs.push(cmd.run(()));
        }

        for rx in rxs {
            assert_eq!(5, rx.recv().unwrap().unwrap());
        }
    }
}
