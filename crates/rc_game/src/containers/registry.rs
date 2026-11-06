use std::collections::HashMap;

use crate::containers::{
    CONTAINER_GLINT_EFFECT_ID, GATHERING_POINT_KIND_ARGUMENT, MAX_REGISTERED_POINTS,
};

#[derive(Debug, Default)]
pub struct ContainerRegistry {
    point_of_action: HashMap<i32, i32>,
    glint_of_point: HashMap<i32, i32>,
}

impl ContainerRegistry {
    pub fn note_registration(&mut self, action_id: i32, point_id: i32, kind_argument: i32) {
        if kind_argument == GATHERING_POINT_KIND_ARGUMENT
            || self.point_of_action.len() >= MAX_REGISTERED_POINTS
        {
            return;
        }
        self.point_of_action.insert(action_id, point_id);
    }

    pub fn note_effect(&mut self, handle: i32, effect_id: i32, point_id: i32) {
        if effect_id != CONTAINER_GLINT_EFFECT_ID
            || self.glint_of_point.len() >= MAX_REGISTERED_POINTS
        {
            return;
        }
        self.glint_of_point.insert(point_id, handle);
    }

    pub fn glint_handle_of(&self, action_id: i32) -> Option<i32> {
        let point_id = self.point_of_action.get(&action_id)?;
        self.glint_of_point.get(point_id).copied()
    }

    pub fn is_registered(&self, action_id: i32) -> bool {
        self.point_of_action.contains_key(&action_id)
    }

    pub fn clear(&mut self) {
        self.point_of_action.clear();
        self.glint_of_point.clear();
    }
}
