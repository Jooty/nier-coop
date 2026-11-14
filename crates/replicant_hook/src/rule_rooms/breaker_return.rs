use std::sync::{Mutex, PoisonError};

use replicant_game::Position;
use replicant_game::fight_sync::CHANGE_CAMERA_TO_FREE_CAMERA_KIND;
use replicant_game::own_ride::SET_PLAYER_DEFAULT_STATUS_NATIVE_RVA;
use replicant_game::rule_rooms::{
    BLACK_FADE_DEPTH, BLACK_FADE_KIND, BreakerReturn, CHANGE_CAMERA_BACK_TO_THE_PLAYER_SECONDS,
    CHANGE_CAMERA_BLEND_KIND, RESET_FREE_CAMERA_NATIVE_RVA, RETURN_FADE_IN_SECONDS,
    RETURN_FADE_OUT_SECONDS, RETURN_INVINCIBLE_SECONDS, ReturnSpot, RuleAct, RulePlayer, RuleRoom,
    SET_PLAYER_VISIBLE_NATIVE_RVA, SET_SNOW_ACTIVE_AREA_BY_PLAYER_POSITION_NATIVE_RVA,
    SET_UNIQUE_ACTOR_INVINCIBLE_NATIVE_RVA, return_spot_of, returned_line,
    returned_without_a_spot_line,
};
use replicant_game::side_view::{CHANGE_CAMERA_NATIVE_RVA, CHANGE_CAMERA_STATUS_NATIVE_RVA};
use replicant_game::story_jump::PLAYER_UNIQUE_ACTOR_ID;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::net::net_clock_ms;
use crate::rule_rooms::rearm::arm_the_breaks_again;
use crate::story_jump::{NativeArg, call_native_with};
use crate::teleport::move_player_facing;

static RETURN: Mutex<BreakerReturn> = Mutex::new(BreakerReturn::new());

pub(super) fn begin(room: &'static RuleRoom, player: RulePlayer) {
    let started =
        RETURN
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .begin(room, player, net_clock_ms());
    if started {
        crate::fade_sync::fade_on_this_game_only(
            true,
            BLACK_FADE_KIND,
            RETURN_FADE_OUT_SECONDS,
            BLACK_FADE_DEPTH,
        );
    }
}

pub(super) fn tick(build: &VerifiedBuild) {
    let due = RETURN
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_if_due(net_clock_ms());
    let Some((room, player)) = due else {
        return;
    };
    let Some(spot) = return_spot_of(room) else {
        log_line(&returned_without_a_spot_line(room, RuleAct::TimeUp));
        return;
    };
    put_the_breaker_back(build, spot);
    arm_the_breaks_again(build, room);
    crate::fade_sync::fade_on_this_game_only(
        false,
        BLACK_FADE_KIND,
        RETURN_FADE_IN_SECONDS,
        BLACK_FADE_DEPTH,
    );
    log_line(&returned_line(room, player, spot));
}

fn put_the_breaker_back(build: &VerifiedBuild, spot: &ReturnSpot) {
    call_native_with(build, SET_PLAYER_DEFAULT_STATUS_NATIVE_RVA, &[]);
    call_native_with(
        build,
        SET_PLAYER_VISIBLE_NATIVE_RVA,
        &[NativeArg::Integer(1)],
    );
    let [x, y, z] = spot.position;
    move_player_facing(build, Position { x, y, z }, spot.heading_degrees);
    call_native_with(
        build,
        CHANGE_CAMERA_NATIVE_RVA,
        &[
            NativeArg::Integer(CHANGE_CAMERA_TO_FREE_CAMERA_KIND),
            NativeArg::Real(CHANGE_CAMERA_BACK_TO_THE_PLAYER_SECONDS),
            NativeArg::Integer(CHANGE_CAMERA_BLEND_KIND),
        ],
    );
    call_native_with(
        build,
        CHANGE_CAMERA_STATUS_NATIVE_RVA,
        &[
            NativeArg::Integer(spot.camera_status_kind),
            NativeArg::Real(spot.camera_status_seconds),
            NativeArg::Integer(CHANGE_CAMERA_BLEND_KIND),
        ],
    );
    call_native_with(build, RESET_FREE_CAMERA_NATIVE_RVA, &[]);
    call_native_with(
        build,
        SET_UNIQUE_ACTOR_INVINCIBLE_NATIVE_RVA,
        &[
            NativeArg::Integer(PLAYER_UNIQUE_ACTOR_ID),
            NativeArg::Real(RETURN_INVINCIBLE_SECONDS),
        ],
    );
    call_native_with(
        build,
        SET_SNOW_ACTIVE_AREA_BY_PLAYER_POSITION_NATIVE_RVA,
        &[],
    );
}
