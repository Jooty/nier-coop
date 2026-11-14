#[cfg(windows)]
pub fn log_readiness(message: &str) {
    crate::diagnostics::log_line(message);
}

#[cfg(not(windows))]
pub fn log_readiness(_message: &str) {}
