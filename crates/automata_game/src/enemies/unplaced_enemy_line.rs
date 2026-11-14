use crate::enemies::EnemySighting;

#[derive(Debug, Clone, PartialEq)]
pub struct UnplacedEnemyFacts {
    pub object_id: u32,
    pub name: String,
    pub set_id: u32,
    pub set_word: u32,
    pub vftable_rva: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub local_story_place: String,
    pub same_object_in_the_other_roster: Option<usize>,
}

pub fn unplaced_enemy_line(facts: &UnplacedEnemyFacts) -> String {
    let class = match facts.vftable_rva {
        Some(rva) => format!("class vftable rva {rva:#X} (no class name is known)"),
        None => "class not read".to_owned(),
    };
    let position = match facts.position {
        Some([x, y, z]) => format!("{x:.1},{y:.1},{z:.1}"),
        None => "not read".to_owned(),
    };
    let other = match facts.same_object_in_the_other_roster {
        Some(0) => "the other copy's list holds no enemy of this object id".to_owned(),
        Some(count) => format!(
            "the other copy's list holds {count} enemies of this object id (its list tells no positions, so within 5 units cannot be said)"
        ),
        None => "no list from the other copy yet".to_owned(),
    };
    format!(
        "enemies: no layout entry or generator holds object {:#X} named {:?}: set id {:#X}, set word {:#X}, {class}, stands at {position}, this copy's story place {}; {other}",
        facts.object_id, facts.name, facts.set_id, facts.set_word, facts.local_story_place
    )
}

pub fn enemies_of_one_object_in_a_roster(
    roster: Option<&[EnemySighting]>,
    object_id: u32,
) -> Option<usize> {
    roster.map(|roster| {
        roster
            .iter()
            .filter(|sighting| sighting.object_id == object_id)
            .count()
    })
}
