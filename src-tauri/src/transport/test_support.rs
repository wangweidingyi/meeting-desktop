use std::sync::{Mutex, MutexGuard, OnceLock};

static NETWORK_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn lock_network_test() -> MutexGuard<'static, ()> {
    NETWORK_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}
