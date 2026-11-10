mod clock;
mod header;

pub use clock::{date_time_text, time_prefix};
pub use header::{header_line, role_of_log_name, stamped_line, unix_millis_now};
