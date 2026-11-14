use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::enemies::{
    GENERATOR_COUNT_DIFFERENCE_IS_TOLD_AGAIN_AFTER_MS, GENERATOR_COUNT_LINES_LOGGED_PER_MINUTE,
    GENERATOR_COUNTS_MAY_DIFFER_FOR_MS, GENERATOR_SLOT_STATE_ENEMY_MADE, GeneratorHeader,
    GeneratorPhase, GeneratorSlot, GeneratorState, HIGHEST_GENERATOR_REST_TIME_S_ON_THE_WIRE,
};
use crate::minute_line_budget::MinuteLineBudget;

const GENERATOR_COUNT_LINES_TOPIC: &str = "enemies: generator count lines";

/// What each copy rolls for itself at a generator's run start (run size, most alive, rest time)
/// and the dead count a script reads of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorRunCounts {
    pub enemies_of_one_run: i16,
    pub most_enemies_alive: i16,
    pub rest_time_bits: u32,
    pub dead: i16,
}

/// One copy's own reading of one of its generators; a generator not started has no counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorCountReading {
    pub generator: u32,
    pub phase: GeneratorPhase,
    pub spawned: u64,
    pub counts: Option<GeneratorRunCounts>,
}

/// How the guest's reading of a generator compares with the host's word on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountVerdict {
    Same,
    Differ,
    StartedOnlyOnTheHost,
    HostSaidNothing,
    NeitherStarted,
}

/// The guest's log of its generators' counts beside the host's: a line on every change, and a
/// line a second while they differ for longer than `GENERATOR_COUNTS_MAY_DIFFER_FOR_MS`.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorCountWatch {
    seen: BTreeMap<u32, CountsSeen>,
    budget: MinuteLineBudget,
}

impl Default for GeneratorCountWatch {
    fn default() -> Self {
        Self {
            seen: BTreeMap::new(),
            budget: MinuteLineBudget::new(GENERATOR_COUNT_LINES_LOGGED_PER_MINUTE),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CountsSeen {
    reading: GeneratorCountReading,
    host: Option<GeneratorState>,
    differ_since_ms: Option<u64>,
    told_again_at_ms: Option<u64>,
}

impl GeneratorRunCounts {
    /// None while the generator is not started (its counts are not set yet) or a count does not
    /// fit the wire.
    pub fn of_generator(header: &GeneratorHeader, slots: &[GeneratorSlot]) -> Option<Self> {
        let phase = header.phase();
        if phase.is_not_started() {
            return None;
        }
        Some(Self {
            enemies_of_one_run: i16::try_from(header.enemies_of_one_run).ok()?,
            most_enemies_alive: i16::try_from(header.most_enemies_alive).ok()?,
            rest_time_bits: header.rest_time_bits,
            dead: i16::try_from(generator_dead_count(
                phase,
                header.spawned,
                header.enemies_of_one_run,
                slots,
            ))
            .ok()?,
        })
    }

    pub fn rest_time_s(&self) -> f32 {
        f32::from_bits(self.rest_time_bits)
    }

    pub fn is_valid(&self) -> bool {
        let rest_time_s = self.rest_time_s();
        self.enemies_of_one_run >= 0
            && self.most_enemies_alive >= 0
            && rest_time_s.is_finite()
            && (0.0..=HIGHEST_GENERATOR_REST_TIME_S_ON_THE_WIRE).contains(&rest_time_s)
    }

    pub fn label(&self) -> String {
        format!(
            "run of {} (at most {} alive, rest {:.1} s), {} dead",
            self.enemies_of_one_run,
            self.most_enemies_alive,
            self.rest_time_s(),
            self.dead
        )
    }
}

impl GeneratorCountReading {
    pub fn of_generator(header: &GeneratorHeader, slots: &[GeneratorSlot]) -> Self {
        Self {
            generator: header.action_id,
            phase: header.phase(),
            spawned: header.spawned,
            counts: GeneratorRunCounts::of_generator(header, slots),
        }
    }

    pub fn verdict(&self, host: Option<&GeneratorState>) -> CountVerdict {
        let Some(host) = host else {
            return if self.phase.is_not_started() {
                CountVerdict::NeitherStarted
            } else {
                CountVerdict::HostSaidNothing
            };
        };
        match (self.phase.is_not_started(), host.phase.is_not_started()) {
            (true, true) => return CountVerdict::NeitherStarted,
            (true, false) => return CountVerdict::StartedOnlyOnTheHost,
            _ => {}
        }
        if host.phase != self.phase || u64::from(host.spawned) != self.spawned {
            return CountVerdict::Differ;
        }
        match (host.counts, self.counts) {
            (Some(hosts), Some(own))
                if hosts.enemies_of_one_run != own.enemies_of_one_run || hosts.dead != own.dead =>
            {
                CountVerdict::Differ
            }
            _ => CountVerdict::Same,
        }
    }
}

impl CountVerdict {
    pub fn is_a_difference(self) -> bool {
        matches!(self, Self::Differ | Self::StartedOnlyOnTheHost)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Same => "the same",
            Self::Differ => "they differ",
            Self::StartedOnlyOnTheHost => {
                "started on the host but not here: this copy's script has not started it, so nothing is written into it, it is given no slot and the host's spawns for it wait"
            }
            Self::HostSaidNothing => "the host has said nothing on it",
            Self::NeitherStarted => "started on neither copy",
        }
    }
}

impl GeneratorCountWatch {
    /// Takes this copy's readings, each beside the host's word on that generator; generators no
    /// longer read are forgotten.
    pub fn note(
        &mut self,
        now_ms: u64,
        readings: &[(GeneratorCountReading, Option<GeneratorState>)],
    ) -> Vec<String> {
        self.seen.retain(|generator, _| {
            readings
                .iter()
                .any(|(reading, _)| reading.generator == *generator)
        });
        let mut lines = Vec::new();
        for (reading, host) in readings {
            let verdict = reading.verdict(host.as_ref());
            let is_new = !self.seen.contains_key(&reading.generator);
            let seen = self.seen.entry(reading.generator).or_insert(CountsSeen {
                reading: *reading,
                host: *host,
                differ_since_ms: None,
                told_again_at_ms: None,
            });
            let changed = seen.reading != *reading || seen.host != *host;
            if (changed && verdict != CountVerdict::NeitherStarted)
                || (is_new && verdict.is_a_difference())
            {
                lines.push(generator_count_comparison_line(
                    reading,
                    host.as_ref(),
                    verdict,
                ));
            }
            seen.reading = *reading;
            seen.host = *host;
            if !verdict.is_a_difference() {
                seen.differ_since_ms = None;
                seen.told_again_at_ms = None;
                continue;
            }
            let since_ms = *seen.differ_since_ms.get_or_insert(now_ms);
            let for_ms = now_ms.saturating_sub(since_ms);
            let told_again_is_due = seen.told_again_at_ms.is_none_or(|at_ms| {
                now_ms.saturating_sub(at_ms) >= GENERATOR_COUNT_DIFFERENCE_IS_TOLD_AGAIN_AFTER_MS
            });
            if for_ms > GENERATOR_COUNTS_MAY_DIFFER_FOR_MS && told_again_is_due {
                seen.told_again_at_ms = Some(now_ms);
                lines.push(generator_counts_still_differ_line(
                    reading,
                    host.as_ref(),
                    for_ms,
                ));
            }
        }
        lines
            .into_iter()
            .flat_map(|line| {
                self.budget
                    .lines_for(now_ms, GENERATOR_COUNT_LINES_TOPIC, line)
            })
            .collect()
    }

    pub fn forget(&mut self) {
        self.seen.clear();
    }
}

/// The game's `getDeadCount`: the run size once finished, else the enemies made in this run
/// minus the slots still loading or holding a living enemy.
pub fn generator_dead_count(
    phase: GeneratorPhase,
    spawned: u64,
    enemies_of_one_run: i32,
    slots: &[GeneratorSlot],
) -> i32 {
    if phase == GeneratorPhase::Finished {
        return enemies_of_one_run;
    }
    let alive = slots
        .iter()
        .filter(|slot| slot.state <= GENERATOR_SLOT_STATE_ENEMY_MADE)
        .count();
    // The game reads the 64-bit count as a 32-bit int.
    (spawned as i32).wrapping_sub(i32::try_from(alive).unwrap_or(i32::MAX))
}

/// The host's run counts the guest writes into its own generator after its own roll: only when
/// both copies have started the generator and the host sent them.
pub fn host_run_counts_to_write(
    own_phase: GeneratorPhase,
    host: Option<&GeneratorState>,
) -> Option<GeneratorRunCounts> {
    let host = host?;
    if own_phase.is_not_started() || host.phase.is_not_started() {
        return None;
    }
    host.counts.filter(GeneratorRunCounts::is_valid)
}

pub fn generator_count_comparison_line(
    reading: &GeneratorCountReading,
    host: Option<&GeneratorState>,
    verdict: CountVerdict,
) -> String {
    format!(
        "enemies: counts of generator {:#X}: the host's {}; here {} ({})",
        reading.generator,
        hosts_counts_part(host),
        counts_part(reading.phase, reading.spawned, reading.counts),
        verdict.label()
    )
}

pub fn generator_counts_still_differ_line(
    reading: &GeneratorCountReading,
    host: Option<&GeneratorState>,
    for_ms: u64,
) -> String {
    format!(
        "enemies: counts of generator {:#X} still differ after {for_ms} ms (more than {GENERATOR_COUNTS_MAY_DIFFER_FOR_MS} ms): the host's {}; here {} ({})",
        reading.generator,
        hosts_counts_part(host),
        counts_part(reading.phase, reading.spawned, reading.counts),
        reading.verdict(host).label()
    )
}

pub fn generator_takes_the_hosts_run_counts_line(
    own: &GeneratorHeader,
    hosts: &GeneratorRunCounts,
    hosts_spawned: Option<u32>,
) -> String {
    format!(
        "enemies: generator {:#X} takes the host's run counts after its own roll: run of {} (its own {}), at most {} alive (its own {}), rest {:.1} s (its own {:.1} s); its spawned count is {} outside the game's spawn decision",
        own.action_id,
        hosts.enemies_of_one_run,
        own.enemies_of_one_run,
        hosts.most_enemies_alive,
        own.most_enemies_alive,
        hosts.rest_time_s(),
        f32::from_bits(own.rest_time_bits),
        hosts_spawned.map_or(
            "its own (no word from the host yet)".to_owned(),
            |spawned| { format!("the host's {spawned}") }
        )
    )
}

fn hosts_counts_part(host: Option<&GeneratorState>) -> String {
    host.map_or("word: none yet".to_owned(), |host| {
        counts_part(host.phase, u64::from(host.spawned), host.counts)
    })
}

fn counts_part(phase: GeneratorPhase, spawned: u64, counts: Option<GeneratorRunCounts>) -> String {
    format!(
        "{}, {spawned} made, {}",
        phase.label(),
        counts.map_or("no run counts".to_owned(), |counts| counts.label())
    )
}
