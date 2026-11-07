const MILLIS_PER_SECOND: u64 = 1000;
const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_MINUTE: u64 = 60;
const DAYS_FROM_CIVIL_EPOCH_SHIFT: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;

pub fn time_prefix(unix_millis: u64) -> String {
    let (hour, minute, second) = clock_parts(unix_millis);
    format!(
        "{hour:02}:{minute:02}:{second:02}.{:03} ",
        unix_millis % MILLIS_PER_SECOND
    )
}

pub fn date_time_text(unix_millis: u64) -> String {
    let (hour, minute, second) = clock_parts(unix_millis);
    let (year, month, day) = civil_date(unix_millis / MILLIS_PER_SECOND / SECONDS_PER_DAY);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
}

fn clock_parts(unix_millis: u64) -> (u64, u64, u64) {
    let second_of_day = unix_millis / MILLIS_PER_SECOND % SECONDS_PER_DAY;
    (
        second_of_day / SECONDS_PER_HOUR,
        second_of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        second_of_day % SECONDS_PER_MINUTE,
    )
}

fn civil_date(days_since_unix_epoch: u64) -> (i64, i64, i64) {
    let shifted = i64::try_from(days_since_unix_epoch).unwrap_or(0) + DAYS_FROM_CIVIL_EPOCH_SHIFT;
    let era = shifted / DAYS_PER_ERA;
    let day_of_era = shifted - era * DAYS_PER_ERA;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}
