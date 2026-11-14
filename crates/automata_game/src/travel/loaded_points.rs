use crate::travel::{MAX_POINT_NAMES_IN_ONE_LOG_LINE, TravelPoint};

/// The known destination codes among the travel points the game holds right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LoadedPoints {
    bits: u32,
}

impl LoadedPoints {
    pub fn of_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let bits = names
            .into_iter()
            .filter_map(TravelPoint::from_code)
            .fold(0, |bits, point| bits | 1 << point.index());
        Self { bits }
    }

    pub fn contains(self, point: TravelPoint) -> bool {
        self.bits & (1 << point.index()) != 0
    }

    pub fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub fn points(self) -> Vec<TravelPoint> {
        TravelPoint::all()
            .filter(|point| self.contains(*point))
            .collect()
    }
}

pub fn point_list_log_line(names: &[String]) -> String {
    let (known, other): (Vec<&str>, Vec<&str>) = names
        .iter()
        .map(String::as_str)
        .partition(|name| TravelPoint::from_code(name).is_some());
    format!(
        "travel: the game's point list holds {} travel points ({}) and {} other points ({}); reading this list is a guess until run",
        known.len(),
        joined(&known),
        other.len(),
        joined(&other)
    )
}

fn joined(names: &[&str]) -> String {
    if names.is_empty() {
        return "none".to_owned();
    }
    let shown = names
        .iter()
        .take(MAX_POINT_NAMES_IN_ONE_LOG_LINE)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    if names.len() > MAX_POINT_NAMES_IN_ONE_LOG_LINE {
        format!("{shown}, ...")
    } else {
        shown
    }
}
