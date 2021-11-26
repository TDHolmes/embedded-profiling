#[cfg(test)]
mod test {
    struct TestEP {
        expected_fn_name: Option<String>,
    }

    impl TestEP {
        pub fn new() -> TestEP {
            TestEP {
                expected_fn_name: None,
            }
        }

        pub fn set_expected_fn_name(&mut self, new_name: &str) {
            self.expected_fn_name = Some(new_name.to_string());
        }
    }

    impl embedded_profiling::EmbeddedProfiler for TestEP {
        fn read_clock(&self) -> embedded_profiling::EPInstant {
            embedded_profiling::EPInstant::from_ticks(0)
        }

        fn log_snapshot(&self, snapshot: &embedded_profiling::EPSnapshot) {
            if let Some(expected_name) = &self.expected_fn_name {
                eprintln!("{:?} == {:?} ?", expected_name, snapshot.name);
                assert_eq!(expected_name, snapshot.name);
            } else {
                panic!("log_snapshot called without an expected fn name");
            }
        }
    }

    use std::sync::Once;

    static INIT_PROFILER: Once = Once::new();
    static mut TEST_PROFILER: Option<TestEP> = None;

    fn set_profiler() {
        INIT_PROFILER.call_once(|| unsafe {
            if TEST_PROFILER.is_none() {
                TEST_PROFILER = Some(TestEP::new());
            }
            embedded_profiling::set_profiler(TEST_PROFILER.as_ref().unwrap()).unwrap();
        });
    }

    /// super unsafe unless our tests are run serially, which we should do anyways
    fn set_expected_fn_name(expected_name: &str) {
        unsafe {
            TEST_PROFILER
                .as_mut()
                .unwrap()
                .set_expected_fn_name(expected_name);
        }
    }

    #[test]
    #[serial_test::serial]
    fn profiled_function_matches() {
        #[embedded_profiling_proc_macros::profile_function]
        fn function_to_profile() {}

        set_profiler();
        set_expected_fn_name("function_to_profile");

        function_to_profile();
    }
}
