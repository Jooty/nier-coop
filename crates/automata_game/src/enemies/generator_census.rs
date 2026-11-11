use std::collections::{BTreeMap, BTreeSet};

use crate::enemies::{
    GENERATOR_LINES_LOGGED_AT_MOST, GeneratorHeader, GeneratorPhase,
    MOST_GENERATORS_NAMED_IN_ONE_LINE,
};

/// What one copy knew of its generators at the last read, and the log lines a new read gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorCensus {
    known: BTreeMap<u32, GeneratorPhase>,
    shared: BTreeSet<u32>,
    lines_left: u32,
}

impl Default for GeneratorCensus {
    fn default() -> Self {
        Self {
            known: BTreeMap::new(),
            shared: BTreeSet::new(),
            lines_left: GENERATOR_LINES_LOGGED_AT_MOST,
        }
    }
}

impl GeneratorCensus {
    pub fn note(&mut self, generators: &[GeneratorHeader]) -> Vec<String> {
        let shared = generators_with_a_shared_id(generators);
        let now: BTreeMap<u32, &GeneratorHeader> = generators
            .iter()
            .filter(|generator| !shared.contains(&generator.action_id))
            .map(|generator| (generator.action_id, generator))
            .collect();
        let mut lines = Vec::new();
        let seen: Vec<String> = now
            .values()
            .filter(|generator| !self.known.contains_key(&generator.action_id))
            .map(|generator| label(generator))
            .collect();
        lines.extend(named_line("seen", &seen));
        let gone: Vec<String> = self
            .known
            .keys()
            .filter(|id| !now.contains_key(id))
            .map(|id| format!("generator {id:#X}"))
            .collect();
        lines.extend(named_line("gone", &gone));
        for generator in now.values() {
            let before = self.known.get(&generator.action_id);
            if before.is_some_and(|before| *before != generator.phase()) {
                lines.push(format!("generators: now {}", label(generator)));
            }
        }
        if shared != self.shared && !shared.is_empty() {
            let ids: Vec<String> = shared.iter().map(|id| format!("{id:#X}")).collect();
            lines.extend(named_line(
                "more than one live generator has the id (or none), they are left to each game:",
                &ids,
            ));
        }
        self.known = now
            .into_iter()
            .map(|(id, generator)| (id, generator.phase()))
            .collect();
        self.shared = shared;
        self.within_the_line_limit(lines)
    }

    fn within_the_line_limit(&mut self, mut lines: Vec<String>) -> Vec<String> {
        let allowed = self.lines_left as usize;
        if lines.len() > allowed {
            lines.truncate(allowed);
            if self.lines_left > 0 {
                lines.push("generators: no more generator lines this session".to_owned());
            }
            self.lines_left = 0;
        } else {
            self.lines_left -= lines.len() as u32;
        }
        lines
    }
}

/// The ids no generator can be named by: carried by more than one live generator, or zero.
pub fn generators_with_a_shared_id(generators: &[GeneratorHeader]) -> BTreeSet<u32> {
    let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
    for generator in generators {
        *counts.entry(generator.action_id).or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|(id, count)| *id == 0 || *count > 1)
        .map(|(id, _)| id)
        .collect()
}

fn label(generator: &GeneratorHeader) -> String {
    format!(
        "generator {:#X} ({}, {} of {} made, at most {} alive, {} slots, {} sets)",
        generator.action_id,
        generator.phase().label(),
        generator.spawned,
        generator.enemies_of_one_run,
        generator.most_enemies_alive,
        generator.slot_count,
        generator.set_count
    )
}

fn named_line(what: &str, labels: &[String]) -> Option<String> {
    if labels.is_empty() {
        return None;
    }
    let named = labels
        .iter()
        .take(MOST_GENERATORS_NAMED_IN_ONE_LINE)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    let more = labels
        .len()
        .saturating_sub(MOST_GENERATORS_NAMED_IN_ONE_LINE);
    Some(if more == 0 {
        format!("generators: {what} {named}")
    } else {
        format!("generators: {what} {named}; and {more} more")
    })
}
