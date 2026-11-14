use crate::rule_rooms::{RoomInhibit, RuleAct, RuleBreak, RuleRoom};

pub static RULE_ROOMS: [RuleRoom; 8] = [
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 1,
        starts: &["Start_Law_Game_01"],
        restart: "Restart_Law_Game_01",
        arms: &["Set_Law_Game_01"],
        removed_before_arming: &[5101, 5102, 5103],
        resets_the_room: false,
        clear: "Clear_Law_Game_01",
        clear_flag: 448,
        breaks: &[
            RuleBreak {
                handler: "Jump_has_Done",
                act: RuleAct::Jump,
            },
            RuleBreak {
                handler: "Hit_has_Done01",
                act: RuleAct::HitByTheRoom,
            },
        ],
        inhibits: &[
            RoomInhibit::Weapon,
            RoomInhibit::Magic,
            RoomInhibit::Jump,
            RoomInhibit::LockOn,
        ],
    },
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 2,
        starts: &["Start_Law_Game_02"],
        restart: "Restart_Law_Game_02",
        arms: &["Set_Law_Game_02", "Set_Law_Game_02_02"],
        removed_before_arming: &[5101, 5102, 5103, 5104],
        resets_the_room: false,
        clear: "Clear_Law_Game_02",
        clear_flag: 449,
        breaks: &[
            RuleBreak {
                handler: "Jump_has_Done02",
                act: RuleAct::Jump,
            },
            RuleBreak {
                handler: "Hit_has_Done02",
                act: RuleAct::HitByTheRoom,
            },
            RuleBreak {
                handler: "Move_has_Done",
                act: RuleAct::Move,
            },
        ],
        inhibits: &[
            RoomInhibit::Weapon,
            RoomInhibit::Magic,
            RoomInhibit::Jump,
            RoomInhibit::LockOn,
        ],
    },
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 3,
        starts: &["Start_Law_Game_03", "Start_Law_Game_03_NoMoveCamera"],
        restart: "Restart_Law_Game_03",
        arms: &["Set_Law_Game_03"],
        removed_before_arming: &[5101, 5102, 5103],
        resets_the_room: false,
        clear: "Clear_Law_Game_03",
        clear_flag: 450,
        breaks: &[
            RuleBreak {
                handler: "Run_has_Done",
                act: RuleAct::Run,
            },
            RuleBreak {
                handler: "Dash_has_Done03",
                act: RuleAct::Dash,
            },
            RuleBreak {
                handler: "CastMagic_has_Done_03",
                act: RuleAct::Magic,
            },
            RuleBreak {
                handler: "Hit_has_Done03",
                act: RuleAct::HitByTheRoom,
            },
        ],
        inhibits: &[
            RoomInhibit::Weapon,
            RoomInhibit::Avoid,
            RoomInhibit::Magic,
            RoomInhibit::LockOn,
        ],
    },
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 4,
        starts: &["Start_Law_Game_04"],
        restart: "Restart_Law_Game_04",
        arms: &["Set_Law_Game_04"],
        removed_before_arming: &[5101, 5102, 5103],
        resets_the_room: false,
        clear: "Clear_Law_Game_04",
        clear_flag: 451,
        breaks: &[
            RuleBreak {
                handler: "Guard_has_Done",
                act: RuleAct::Guard,
            },
            RuleBreak {
                handler: "CastMagic_has_Done_04",
                act: RuleAct::Magic,
            },
            RuleBreak {
                handler: "Hit_has_Done04",
                act: RuleAct::HitByTheRoom,
            },
        ],
        inhibits: &[
            RoomInhibit::Weapon,
            RoomInhibit::Magic,
            RoomInhibit::Guard,
            RoomInhibit::LockOn,
        ],
    },
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 5,
        starts: &["Start_Law_Game_05"],
        restart: "Restart_Law_Game_05",
        arms: &["Set_Law_Game_05"],
        removed_before_arming: &[5101, 5102, 5103],
        resets_the_room: false,
        clear: "Clear_Law_Game_05",
        clear_flag: 452,
        breaks: &[
            RuleBreak {
                handler: "Dash_has_Done05",
                act: RuleAct::Dash,
            },
            RuleBreak {
                handler: "CastMagic_has_Done",
                act: RuleAct::Magic,
            },
            RuleBreak {
                handler: "Hit_has_Done05",
                act: RuleAct::HitByTheRoom,
            },
        ],
        inhibits: &[
            RoomInhibit::Weapon,
            RoomInhibit::Magic,
            RoomInhibit::Avoid,
            RoomInhibit::LockOn,
        ],
    },
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 6,
        starts: &["Start_Law_Game_06"],
        restart: "Restart_Law_Game_06",
        arms: &["Set_Law_Game_06"],
        removed_before_arming: &[5101, 5102, 5103, 5108],
        resets_the_room: false,
        clear: "Clear_Law_Game_06",
        clear_flag: 453,
        breaks: &[
            RuleBreak {
                handler: "Attack_has_Done",
                act: RuleAct::Attack,
            },
            RuleBreak {
                handler: "Hit_has_Done06",
                act: RuleAct::HitByTheRoom,
            },
        ],
        inhibits: &[RoomInhibit::Weapon, RoomInhibit::Magic, RoomInhibit::LockOn],
    },
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 7,
        starts: &["Start_Law_Game_07"],
        restart: "Restart_Law_Game_07",
        arms: &["Set_Law_Game_07"],
        removed_before_arming: &[5101, 5102],
        resets_the_room: false,
        clear: "Clear_Law_Game_07",
        clear_flag: 454,
        breaks: &[
            RuleBreak {
                handler: "Dash_has_Done",
                act: RuleAct::Dash,
            },
            RuleBreak {
                handler: "Hit_has_Done07",
                act: RuleAct::HitByTheRoom,
            },
        ],
        inhibits: &[RoomInhibit::Avoid],
    },
    RuleRoom {
        pack: "A_DESERT_TEMPLE_01",
        room: 8,
        starts: &["Start_Law_Game_08"],
        restart: "Restart_Law_Game_08",
        arms: &["Set_Law_Game_08"],
        removed_before_arming: &[5101, 5102],
        resets_the_room: true,
        clear: "Clear_Law_Game_08",
        clear_flag: 455,
        breaks: &[
            RuleBreak {
                handler: "Time_has_Done",
                act: RuleAct::TimeUp,
            },
            RuleBreak {
                handler: "Hit_has_Done08",
                act: RuleAct::HitByTheRoom,
            },
        ],
        inhibits: &[],
    },
];
