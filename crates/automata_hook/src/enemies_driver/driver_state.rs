use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Mutex;
use std::time::Instant;

use automata_game::enemies::{
    Census, DefeatWatch, EnemyHit, EnemyId, EnemySighting, GeneratorCensus, GeneratorSetComparison,
    GeneratorState, HitLedger, HostSpawns, LiveGeneratorsAssembly, LiveGeneratorsPacer,
    MismatchWatch, RosterAssembly, RosterPacer,
};
use automata_game::save_redirect::SaveRole;

use crate::game_calls::GameCalls;

pub(super) static DRIVER: Mutex<Option<EnemiesDriver>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct CensusSummary {
    pub lists: usize,
    pub enemies: usize,
    pub with_the_base_damage_code: usize,
    pub without_the_entry_id_on_the_body: usize,
    pub objects_without_an_enemys_health: usize,
}

pub(super) struct EnemiesDriver {
    pub calls: GameCalls,
    pub role: SaveRole,
    pub health_is_shared: bool,
    pub started: Instant,
    pub last_census_ms: Option<u64>,
    pub census: Census,
    pub pacer: RosterPacer,
    pub assembly: RosterAssembly,
    pub other_roster: Option<Vec<EnemySighting>>,
    pub mismatches: MismatchWatch,
    pub mismatch_lines_left: u32,
    pub was_connected: bool,
    pub summary_logged: Option<(CensusSummary, u64)>,
    pub defeats: DefeatWatch,
    pub defeats_are_told_again: bool,
    pub hits_heard: Vec<EnemyHit>,
    pub own_hits: HitLedger,
    pub own_hit_at_ms: HashMap<EnemyId, u64>,
    pub handles_defeated_here: HashSet<u32>,
    pub objects_with_their_own_damage_code: HashSet<u32>,
    pub share_lines_left: u32,
    pub entity_table_read_at_ms: Option<u64>,
    pub unplaced_logged: Option<(usize, usize)>,
    pub generators_are_hooked: bool,
    pub spawns_follow_the_host: bool,
    pub host_spawns: HostSpawns,
    pub generator_census: GeneratorCensus,
    pub generator_states: Vec<GeneratorState>,
    pub generators_read_at_ms: Option<u64>,
    pub generators_are_held: bool,
    pub living_spawns_are_told_again: bool,
    pub generators_left_to_the_game: BTreeSet<u32>,
    pub spawn_lines_left: u32,
    pub generators_live_here: BTreeSet<u32>,
    pub spawns_waiting_without_a_generator_since_ms: HashMap<EnemyId, u64>,
    pub enemy_reads_asked: HashMap<EnemyId, (u32, u32)>,
    pub route_hash_here: u32,
    pub live_generators_pacer: LiveGeneratorsPacer,
    pub live_generators_assembly: LiveGeneratorsAssembly,
    pub generator_set_comparison: GeneratorSetComparison,
    pub group_words_logged: BTreeSet<EnemyId>,
}
