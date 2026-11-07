use rc_game::log_stamp::{
    date_time_text, header_line, role_of_log_name, stamped_line, time_prefix,
};

const LEAP_DAY_2024_AT_NOON_MILLIS: u64 = 1_709_207_045_678;

#[test]
fn the_prefix_is_utc_time_with_milliseconds() {
    assert_eq!(time_prefix(0), "00:00:00.000 ");
    assert_eq!(time_prefix(LEAP_DAY_2024_AT_NOON_MILLIS), "11:44:05.678 ");
}

#[test]
fn the_date_text_is_the_full_utc_date() {
    assert_eq!(date_time_text(0), "1970-01-01 00:00:00 UTC");
    assert_eq!(
        date_time_text(LEAP_DAY_2024_AT_NOON_MILLIS),
        "2024-02-29 11:44:05 UTC"
    );
}

#[test]
fn a_stamped_line_is_the_prefix_then_the_message() {
    assert_eq!(stamped_line(1500, "story: x"), "00:00:01.500 story: x");
}

#[test]
fn the_header_names_date_build_and_role() {
    assert_eq!(
        header_line(0, "abc123", "host"),
        "log opened 1970-01-01 00:00:00 UTC, build abc123, role host"
    );
}

#[test]
fn the_role_comes_from_the_log_file_name() {
    assert_eq!(role_of_log_name("Z:\\logs\\game-host.log"), "host");
    assert_eq!(role_of_log_name("/x/launcher-guest.log"), "guest");
    assert_eq!(role_of_log_name("launcher.log"), "unknown");
}
