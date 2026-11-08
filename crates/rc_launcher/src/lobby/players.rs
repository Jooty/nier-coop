use rc_proto::PartnerBody;

pub const HOST_CHARACTER_TOOLTIP: &str =
    "The host plays the normal body; changing it would break sync.";

pub const PLAYER_TABLE_HEADERS: [&str; 4] = ["Player", "Role", "Character", "Status"];

const PICKER_CHOICES: [PartnerBody; 2] = [PartnerBody::Nier, PartnerBody::PrologueCharacter];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerRole {
    Host,
    Guest,
}

impl PlayerRole {
    pub fn text(self) -> &'static str {
        match self {
            Self::Host => "Host",
            Self::Guest => "Guest",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RowInputs<'a> {
    pub viewer_is_guest: bool,
    pub own_name: &'a str,
    pub peer_name: Option<&'a str>,
    pub guest_body: PartnerBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRow {
    pub name: String,
    pub role: PlayerRole,
    pub you: bool,
    pub body: PartnerBody,
    pub picker: bool,
}

pub fn picker_choices() -> &'static [PartnerBody] {
    &PICKER_CHOICES
}

pub fn player_rows(inputs: RowInputs) -> Vec<PlayerRow> {
    let (host_name, guest_name) = if inputs.viewer_is_guest {
        (inputs.peer_name, Some(inputs.own_name))
    } else {
        (Some(inputs.own_name), inputs.peer_name)
    };
    let mut rows = Vec::new();
    if let Some(name) = host_name {
        rows.push(PlayerRow {
            name: name.to_owned(),
            role: PlayerRole::Host,
            you: !inputs.viewer_is_guest,
            body: PartnerBody::Nier,
            picker: false,
        });
    }
    if let Some(name) = guest_name {
        rows.push(PlayerRow {
            name: name.to_owned(),
            role: PlayerRole::Guest,
            you: inputs.viewer_is_guest,
            body: inputs.guest_body,
            picker: inputs.viewer_is_guest,
        });
    }
    rows
}
