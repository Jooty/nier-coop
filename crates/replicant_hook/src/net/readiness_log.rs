#[cfg(windows)]
pub(super) fn log_readiness(message: &str) {
    crate::diagnostics::log_line(message);
}

#[cfg(not(windows))]
pub(super) fn log_readiness(_message: &str) {}
