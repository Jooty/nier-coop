use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::Replicant;
use replicant_game::cube_probe::{
    GrabEvent, LinkTarget, NEAR_ACTOR_RANGE, NearbyActor, PLAYER_LINK_SCAN_BYTES, find_links,
    grab_link_text, grab_started_text, held_position_text, nearby_actor_text, nearest_actor_index,
    read_block_lossy, read_nearby_actors, released_text,
};
use replicant_game::move_log::role_label;
use replicant_game::stay_together::FollowRole;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::gimmick_probe::actor_list::actor_position;
use crate::gimmick_probe::subject::{HeldChoice, HeldCube, Subject};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::puppet::first_actor_address;
use crate::world_objects::actor_in_group;

const LOCAL_LABEL: &str = "local player";
const PARTNER_LABEL: &str = "partner puppet";

static SUBJECTS: LazyLock<Mutex<[Subject; 2]>> =
    LazyLock::new(|| Mutex::new([Subject::new(LOCAL_LABEL), Subject::new(PARTNER_LABEL)]));

pub fn tick_gimmick_probe(build: &VerifiedBuild) {
    let memory = InProcessMemory::new(build);
    let game = Replicant::new(InProcessMemory::new(build));
    let role = role_label(net::handle().map_or(FollowRole::Unknown, |net| {
        FollowRole::from_slot(net.view().local_slot)
    }));
    let actors = [
        game.playable_actor().unwrap_or(0),
        first_actor_address().unwrap_or(0),
    ];
    let now = net_clock_ms();
    let mut subjects = SUBJECTS.lock().unwrap_or_else(PoisonError::into_inner);
    for (subject, actor) in subjects.iter_mut().zip(actors) {
        subject.follow_actor(actor);
        if actor == 0 {
            continue;
        }
        let name = subject.body_motion_name(&memory);
        let event = subject.watch.observe(now, name.as_deref());
        if event != GrabEvent::Idle {
            report(build, &memory, role, subject, event, name.as_deref());
        }
    }
}

fn report(
    build: &VerifiedBuild,
    memory: &InProcessMemory,
    role: &str,
    subject: &mut Subject,
    event: GrabEvent,
    motion_name: Option<&str>,
) {
    let label = subject.label;
    let now = net_clock_ms();
    let held_for_ms = subject.watch.held_for_ms(now);
    match event {
        GrabEvent::Idle => {}
        GrabEvent::Started => {
            let Some(position) = actor_position(memory, subject.actor) else {
                return;
            };
            let near = nearby(build, memory, subject.actor, position);
            log_line(&grab_started_text(
                role,
                label,
                motion_name.unwrap_or("unnamed"),
                position,
                near.len(),
            ));
            for (rank, actor) in near.iter().enumerate() {
                log_line(&nearby_actor_text(
                    role,
                    label,
                    build.module_base(),
                    rank,
                    actor,
                ));
            }
            update_links(memory, role, subject, &near);
            choose_held(subject, &near);
        }
        GrabEvent::Holding => {
            if let Some(position) = actor_position(memory, subject.actor) {
                let near = nearby(build, memory, subject.actor, position);
                update_links(memory, role, subject, &near);
                choose_held(subject, &near);
            }
            if let Some(held) = subject.held {
                log_line(&held_position_text(
                    role,
                    label,
                    held.actor,
                    held.choice.text(),
                    actor_position(memory, held.actor),
                    held_for_ms,
                ));
            }
        }
        GrabEvent::Released | GrabEvent::GaveUp => {
            let ended_by = if event == GrabEvent::Released {
                "released"
            } else {
                "hold ended without a release motion"
            };
            let held = subject.held.take();
            log_line(&released_text(
                role,
                label,
                held.map(|held| held.actor),
                held.and_then(|held| actor_position(memory, held.actor)),
                held_for_ms,
                ended_by,
            ));
            subject.last_links.clear();
        }
    }
}

fn nearby(
    build: &VerifiedBuild,
    memory: &InProcessMemory,
    subject_actor: u64,
    center: [f32; 3],
) -> Vec<NearbyActor> {
    read_nearby_actors(
        memory,
        center,
        NEAR_ACTOR_RANGE,
        &[subject_actor],
        |group, index| actor_in_group(build, group, index),
    )
}

fn update_links(memory: &InProcessMemory, role: &str, subject: &mut Subject, near: &[NearbyActor]) {
    let targets: Vec<LinkTarget> = near
        .iter()
        .map(|actor| LinkTarget {
            actor: actor.actor,
            handle: actor.handle,
        })
        .collect();
    let block = read_block_lossy(memory, subject.actor, PLAYER_LINK_SCAN_BYTES);
    let links = find_links(&block, &targets);
    if links != subject.last_links {
        log_line(&grab_link_text(
            role,
            subject.label,
            PLAYER_LINK_SCAN_BYTES,
            &links,
        ));
        subject.last_links = links;
    }
}

fn choose_held(subject: &mut Subject, near: &[NearbyActor]) {
    if let Some(link) = subject.last_links.first() {
        subject.held = Some(HeldCube {
            actor: link.actor,
            choice: HeldChoice::Link,
        });
        return;
    }
    if subject
        .held
        .is_some_and(|held| held.choice == HeldChoice::Link)
    {
        return;
    }
    subject.held = nearest_actor_index(near).map(|index| HeldCube {
        actor: near[index].actor,
        choice: HeldChoice::NearestGuess,
    });
}
