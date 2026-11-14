use crate::event_follow::EventProgress;

pub fn event_progress_line(
    progress: &EventProgress,
    since_start_ms: Option<u64>,
    host_start_earlier_by_ms: Option<i64>,
) -> String {
    let since = since_start_ms.map_or_else(
        || "start here not seen".to_owned(),
        |ms| format!("{ms} ms since start here"),
    );
    let gap = match host_start_earlier_by_ms {
        Some(gap) if gap >= 0 => format!("the host's start was {gap} ms earlier"),
        Some(gap) => format!("the host's start was {} ms later", gap.unsigned_abs()),
        None => "the other copy's start of it is not known".to_owned(),
    };
    format!(
        "event: {} at frame {} of {}, paused {}, {since}, {gap}",
        progress.id.describe(),
        progress.frame,
        progress.last_frame,
        if progress.paused { "yes" } else { "no" },
    )
}

pub fn outlast_cut_line(progress: &EventProgress) -> String {
    format!(
        "event: {} is cut by the outlast rule at frame {} of {}, paused {}",
        progress.id.describe(),
        progress.frame,
        progress.last_frame,
        if progress.paused { "yes" } else { "no" },
    )
}
