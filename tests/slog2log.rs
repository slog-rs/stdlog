use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use slog::Drain;
use log::RecordBuilder;
use fragile::Fragile;

struct StdLogAssertExpected<'a> {
    expected: Mutex<Vec<Fragile<log::Record<'a>>>>,
    current_index: AtomicUsize,
}
impl log::Log for StdLogAssertExpected<'_> {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, actual: &log::Record<'_>) {
        let expected = {
            let expected = self.expected.lock().unwrap();
            // NOTE: I think load fence is implied by the lock
            let old_index = self.current_index.load(Ordering::Relaxed);
            match expected.get(old_index) {
                Some(e) => {
                    assert_eq!(
                        old_index,
                        // Do we need a store fence, or is that implied as well?
                        self.current_index.fetch_add(1, Ordering::Acquire)
                    );
                    e.get().clone()
                },
                None => panic!("Expected no more log records. but got {:?}", actual)
            }
        };
        assert_eq!(expected.metadata(), actual.metadata());
        assert_eq!(expected.args().to_string(), actual.args().to_string());
        assert_eq!(expected.level(), actual.level());
        assert_eq!(expected.target(), actual.target());
        assert_eq!(expected.module_path(), actual.module_path());
        assert_eq!(expected.file(), actual.file());
        // NOTE: Intentionally ignored `line`
        if cfg!(feature = "kv_unstable") {
            todo!("Structure not currently used. See PR #26");
        }
    }
    fn flush(&self) {}
}
impl StdLogAssertExpected<'_> {
    fn assert_finished(&self) {
        let expected = self.expected.lock().unwrap();
        // load fence implied (I think)
        let remaining = expected.len() - self.current_index.load(Ordering::Relaxed);
        assert!(
            remaining == 0,
            "Expected {remaining} more values (first={first:?}) {maybeLast}",
            first = expected.first().unwrap(),
            maybeLast = if remaining >= 2 {
                format!("(last={:?})", expected.last().unwrap())
            } else {
                String::new()
            }
        );
    }
}

macro_rules! record {
    ($level:ident, $fmt:expr) => {
        RecordBuilder::new()
            .args(format_args!($fmt))
            .level(log::Level::$level)
            .file(Some(file!()))
            .module_path(Some(module_path!()))
            .target(module_path!())
            .build()
    }
}

#[test]
fn test_slog2log() {
    let expected = vec![
        record!(Info, "Hello World!"),
        record!(Debug, "Hello World, I am 100 years old")
    ].into_iter().map(Fragile::new).collect::<Vec<Fragile<log::Record>>>();
    let std_logger = Box::leak(Box::new(StdLogAssertExpected {
        expected: Mutex::new(expected),
        current_index: 0.into(),
    }));
    log::set_logger(std_logger).unwrap();
    let sl = slog::Logger::root(slog_stdlog::StdLog.fuse(), slog::o!());
    slog::info!(sl, "Hello {}", "World!");
    slog::debug!(sl, "Hello {}, I am {} years old", "World", 100);
    std_logger.assert_finished();
}
