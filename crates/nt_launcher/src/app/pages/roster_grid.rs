use eframe::egui::Rangef;

const SIDE_PADDING_PX: f32 = 8.0;
const NARROW_CHARACTER_KEY: &str = "Char.";
const WIDE_CHARACTER_KEY: &str = "Character";

struct ColumnWidths {
    number: f32,
    role: f32,
    character: f32,
    status: f32,
    ping: f32,
}

const NARROW_WIDTHS_PX: ColumnWidths = ColumnWidths {
    number: 30.0,
    role: 60.0,
    character: 46.0,
    status: 108.0,
    ping: 54.0,
};
const WIDE_WIDTHS_PX: ColumnWidths = ColumnWidths {
    number: 40.0,
    role: 96.0,
    character: 150.0,
    status: 170.0,
    ping: 80.0,
};

/// Where each of a roster column's cells sits across it: NO., NAME, ROLE, CHARACTER, STATUS and PING.
/// A board of one column spreads them wide; a board of two packs them.
pub(super) struct RosterGrid {
    pub number: Rangef,
    pub name: Rangef,
    pub role: Rangef,
    pub character: Rangef,
    pub status: Rangef,
    pub ping: Rangef,
    pub character_key: &'static str,
}

pub(super) fn roster_grid(column: Rangef, wide: bool) -> RosterGrid {
    let widths = if wide {
        &WIDE_WIDTHS_PX
    } else {
        &NARROW_WIDTHS_PX
    };
    let left = column.min + SIDE_PADDING_PX;
    let right = column.max - SIDE_PADDING_PX;
    let ping = Rangef::new(right - widths.ping, right);
    let status = Rangef::new(ping.min - widths.status, ping.min);
    let character = Rangef::new(status.min - widths.character, status.min);
    let role = Rangef::new(character.min - widths.role, character.min);
    let number = Rangef::new(left, left + widths.number);
    RosterGrid {
        number,
        name: Rangef::new(number.max, role.min.max(number.max)),
        role,
        character,
        status,
        ping,
        character_key: if wide {
            WIDE_CHARACTER_KEY
        } else {
            NARROW_CHARACTER_KEY
        },
    }
}
