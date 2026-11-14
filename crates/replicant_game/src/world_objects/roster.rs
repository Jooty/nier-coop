use replicant_proto::WorldObjectKey;

use crate::world_objects::world_object_key;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectSighting {
    pub actor: u64,
    pub kind_vtable_rva: u32,
    pub group: u16,
    pub position: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownObject {
    pub id: u16,
    pub key: WorldObjectKey,
    pub actor: u64,
}

/// The listed objects of this map as this game first saw them. The key travels on the wire; the
/// id is this game's own short name for an object and never leaves it.
#[derive(Debug, Default)]
pub struct WorldObjectRoster {
    phase: Option<u16>,
    objects: Vec<KnownObject>,
    next_id: u16,
}

impl WorldObjectRoster {
    pub fn objects(&self) -> &[KnownObject] {
        &self.objects
    }

    pub fn actor_of(&self, id: u16) -> Option<u64> {
        self.find(|object| object.id == id)
            .map(|object| object.actor)
    }

    pub fn key_of(&self, id: u16) -> Option<WorldObjectKey> {
        self.find(|object| object.id == id).map(|object| object.key)
    }

    pub fn id_of_actor(&self, actor: u64) -> Option<u16> {
        self.find(|object| object.actor == actor)
            .map(|object| object.id)
    }

    pub fn id_of_key(&self, key: &WorldObjectKey) -> Option<u16> {
        self.find(|object| object.key == *key)
            .map(|object| object.id)
    }

    pub fn observe(&mut self, phase: u16, sightings: &[ObjectSighting]) -> Vec<KnownObject> {
        if self.phase != Some(phase) {
            self.phase = Some(phase);
            self.objects.clear();
        }
        self.objects.retain(|object| {
            sightings.iter().any(|sighting| {
                sighting.actor == object.actor
                    && sighting.kind_vtable_rva == object.key.kind_vtable_rva
            })
        });
        let mut added = Vec::new();
        for sighting in sightings {
            if self.id_of_actor(sighting.actor).is_some() {
                continue;
            }
            let key = world_object_key(sighting.kind_vtable_rva, sighting.group, sighting.position);
            if self.id_of_key(&key).is_some() {
                continue;
            }
            let object = KnownObject {
                id: self.next_id,
                key,
                actor: sighting.actor,
            };
            self.next_id = self.next_id.wrapping_add(1);
            self.objects.push(object);
            added.push(object);
        }
        added
    }

    fn find(&self, wanted: impl Fn(&KnownObject) -> bool) -> Option<&KnownObject> {
        self.objects.iter().find(|object| wanted(object))
    }
}
