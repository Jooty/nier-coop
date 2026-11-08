use crate::view_follow::{DOOR_IN_PREFIX, DOOR_IN_VIEW_PART_NUMBERS, DOOR_OUT_PREFIX};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorChainEvent<'a> {
    pub chain: &'a str,
    pub inside: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewChain {
    pub chain: &'static str,
    pub entry: &'static [&'static str],
    pub exit: &'static [&'static str],
}

pub const VIEW_CHAINS: [ViewChain; 10] = [
    ViewChain {
        chain: "of_NierHouse",
        entry: &[
            "GM_IN_Door_of_NierHouse_02",
            "CB_SideView_SetBright_NierHome_In",
        ],
        exit: &[
            "GM_OUT_Door_of_NierHouse_02",
            "CB_SideView_SetBright_NierHome_Out",
        ],
    },
    ViewChain {
        chain: "of_CenterBar",
        entry: &[
            "GM_IN_Door_of_CenterBar_02",
            "CB_SideView_SetBright_CenterBar_In",
        ],
        exit: &[
            "GM_OUT_Door_of_CenterBar_02",
            "CB_SideView_SetBright_CenterBar_Out",
        ],
    },
    ViewChain {
        chain: "RobotHouse",
        entry: &[
            "GM_IN_Door_RobotHouse02",
            "CB_SideView_SetBright_RobotHouse_In",
        ],
        exit: &[
            "GM_OUT_Door_RobotHouse02",
            "CB_SideView_SetBright_RobotHouse_Out",
        ],
    },
    ViewChain {
        chain: "of_Bar",
        entry: &["GM_IN_Door_of_Bar_02"],
        exit: &["GM_OUT_Door_of_Bar_02"],
    },
    ViewChain {
        chain: "of_Lighthouse",
        entry: &["GM_IN_Door_of_Lighthouse_02"],
        exit: &["GM_OUT_Door_of_Lighthouse_02"],
    },
    ViewChain {
        chain: "of_PopolRoom",
        entry: &["GM_IN_Door_of_PopolRoom_02"],
        exit: &["GM_OUT_Door_of_PopolRoom_02"],
    },
    ViewChain {
        chain: "of_PopolRoom2",
        entry: &["GM_IN_Door_of_PopolRoom2_02"],
        exit: &["GM_OUT_Door_of_PopolRoom2_02"],
    },
    ViewChain {
        chain: "of_Post",
        entry: &["GM_IN_Door_of_Post_02"],
        exit: &["GM_OUT_Door_of_Post_02"],
    },
    ViewChain {
        chain: "of_WestRoom",
        entry: &["GM_IN_Door_of_WestRoom_02"],
        exit: &["GM_OUT_Door_of_WestRoom_02"],
    },
    ViewChain {
        chain: "of_WestRoom2",
        entry: &["GM_IN_Door_of_WestRoom2_02"],
        exit: &["GM_OUT_Door_of_WestRoom2_02"],
    },
];

pub fn view_chain(chain: &str) -> Option<&'static ViewChain> {
    VIEW_CHAINS.iter().find(|known| known.chain == chain)
}

pub fn door_chain_event(name: &str) -> Option<DoorChainEvent<'_>> {
    let (inside, rest) = match (
        name.strip_prefix(DOOR_IN_PREFIX),
        name.strip_prefix(DOOR_OUT_PREFIX),
    ) {
        (Some(rest), _) => (true, rest),
        (None, Some(rest)) => (false, rest),
        (None, None) => return None,
    };
    let split = rest.len().checked_sub(2)?;
    if !rest.is_char_boundary(split) {
        return None;
    }
    let (stem, number) = rest.split_at(split);
    if !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if inside && !DOOR_IN_VIEW_PART_NUMBERS.contains(&number) {
        return None;
    }
    let chain = stem.strip_suffix('_').unwrap_or(stem);
    (!chain.is_empty() && view_chain(chain).is_some()).then_some(DoorChainEvent { chain, inside })
}
