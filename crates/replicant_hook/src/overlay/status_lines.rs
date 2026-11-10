use std::fmt::Display;

use replicant_game::Replicant;

use crate::memory::InProcessMemory;

pub(super) fn game_status_lines(game: Option<&Replicant<InProcessMemory>>) -> Vec<String> {
    let Some(game) = game else {
        return vec!["game memory not read: build gate failed".to_owned()];
    };
    let position = match game.position() {
        Ok(position) => format!(
            "position: {:.2} {:.2} {:.2}",
            position.x, position.y, position.z
        ),
        Err(error) => format!("position: {error}"),
    };
    vec![
        line("phase", game.current_phase()),
        position,
        pair_line("hp", game.health(), game.max_health()),
        pair_line(
            "mp",
            game.magic().map(|mp| format!("{mp:.1}")),
            game.max_magic().map(|mp| format!("{mp:.1}")),
        ),
        line("level", game.level()),
    ]
}

fn line<T: Display, E: Display>(label: &str, result: Result<T, E>) -> String {
    match result {
        Ok(value) => format!("{label}: {value}"),
        Err(error) => format!("{label}: {error}"),
    }
}

fn pair_line<T: Display, U: Display, E: Display, F: Display>(
    label: &str,
    current: Result<T, E>,
    max: Result<U, F>,
) -> String {
    let current = current.map_or_else(|error| error.to_string(), |value| value.to_string());
    let max = max.map_or_else(|error| error.to_string(), |value| value.to_string());
    format!("{label}: {current} / {max}")
}
