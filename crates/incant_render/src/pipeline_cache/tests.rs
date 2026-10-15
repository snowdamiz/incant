use super::*;
use std::{cell::Cell, io};

#[test]
fn failed_preparation_never_requests_driver_data_or_publishes() {
    let queried = Cell::new(false);
    let error = complete::<()>(
        Err(io::Error::new(io::ErrorKind::InvalidData, "late preparation failure").into()),
        || queried.set(true),
    )
    .unwrap_err();
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(!queried.get());
}

#[test]
fn successful_preparation_publishes_once_and_preserves_the_prepared_value() {
    let calls = Cell::new(0);
    let resources = vec![1, 2, 3];
    let address = resources.as_ptr();
    let prepared = complete(Ok(resources), || calls.set(calls.get() + 1)).unwrap();
    assert_eq!(prepared.as_ptr(), address);
    assert_eq!(prepared, [1, 2, 3]);
    assert_eq!(calls.get(), 1);
}

#[test]
fn inactive_sessions_stay_inert_across_success_failure_and_later_success() {
    for read in [
        CacheRead::Disabled,
        CacheRead::Unsupported,
        CacheRead::UnsupportedStorage,
        CacheRead::NoDirectory,
    ] {
        let mut session = Session::inactive(read.clone());
        session.complete(Ok(())).unwrap();
        assert!(
            session
                .complete::<()>(Err(io::Error::other("refused").into()))
                .is_err()
        );
        session.complete(Ok(())).unwrap();
        assert!(session.cache.is_none());
        assert_eq!(
            session.report,
            PipelineCacheReport {
                read,
                write: CacheWrite::NotAttempted,
                #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
                metal: None
            }
        );
    }
}
