use core::ffi::CStr;
use std::collections::HashSet;

use crate::game::GameError;
use crate::memory::GameMemory;

pub const PLAYER_SPELL_EFFECT_PREFIX: &str = "efP_black";
pub const PLAYER_SPELL_KINDS: [&str; 8] = [
    "Bullet",
    "Spear",
    "Execution",
    "Glutton",
    "Hand",
    "Wall",
    "Rotation",
    "Illusion",
];
pub const SWAPPED_BODY_SPELL_TINT_RGB: [f32; 3] = [0.3, 0.55, 1.0];
pub const SWAPPED_SPELL_EFFECT_MAX_LIFE_MS: u64 = 8_000;
pub const MAX_TRACKED_SWAPPED_SPELL_EFFECTS: usize = 256;
pub const SPELL_EFFECT_NAME_READ_LEN: usize = 64;

/// How sure a pair is before a run: `ByRole` when both names say the same part (a shot, its hit),
/// `Guess` when the role of her effect is read from its name only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairBasis {
    ByRole,
    Guess,
}

/// One of the player's spell effects and the effect of hers that is drawn instead on the swapped
/// body. Ids are the effect number table's (`None` where the game spawns the effect by name only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLookPair {
    pub original: &'static str,
    pub original_id: Option<i32>,
    pub hers: &'static CStr,
    pub hers_id: i32,
    pub basis: PairBasis,
}

/// What a spell effect looks like on the swapped body: her effect, the spell's own effect in her
/// colour, or unchanged (not a spell effect, or nothing to swap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLook {
    Hers(&'static SpellLookPair),
    Tinted,
    AsItIs,
}

/// The names already logged, so each spell effect is logged once.
#[derive(Debug, Default)]
pub struct SpellLookLog {
    seen: HashSet<String>,
}

/// Handles of her effects drawn in place of a spell's, with when they started: one that outlives
/// `SWAPPED_SPELL_EFFECT_MAX_LIFE_MS` is ended, in case hers loops where the spell's did not.
#[derive(Debug, Default)]
pub struct SwappedSpellEffects {
    started: Vec<(u32, u64)>,
}

const HER_BALL_EFFECT: &CStr = c"efC_kaine_MagicBall01";
const HER_BALL_EFFECT_ID: i32 = 233;
const HER_BIG_BALL_EFFECT: &CStr = c"efC_kaine_MagicBall02";
const HER_BIG_BALL_EFFECT_ID: i32 = 288;
const HER_BALL_START_EFFECT: &CStr = c"efC_kaine_MagicBallStart02";
const HER_BALL_START_EFFECT_ID: i32 = 289;
const HER_BALL_HIT_EFFECT: &CStr = c"efC_kaine_MagicBallHit01";
const HER_BALL_HIT_EFFECT_ID: i32 = 588;
const HER_BIG_BALL_HIT_EFFECT: &CStr = c"efC_kaine_MagicBallHit02";
const HER_BIG_BALL_HIT_EFFECT_ID: i32 = 290;
const HER_ARRAY_EFFECT: &CStr = c"efC_kaine_MagicArray01";
const HER_ARRAY_EFFECT_ID: i32 = 230;
const HER_ARRAY_START_EFFECT: &CStr = c"efC_kaine_MagicArrayStart01";
const HER_ARRAY_START_EFFECT_ID: i32 = 234;
const HER_MAGIC_HIT_EFFECT: &CStr = c"efC_kaine_MagicHit01";
const HER_MAGIC_HIT_EFFECT_ID: i32 = 9821;
const HER_SHOCK_WAVE_EFFECT: &CStr = c"efC_kaine_ShockWave01";
const HER_SHOCK_WAVE_EFFECT_ID: i32 = 225;
const HER_TRACE_EFFECT: &CStr = c"efC_kaine_Trace01";
const HER_TRACE_EFFECT_ID: i32 = 271;
const HER_TRACE_BIRTH_EFFECT: &CStr = c"efC_kaine_TraceBirth01";
const HER_TRACE_BIRTH_EFFECT_ID: i32 = 272;
const HER_TRACE_START_EFFECT: &CStr = c"efC_kaine_TraceStart01";
const HER_TRACE_START_EFFECT_ID: i32 = 7007;
const HER_TRACE_HIT_EFFECT: &CStr = c"efC_kaine_TraceHit01";
const HER_TRACE_HIT_EFFECT_ID: i32 = 7023;

const fn pair(
    original: &'static str,
    original_id: Option<i32>,
    hers: &'static CStr,
    hers_id: i32,
    basis: PairBasis,
) -> SpellLookPair {
    SpellLookPair {
        original,
        original_id,
        hers,
        hers_id,
        basis,
    }
}

pub const SPELL_LOOK_PAIRS: [SpellLookPair; 54] = [
    pair(
        "efP_blackBulletBullet01",
        Some(47),
        HER_BALL_EFFECT,
        HER_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackBullet1stShot01",
        Some(245),
        HER_BALL_EFFECT,
        HER_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackBulletChargeBullet01",
        Some(51),
        HER_BIG_BALL_EFFECT,
        HER_BIG_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackBulletMuzzle01",
        None,
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackBullet1stShotMuzzle01",
        None,
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackBulletChargeMuzzle01",
        None,
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackBulletHit01",
        Some(192),
        HER_BALL_HIT_EFFECT,
        HER_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackBulletHitWall01",
        Some(193),
        HER_BALL_HIT_EFFECT,
        HER_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackBulletJustHit01",
        Some(9903),
        HER_BALL_HIT_EFFECT,
        HER_BALL_HIT_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackBulletExtinct01",
        Some(197),
        HER_BALL_HIT_EFFECT,
        HER_BALL_HIT_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackBulletChargeHit01",
        Some(9902),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackBulletChargeHitWall01",
        Some(9906),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackBulletChargeExtinct01",
        Some(9905),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackBulletChargeTrail01",
        Some(196),
        HER_TRACE_EFFECT,
        HER_TRACE_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearBullet01",
        None,
        HER_TRACE_EFFECT,
        HER_TRACE_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearBullet11",
        None,
        HER_TRACE_EFFECT,
        HER_TRACE_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearMuzzle01",
        None,
        HER_TRACE_START_EFFECT,
        HER_TRACE_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearMuzzle11",
        None,
        HER_TRACE_START_EFFECT,
        HER_TRACE_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearStagnation01",
        None,
        HER_TRACE_BIRTH_EFFECT,
        HER_TRACE_BIRTH_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearStagnation11",
        None,
        HER_TRACE_BIRTH_EFFECT,
        HER_TRACE_BIRTH_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearHit01",
        None,
        HER_TRACE_HIT_EFFECT,
        HER_TRACE_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackSpearHit11",
        None,
        HER_TRACE_HIT_EFFECT,
        HER_TRACE_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackSpearWallHit01",
        None,
        HER_TRACE_HIT_EFFECT,
        HER_TRACE_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackSpearExtinct01",
        Some(186),
        HER_MAGIC_HIT_EFFECT,
        HER_MAGIC_HIT_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackSpearVanish01",
        None,
        HER_MAGIC_HIT_EFFECT,
        HER_MAGIC_HIT_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackExecution_mc01_st",
        None,
        HER_ARRAY_START_EFFECT,
        HER_ARRAY_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackExecution_mc01",
        None,
        HER_ARRAY_EFFECT,
        HER_ARRAY_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackExecution_mc02",
        None,
        HER_ARRAY_EFFECT,
        HER_ARRAY_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackExecution_mc03",
        None,
        HER_ARRAY_EFFECT,
        HER_ARRAY_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackExecution_mc04",
        None,
        HER_ARRAY_EFFECT,
        HER_ARRAY_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackExecutionHit01",
        None,
        HER_MAGIC_HIT_EFFECT,
        HER_MAGIC_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonMagicCircle01",
        None,
        HER_ARRAY_EFFECT,
        HER_ARRAY_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonMagicCircle02",
        None,
        HER_ARRAY_EFFECT,
        HER_ARRAY_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonMagicCircle03",
        None,
        HER_ARRAY_EFFECT,
        HER_ARRAY_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonMuzzle01",
        Some(9910),
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonMuzzle02",
        Some(9911),
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonMuzzle03",
        Some(9912),
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonMuzzle04",
        Some(9913),
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonMuzzle05",
        Some(9914),
        HER_BALL_START_EFFECT,
        HER_BALL_START_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackGluttonShot01",
        Some(9915),
        HER_BIG_BALL_EFFECT,
        HER_BIG_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShot02",
        Some(9916),
        HER_BIG_BALL_EFFECT,
        HER_BIG_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShot03",
        Some(9917),
        HER_BIG_BALL_EFFECT,
        HER_BIG_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShot04",
        Some(9918),
        HER_BIG_BALL_EFFECT,
        HER_BIG_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShot05",
        Some(9919),
        HER_BIG_BALL_EFFECT,
        HER_BIG_BALL_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShotHit01",
        Some(9924),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShotHit02",
        Some(9925),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShotHit03",
        Some(9926),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShotHit04",
        Some(9927),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackGluttonShotHit05",
        Some(9928),
        HER_BIG_BALL_HIT_EFFECT,
        HER_BIG_BALL_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackHandHit01",
        None,
        HER_MAGIC_HIT_EFFECT,
        HER_MAGIC_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackHandChargeHit01",
        None,
        HER_MAGIC_HIT_EFFECT,
        HER_MAGIC_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackHandJGHit01",
        None,
        HER_MAGIC_HIT_EFFECT,
        HER_MAGIC_HIT_EFFECT_ID,
        PairBasis::ByRole,
    ),
    pair(
        "efP_blackHandWave01",
        None,
        HER_SHOCK_WAVE_EFFECT,
        HER_SHOCK_WAVE_EFFECT_ID,
        PairBasis::Guess,
    ),
    pair(
        "efP_blackHandChargeWave01",
        None,
        HER_SHOCK_WAVE_EFFECT,
        HER_SHOCK_WAVE_EFFECT_ID,
        PairBasis::Guess,
    ),
];

impl SpellLookPair {
    pub fn her_name(&self) -> &'static str {
        self.hers.to_str().unwrap_or_default()
    }
}

impl SpellLookLog {
    pub fn is_first(&mut self, name: &str) -> bool {
        self.seen.insert(name.to_owned())
    }
}

impl SwappedSpellEffects {
    pub fn note(&mut self, handle: u32, now_ms: u64) {
        if self.started.len() >= MAX_TRACKED_SWAPPED_SPELL_EFFECTS {
            self.started.remove(0);
        }
        self.started.push((handle, now_ms));
    }

    pub fn take_overdue(&mut self, now_ms: u64) -> Vec<u32> {
        let (overdue, live): (Vec<_>, Vec<_>) = self.started.iter().partition(|(_, started)| {
            now_ms.saturating_sub(*started) >= SWAPPED_SPELL_EFFECT_MAX_LIFE_MS
        });
        self.started = live;
        overdue.into_iter().map(|(handle, _)| handle).collect()
    }

    pub fn len(&self) -> usize {
        self.started.len()
    }

    pub fn is_empty(&self) -> bool {
        self.started.is_empty()
    }
}

pub fn spell_kind(name: &str) -> Option<&'static str> {
    let rest = name.strip_prefix(PLAYER_SPELL_EFFECT_PREFIX)?;
    Some(
        PLAYER_SPELL_KINDS
            .iter()
            .find(|kind| rest.starts_with(**kind))
            .copied()
            .unwrap_or("other"),
    )
}

pub fn spell_look(name: &str) -> SpellLook {
    if let Some(found) = SPELL_LOOK_PAIRS.iter().find(|pair| pair.original == name) {
        return SpellLook::Hers(found);
    }
    if name.starts_with(PLAYER_SPELL_EFFECT_PREFIX) {
        SpellLook::Tinted
    } else {
        SpellLook::AsItIs
    }
}

/// The look of an effect spawned on this side: only the local player's spells on the swapped
/// body change, never an effect played for the partner.
pub fn swapped_spell_look(name: &str, wears_swapped_body: bool, for_partner: bool) -> SpellLook {
    if !wears_swapped_body || for_partner {
        return SpellLook::AsItIs;
    }
    spell_look(name)
}

/// The name sent to the partner for an effect of the local player's: hers when it has a pair.
pub fn sent_spell_effect_name(name: String, wears_swapped_body: bool) -> String {
    match swapped_spell_look(&name, wears_swapped_body, false) {
        SpellLook::Hers(found) => found.her_name().to_owned(),
        SpellLook::Tinted | SpellLook::AsItIs => name,
    }
}

pub fn her_effect_id(effect_id: i32) -> Option<i32> {
    SPELL_LOOK_PAIRS
        .iter()
        .find(|pair| pair.original_id == Some(effect_id))
        .map(|pair| pair.hers_id)
}

/// A shot's four effect ids as sent to the partner: each with a pair becomes hers.
pub fn sent_shot_effect_ids<const SLOTS: usize>(
    effect_ids: [i32; SLOTS],
    wears_swapped_body: bool,
) -> [i32; SLOTS] {
    if !wears_swapped_body {
        return effect_ids;
    }
    effect_ids.map(|id| her_effect_id(id).unwrap_or(id))
}

pub fn tint_style(style: &mut [f32]) {
    for (channel, value) in style.iter_mut().zip(SWAPPED_BODY_SPELL_TINT_RGB) {
        *channel = value;
    }
}

pub fn spell_look_line(name: &str, look: SpellLook) -> String {
    let kind = spell_kind(name).unwrap_or("other");
    match look {
        SpellLook::Hers(found) => {
            let original_id = found
                .original_id
                .map_or_else(|| "by name".to_owned(), |id| format!("id {id}"));
            let basis = match found.basis {
                PairBasis::ByRole => "same role",
                PairBasis::Guess => "a guess",
            };
            format!(
                "spell look: {kind} spell effect {name} ({original_id}) -> {} (id {}), {basis}",
                found.her_name(),
                found.hers_id
            )
        }
        SpellLook::Tinted => format!(
            "spell look: {kind} spell effect {name} has no pair -> kept, in her colour when spawned by name"
        ),
        SpellLook::AsItIs => format!("spell look: {name} kept as it is"),
    }
}

/// The name of an effect spawned by name when it is one of the player's spell effects; None
/// after reading only the prefix otherwise.
pub fn read_spell_effect_name<M: GameMemory>(
    memory: &M,
    address: u64,
) -> Result<Option<String>, GameError> {
    if address == 0 {
        return Ok(None);
    }
    let mut prefix = [0_u8; PLAYER_SPELL_EFFECT_PREFIX.len()];
    memory.read(address, &mut prefix)?;
    if prefix != *PLAYER_SPELL_EFFECT_PREFIX.as_bytes() {
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(SPELL_EFFECT_NAME_READ_LEN);
    for offset in 0..SPELL_EFFECT_NAME_READ_LEN as u64 {
        match memory.read_u8(address + offset)? {
            0 => return Ok(String::from_utf8(bytes).ok()),
            byte => bytes.push(byte),
        }
    }
    Ok(None)
}
