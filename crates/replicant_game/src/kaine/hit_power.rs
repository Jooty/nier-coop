use crate::dialogue::GAME_STATE_RVA;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_PLAYER_PARAM_POINTER_OFFSET, ATTACK_PARAM_MINIMUM_DAMAGE_OFFSETS,
    ATTACK_RECORD_PARAM_RECORD_POINTER_OFFSET, ATTACK_SOURCE_CLASSES_POWERED_BY_WEAPON,
    ENTITY_LEVEL_OFFSET, ENTITY_RVA, FORCED_WEAPON_ID, GAME_STATE_FIXED_PLAYER_LEVEL_OFFSET,
    MAX_WEAPON_LEVEL_STORED, PLAYER_PARAM_CACHED_ATTACK_OFFSET, SAVE_EQUIPPED_WEAPON_ID_OFFSET,
    SAVE_FORCED_WEAPON_FLAG_OFFSET, SAVE_WEAPON_LEVELS_OFFSET,
    WEAPON_RECORD_HP_ATTACK_PERCENT_OFFSETS, WEAPON_RECORD_SECOND_GAUGE_PERCENT_OFFSETS,
    WEAPON_RECORD_THIRD_GAUGE_PERCENT_OFFSETS, WEAPON_TABLE_LEN, WEAPON_TABLE_RVA,
};

use super::{KAINE_LEVEL_SAVE_OFFSET, KAINE_PLAYER_FLAG_SET};

pub const PERCENT_PER_WHOLE: f32 = 100.0;
pub const SWAPPED_POWER_LINE_INTERVAL_MS: u64 = 1000;

/// A weapon record's attack percentages, one per upgrade level `0..=3`, for the three numbers
/// of a swing: HP damage, the second gauge and the third gauge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeaponPercents {
    pub hp: [i32; 4],
    pub second_gauge: [i32; 4],
    pub third_gauge: [i32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldWeapon {
    pub id: i32,
    pub level: i8,
    pub percents: WeaponPercents,
}

/// What the swapped body's swings are worked out from, next to what the first body would use
/// at the same level: the save's own equipped weapon at its own upgrade level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwappedBodyPower {
    pub level_in_use: i32,
    pub first_body_level: i32,
    pub base_attack: Option<i32>,
    pub fixed_level: bool,
    pub worn: HeldWeapon,
    pub first: HeldWeapon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SwappedHitTally {
    pub hits: u32,
    pub game_hp: i32,
    pub sent_hp: i32,
}

#[derive(Debug, Default)]
pub struct SwappedPowerLog {
    last_line_ms: Option<u64>,
    pending: SwappedHitTally,
}

impl WeaponPercents {
    pub fn read<M: GameMemory>(memory: &M, weapon_id: i32) -> Option<Self> {
        if !(0..WEAPON_TABLE_LEN).contains(&weapon_id) {
            return None;
        }
        let slot = u64::try_from(weapon_id).ok()? * 8;
        let record = memory
            .read_u64(memory.module_base() + u64::from(WEAPON_TABLE_RVA) + slot)
            .ok()
            .filter(|record| *record != 0)?;
        let row = |offsets: [u64; 4]| -> Option<[i32; 4]> {
            let mut row = [0; 4];
            for (value, offset) in row.iter_mut().zip(offsets) {
                *value = memory.read_i32(record + offset).ok()?;
            }
            Some(row)
        };
        Some(Self {
            hp: row(WEAPON_RECORD_HP_ATTACK_PERCENT_OFFSETS)?,
            second_gauge: row(WEAPON_RECORD_SECOND_GAUGE_PERCENT_OFFSETS)?,
            third_gauge: row(WEAPON_RECORD_THIRD_GAUGE_PERCENT_OFFSETS)?,
        })
    }
}

impl HeldWeapon {
    pub fn read<M: GameMemory>(memory: &M, weapon_id: i32) -> Option<Self> {
        let save = memory.module_base() + u64::from(ENTITY_RVA);
        let level_at = save + SAVE_WEAPON_LEVELS_OFFSET + u64::try_from(weapon_id).ok()?;
        Some(Self {
            id: weapon_id,
            level: memory.read_u8(level_at).ok()?.cast_signed(),
            percents: WeaponPercents::read(memory, weapon_id)?,
        })
    }

    /// The factors the game's three attack getters multiply the body's base stats by, in the
    /// record's order (HP damage, second gauge, third gauge).
    pub fn factors(&self, fixed_level: bool) -> [f32; 3] {
        let hp_level = if fixed_level { 0 } else { self.level };
        [
            weapon_factor(self.percents.hp, hp_level),
            weapon_factor(self.percents.second_gauge, self.level),
            weapon_factor(self.percents.third_gauge, self.level),
        ]
    }

    pub fn percents_at_level(&self) -> [i32; 3] {
        let index = level_index(self.level).unwrap_or(0);
        [
            self.percents.hp[index],
            self.percents.second_gauge[index],
            self.percents.third_gauge[index],
        ]
    }
}

impl SwappedBodyPower {
    /// `None` unless the save's swapped-body flag is set, so the forced weapon is in play.
    pub fn read<M: GameMemory>(memory: &M, player: u64) -> Option<Self> {
        let base = memory.module_base();
        let save = base + u64::from(ENTITY_RVA);
        if memory
            .read_i32(save + SAVE_FORCED_WEAPON_FLAG_OFFSET)
            .ok()?
            != KAINE_PLAYER_FLAG_SET
        {
            return None;
        }
        let first_weapon = memory
            .read_i32(save + SAVE_EQUIPPED_WEAPON_ID_OFFSET)
            .ok()?;
        let fixed_level = memory
            .read_i32(base + u64::from(GAME_STATE_RVA) + GAME_STATE_FIXED_PLAYER_LEVEL_OFFSET)
            .is_ok_and(|level| level >= 0);
        let base_attack = memory
            .read_u64(player + ACTOR_PLAYER_PARAM_POINTER_OFFSET)
            .ok()
            .filter(|param| *param != 0)
            .and_then(|param| {
                memory
                    .read_i32(param + PLAYER_PARAM_CACHED_ATTACK_OFFSET)
                    .ok()
            });
        Some(Self {
            level_in_use: memory
                .read_i32(save + KAINE_LEVEL_SAVE_OFFSET as u64)
                .ok()?,
            first_body_level: memory
                .read_i32(save + u64::from(ENTITY_LEVEL_OFFSET))
                .ok()?,
            base_attack,
            fixed_level,
            worn: HeldWeapon::read(memory, FORCED_WEAPON_ID)?,
            first: HeldWeapon::read(memory, first_weapon)?,
        })
    }

    /// First-body factor over worn factor, per record number; `1.0` where the worn weapon adds
    /// nothing to divide by.
    pub fn scale(&self) -> [f32; 3] {
        let worn = self.worn.factors(self.fixed_level);
        let first = self.first.factors(self.fixed_level);
        let mut scale = [1.0; 3];
        for ((scale, worn), first) in scale.iter_mut().zip(worn).zip(first) {
            if worn > 0.0 {
                *scale = first / worn;
            }
        }
        scale
    }

    /// The record's numbers as the first body's weapon would have made them: each scaled number
    /// still gets the attack's own minimum, as the game's attack query does.
    pub fn scaled(&self, numbers: [i32; 3], minimums: Option<[i32; 3]>) -> [i32; 3] {
        let scale = self.scale();
        let minimums = minimums.unwrap_or([0; 3]);
        let mut scaled = [0; 3];
        for (index, out) in scaled.iter_mut().enumerate() {
            *out = scaled_number(numbers[index], scale[index], minimums[index]);
        }
        scaled
    }

    pub fn attack_in_use(&self) -> Option<i32> {
        self.base_attack
            .map(|base| scaled_number(base, self.worn.factors(self.fixed_level)[0], 0))
    }

    pub fn first_body_attack(&self) -> Option<i32> {
        self.base_attack
            .map(|base| scaled_number(base, self.first.factors(self.fixed_level)[0], 0))
    }
}

impl SwappedPowerLog {
    pub const fn new() -> Self {
        Self {
            last_line_ms: None,
            pending: SwappedHitTally {
                hits: 0,
                game_hp: 0,
                sent_hp: 0,
            },
        }
    }

    /// Counts one weapon hit; returns the hits since the last line once a line is due.
    pub fn note_hit(&mut self, now_ms: u64, game_hp: i32, sent_hp: i32) -> Option<SwappedHitTally> {
        self.pending.hits = self.pending.hits.saturating_add(1);
        self.pending.game_hp = self.pending.game_hp.saturating_add(game_hp);
        self.pending.sent_hp = self.pending.sent_hp.saturating_add(sent_hp);
        if self
            .last_line_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < SWAPPED_POWER_LINE_INTERVAL_MS)
        {
            return None;
        }
        self.last_line_ms = Some(now_ms);
        Some(core::mem::take(&mut self.pending))
    }
}

pub fn weapon_powered_source(source_class: i32) -> bool {
    ATTACK_SOURCE_CLASSES_POWERED_BY_WEAPON.contains(&source_class)
}

/// The game's attack getters: a level below 0 reads as level 0, and a level past the stored
/// range leaves the weapon out (factor 1).
pub fn weapon_factor(percents: [i32; 4], level: i8) -> f32 {
    level_index(level).map_or(1.0, |index| percents[index] as f32 / PERCENT_PER_WHOLE)
}

pub fn read_minimum_damages<M: GameMemory>(memory: &M, record: u64) -> Option<[i32; 3]> {
    let param = memory
        .read_u64(record + ATTACK_RECORD_PARAM_RECORD_POINTER_OFFSET)
        .ok()
        .filter(|param| *param != 0)?;
    let mut minimums = [0; 3];
    for (value, offset) in minimums.iter_mut().zip(ATTACK_PARAM_MINIMUM_DAMAGE_OFFSETS) {
        *value = memory.read_i32(param + offset).ok()?;
    }
    Some(minimums)
}

pub fn swapped_power_line(power: &SwappedBodyPower, tally: SwappedHitTally) -> String {
    let shown =
        |value: Option<i32>| value.map_or_else(|| "?".to_owned(), |value| value.to_string());
    let [worn_hp, worn_second, worn_third] = power.worn.percents_at_level();
    let [first_hp, first_second, first_third] = power.first.percents_at_level();
    let [scale_hp, scale_second, scale_third] = power.scale();
    format!(
        "swapped body power: level {} in use (first body {}), base attack {}, fixed level {}, worn weapon {} lv {} at {worn_hp}/{worn_second}/{worn_third}%, first body weapon {} lv {} at {first_hp}/{first_second}/{first_third}%, attack {} in use, {} on the first body, hits x{scale_hp:.3}/x{scale_second:.3}/x{scale_third:.3}; {} weapon hits since the last line, hp {} from the game, {} sent",
        power.level_in_use,
        power.first_body_level,
        shown(power.base_attack),
        if power.fixed_level { "on" } else { "off" },
        power.worn.id,
        power.worn.level,
        power.first.id,
        power.first.level,
        shown(power.attack_in_use()),
        shown(power.first_body_attack()),
        tally.hits,
        tally.game_hp,
        tally.sent_hp,
    )
}

fn level_index(level: i8) -> Option<usize> {
    if level > MAX_WEAPON_LEVEL_STORED {
        return None;
    }
    usize::try_from(level.max(0)).ok()
}

fn scaled_number(number: i32, scale: f32, minimum: i32) -> i32 {
    let scaled = (number as f32 * scale).round() as i32;
    scaled.max(minimum)
}
