use std::panic::{self, PanicHookInfo};
use std::thread;

use replicant_game::crash_report::panic_line;

use crate::crash_log::handlers::write_durable_line;

pub(super) fn install() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        write_durable_line(&describe(info));
        previous(info);
    }));
}

fn describe(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a panic with no text");
    let current = thread::current();
    let thread_label = match current.name() {
        Some(name) => format!("{name} {:?}", current.id()),
        None => format!("{:?}", current.id()),
    };
    let location = info
        .location()
        .map(|place| (place.file(), place.line(), place.column()));
    panic_line(&message.replace('\n', " | "), location, &thread_label)
}
