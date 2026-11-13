mod companion_off;
mod hit_power;
mod partner_hide;
mod save_guard;
mod stats;

pub(crate) use companion_off::{
    companion_off_function_ran, keep_companion_off, note_hit_for_companion_off,
};
pub(crate) use hit_power::hit_at_first_body_power;
pub(crate) use partner_hide::hide_partner_kaine;
pub(crate) use save_guard::guard_coop_save;
pub(crate) use stats::{match_kaine_stats_to_protagonist, set_her_level_before_player_is_built};
