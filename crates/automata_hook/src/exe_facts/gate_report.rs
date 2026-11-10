use automata_game::exe::{ExeFacts, identify};

pub fn gate_report_line(facts: &ExeFacts) -> String {
    match identify(facts) {
        Some(known) => format!(
            "exe variant: {} ({}), size {}, timestamp {:#x}",
            known.variant.label(),
            known.name,
            facts.file_size,
            facts.time_date_stamp
        ),
        None => format!(
            "unknown exe: size {}, timestamp {:#x}, sha256 {}; nothing installed",
            facts.file_size, facts.time_date_stamp, facts.sha256_hex
        ),
    }
}
