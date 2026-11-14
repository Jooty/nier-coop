use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::enemies::{
    GENERATOR_SET_LINES_LOGGED_PER_MINUTE, GENERATOR_SET_MISMATCH_MUST_LAST_MS,
    LIVE_GENERATOR_IDS_IN_ONE_PART, LIVE_GENERATORS_ARE_TOLD_AGAIN_AFTER_MS,
    MOST_GENERATOR_IDS_IN_ONE_LINE, MOST_LIVE_GENERATOR_PARTS,
};
use crate::minute_line_budget::MinuteLineBudget;
use crate::start::route_flag_label;

const GENERATOR_SET_LINES_TOPIC: &str = "generators: generator set lines";

/// One part of the host's live generator ids, with the route flag its game holds (0: none).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveGenerators {
    pub route_hash: u32,
    pub part: u8,
    pub parts: u8,
    pub ids: Vec<u32>,
}

/// The host's side: when its live set goes to the guest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveGeneratorsPacer {
    told: Option<(u32, BTreeSet<u32>, u64)>,
}

/// The guest's side: the host's parts put together into one set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveGeneratorsAssembly {
    route_hash: u32,
    parts: u8,
    heard: BTreeMap<u8, Vec<u32>>,
}

/// The guest's comparison of its own live generators with the host's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorSetComparison {
    hosts: Option<(u32, BTreeSet<u32>)>,
    differing: Option<(GeneratorSetDifference, u64)>,
    logged: Option<GeneratorSetDifference>,
    budget: MinuteLineBudget,
}

/// Which ids only one copy holds; both empty when the sets are the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorSetDifference {
    pub only_on_the_host: Vec<u32>,
    pub only_here: Vec<u32>,
    pub host_route_hash: u32,
    pub own_route_hash: u32,
    pub count_here: usize,
}

impl LiveGenerators {
    pub fn is_valid(&self) -> bool {
        self.parts >= 1
            && self.parts <= MOST_LIVE_GENERATOR_PARTS
            && self.part < self.parts
            && self.ids.len() <= LIVE_GENERATOR_IDS_IN_ONE_PART
    }
}

impl LiveGeneratorsPacer {
    /// The parts to send now: when the set or the route changed, or the last were told long ago.
    pub fn parts_to_send(
        &mut self,
        now_ms: u64,
        route_hash: u32,
        ids: &BTreeSet<u32>,
    ) -> Vec<LiveGenerators> {
        let due = self.told.as_ref().is_none_or(|(route, told, at_ms)| {
            *route != route_hash
                || told != ids
                || now_ms.saturating_sub(*at_ms) >= LIVE_GENERATORS_ARE_TOLD_AGAIN_AFTER_MS
        });
        if !due {
            return Vec::new();
        }
        self.told = Some((route_hash, ids.clone(), now_ms));
        live_generator_parts(route_hash, ids)
    }

    pub fn forget(&mut self) {
        self.told = None;
    }
}

impl LiveGeneratorsAssembly {
    /// Takes one part; gives the whole set and the route once every part of one telling is in.
    pub fn receive(&mut self, part: LiveGenerators) -> Option<(u32, BTreeSet<u32>)> {
        if part.part == 0 || part.parts != self.parts || part.route_hash != self.route_hash {
            self.heard.clear();
            self.parts = part.parts;
            self.route_hash = part.route_hash;
        }
        self.heard.insert(part.part, part.ids);
        if self.heard.len() < usize::from(self.parts) {
            return None;
        }
        let ids = core::mem::take(&mut self.heard)
            .into_values()
            .flatten()
            .collect();
        Some((self.route_hash, ids))
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }
}

impl Default for GeneratorSetComparison {
    fn default() -> Self {
        Self {
            hosts: None,
            differing: None,
            logged: None,
            budget: MinuteLineBudget::new(GENERATOR_SET_LINES_LOGGED_PER_MINUTE),
        }
    }
}

impl GeneratorSetComparison {
    pub fn hear_the_hosts(&mut self, route_hash: u32, ids: BTreeSet<u32>) {
        self.hosts = Some((route_hash, ids));
    }

    pub fn forget(&mut self) {
        *self = Self {
            budget: self.budget.clone(),
            ..Self::default()
        };
    }

    /// Compares after each read of this copy's generators. A same set is said at once, a
    /// difference only once it has lasted; each result is said once until it changes.
    pub fn note(&mut self, now_ms: u64, own_route_hash: u32, own: &BTreeSet<u32>) -> Vec<String> {
        let Some((host_route_hash, hosts)) = self.hosts.as_ref() else {
            return Vec::new();
        };
        if hosts.is_empty() || own.is_empty() {
            self.differing = None;
            return Vec::new();
        }
        let difference = GeneratorSetDifference {
            only_on_the_host: hosts.difference(own).copied().collect(),
            only_here: own.difference(hosts).copied().collect(),
            host_route_hash: *host_route_hash,
            own_route_hash,
            count_here: own.len(),
        };
        if self.logged.as_ref() == Some(&difference) {
            self.differing = None;
            return Vec::new();
        }
        if !difference.is_same() {
            let since_ms = match &self.differing {
                Some((before, since_ms)) if *before == difference => *since_ms,
                _ => now_ms,
            };
            self.differing = Some((difference.clone(), since_ms));
            if now_ms.saturating_sub(since_ms) < GENERATOR_SET_MISMATCH_MUST_LAST_MS {
                return Vec::new();
            }
        }
        self.differing = None;
        let line = difference.line();
        self.logged = Some(difference);
        self.budget
            .lines_for(now_ms, GENERATOR_SET_LINES_TOPIC, line)
    }
}

impl GeneratorSetDifference {
    pub fn is_same(&self) -> bool {
        self.only_on_the_host.is_empty() && self.only_here.is_empty()
    }

    pub fn line(&self) -> String {
        let routes = if self.host_route_hash == self.own_route_hash {
            format!("{} on both", route_flag_label(self.own_route_hash))
        } else {
            format!(
                "{} on the host, {} here",
                route_flag_label(self.host_route_hash),
                route_flag_label(self.own_route_hash)
            )
        };
        if self.is_same() {
            return format!(
                "generators: same set as the host: {} live generators on both copies ({routes})",
                self.count_here
            );
        }
        format!(
            "generators: mismatch: the copies hold different generators ({routes}): {} only on the host ({}), {} only here ({})",
            self.only_on_the_host.len(),
            ids_named(&self.only_on_the_host),
            self.only_here.len(),
            ids_named(&self.only_here)
        )
    }
}

pub fn live_generator_parts(route_hash: u32, ids: &BTreeSet<u32>) -> Vec<LiveGenerators> {
    let ids: Vec<u32> = ids
        .iter()
        .copied()
        .take(LIVE_GENERATOR_IDS_IN_ONE_PART * usize::from(MOST_LIVE_GENERATOR_PARTS))
        .collect();
    let chunks: Vec<&[u32]> = if ids.is_empty() {
        vec![&[]]
    } else {
        ids.chunks(LIVE_GENERATOR_IDS_IN_ONE_PART).collect()
    };
    let parts = u8::try_from(chunks.len()).unwrap_or(MOST_LIVE_GENERATOR_PARTS);
    chunks
        .into_iter()
        .enumerate()
        .map(|(part, chunk)| LiveGenerators {
            route_hash,
            part: u8::try_from(part).unwrap_or(u8::MAX),
            parts,
            ids: chunk.to_vec(),
        })
        .collect()
}

fn ids_named(ids: &[u32]) -> String {
    if ids.is_empty() {
        return "none".to_owned();
    }
    let named: Vec<String> = ids
        .iter()
        .take(MOST_GENERATOR_IDS_IN_ONE_LINE)
        .map(|id| format!("{id:X}"))
        .collect();
    let more = ids.len().saturating_sub(MOST_GENERATOR_IDS_IN_ONE_LINE);
    if more == 0 {
        named.join(" ")
    } else {
        format!("{} and {more} more", named.join(" "))
    }
}
