use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterHello {
    pub character: Character,
    pub exe: ExeVariant,
}
