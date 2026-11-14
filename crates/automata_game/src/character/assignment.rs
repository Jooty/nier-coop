use crate::character::Character;

pub fn pick_distinct(wanted: Character, taken: Character) -> Character {
    if wanted != taken {
        return wanted;
    }
    Character::ALL
        .into_iter()
        .find(|character| *character != taken)
        .unwrap_or(wanted)
}
