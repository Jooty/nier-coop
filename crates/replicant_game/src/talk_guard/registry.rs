use std::collections::{HashMap, HashSet};

use crate::talk_guard::{
    TALK_REGISTRATION_MODE_COUNT, important_base_of_talk_id, is_important_talk,
};

#[derive(Debug, Default, Clone)]
struct ActorTalks {
    names: [Option<String>; TALK_REGISTRATION_MODE_COUNT],
    talk_ids: [Option<i32>; TALK_REGISTRATION_MODE_COUNT],
}

#[derive(Debug, Default)]
pub struct TalkRegistry {
    phase: String,
    actors: HashMap<i32, ActorTalks>,
    pressed: HashSet<i32>,
}

impl TalkRegistry {
    pub fn enter_phase(&mut self, phase: &str) {
        if self.phase != phase {
            self.clear();
            phase.clone_into(&mut self.phase);
        }
    }

    pub fn clear(&mut self) {
        self.actors.clear();
        self.pressed.clear();
    }

    pub fn register_name(&mut self, actor_id: i32, mode: i32, name: &str) {
        let Some(slot) = mode_slot(mode) else {
            return;
        };
        if name.is_empty() {
            return;
        }
        self.actors.entry(actor_id).or_default().names[slot] = Some(name.to_owned());
    }

    pub fn register_talk_id(&mut self, actor_id: i32, mode: i32, talk_id: i32) {
        let Some(slot) = mode_slot(mode) else {
            return;
        };
        self.actors.entry(actor_id).or_default().talk_ids[slot] = Some(talk_id);
    }

    pub fn names_of(&self, actor_id: i32) -> [Option<&str>; TALK_REGISTRATION_MODE_COUNT] {
        let Some(talks) = self.actors.get(&actor_id) else {
            return [None; TALK_REGISTRATION_MODE_COUNT];
        };
        [talks.names[0].as_deref(), talks.names[1].as_deref()]
    }

    pub fn important_name_of(&self, actor_id: i32) -> Option<&str> {
        self.names_of(actor_id)
            .into_iter()
            .flatten()
            .find(|name| is_important_talk(name))
    }

    pub fn important_talk_id_of(&self, actor_id: i32) -> Option<(i32, &'static str)> {
        let talks = self.actors.get(&actor_id)?;
        talks
            .talk_ids
            .iter()
            .flatten()
            .find_map(|id| Some((*id, important_base_of_talk_id(*id)?)))
    }

    pub fn first_press(&mut self, actor_id: i32) -> bool {
        self.pressed.insert(actor_id)
    }

    pub fn press_line(&self, actor_id: i32) -> String {
        let [first, second] = self.names_of(actor_id);
        let talk_id = self
            .actors
            .get(&actor_id)
            .and_then(|talks| talks.talk_ids.iter().flatten().next().copied());
        format!(
            "talk press: actor {actor_id} mode0 {} mode1 {} talk id {}",
            first.unwrap_or("none"),
            second.unwrap_or("none"),
            talk_id.map_or_else(|| "none".to_owned(), |id| id.to_string())
        )
    }
}

fn mode_slot(mode: i32) -> Option<usize> {
    usize::try_from(mode)
        .ok()
        .filter(|slot| *slot < TALK_REGISTRATION_MODE_COUNT)
}
