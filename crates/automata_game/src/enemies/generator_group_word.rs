use crate::enemies::{COMPONENT_MAP_ENTRY_LEN, COMPONENT_MAP_ENTRY_VALUE_OFFSET};

/// The 16-bit word a generator writes into each enemy it made, on every step of its slot, read
/// back from the enemy. A guess: an area or group tag the game may need to draw or update it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorGroupWord {
    Word(u16),
    NoComponent,
    Unreadable,
}

impl GeneratorGroupWord {
    pub fn label(self) -> String {
        match self {
            Self::Word(word) => format!("{word:#X}"),
            Self::NoComponent => "none (the enemy has no such component)".to_owned(),
            Self::Unreadable => "unreadable".to_owned(),
        }
    }
}

/// The value of the component map entry with `kind`, from the map's raw entries
/// (`i32` kind, then the value at +8, 16 bytes each). Zero values count as absent.
pub fn component_in_map(entries: &[u8], kind: i32) -> Option<u64> {
    entries
        .as_chunks::<COMPONENT_MAP_ENTRY_LEN>()
        .0
        .iter()
        .find(|entry| entry[..4] == kind.to_le_bytes())
        .map(|entry| {
            let mut value = [0; 8];
            value.copy_from_slice(
                &entry[COMPONENT_MAP_ENTRY_VALUE_OFFSET..COMPONENT_MAP_ENTRY_VALUE_OFFSET + 8],
            );
            u64::from_le_bytes(value)
        })
        .filter(|value| *value != 0)
}
