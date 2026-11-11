use crate::party_frame::PARTY_FRAME_NAME_SIZE_REFERENCE_PIXELS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuningField {
    pub key: &'static str,
    pub label: &'static str,
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

pub const TUNING_FIELDS: [TuningField; 11] = [
    TuningField {
        key: "offset_x",
        label: "frame X offset (moves the bars' right end)",
        default: 70.0,
        min: -400.0,
        max: 400.0,
    },
    TuningField {
        key: "offset_y",
        label: "frame Y offset",
        default: 1.0,
        min: -200.0,
        max: 400.0,
    },
    TuningField {
        key: "bar_scale",
        label: "bar size (1 = the game's own bars)",
        default: 1.0,
        min: 0.5,
        max: 2.0,
    },
    TuningField {
        key: "bar_gap",
        label: "gap between the HP and MP bars",
        default: 1.5,
        min: -10.0,
        max: 40.0,
    },
    TuningField {
        key: "name_offset_x",
        label: "name X offset",
        default: 0.0,
        min: -300.0,
        max: 300.0,
    },
    TuningField {
        key: "name_offset_y",
        label: "name Y offset",
        default: -5.5,
        min: -60.0,
        max: 60.0,
    },
    TuningField {
        key: "name_size",
        label: "name size",
        default: PARTY_FRAME_NAME_SIZE_REFERENCE_PIXELS,
        min: 8.0,
        max: 40.0,
    },
    TuningField {
        key: "name_row_gap",
        label: "vertical gap between the two name rows",
        default: 38.0,
        min: 0.0,
        max: 80.0,
    },
    TuningField {
        key: "status_size",
        label: "status word size",
        default: 13.0,
        min: 6.0,
        max: 30.0,
    },
    TuningField {
        key: "status_offset_x",
        label: "status word X offset (0 = right end of the HP bar)",
        default: 0.0,
        min: -300.0,
        max: 300.0,
    },
    TuningField {
        key: "status_offset_y",
        label: "status word Y offset (0 = centred on the HP bar)",
        default: 0.0,
        min: -60.0,
        max: 60.0,
    },
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartyFrameTuning {
    values: [f32; TUNING_FIELDS.len()],
}

impl PartyFrameTuning {
    pub const fn new() -> Self {
        let mut values = [0.0; TUNING_FIELDS.len()];
        let mut index = 0;
        while index < TUNING_FIELDS.len() {
            values[index] = TUNING_FIELDS[index].default;
            index += 1;
        }
        Self { values }
    }

    pub fn get(&self, index: usize) -> f32 {
        self.values.get(index).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, index: usize, value: f32) {
        if let (Some(slot), Some(field)) = (self.values.get_mut(index), TUNING_FIELDS.get(index))
            && value.is_finite()
        {
            *slot = value.clamp(field.min, field.max);
        }
    }

    pub fn set_by_key(&mut self, key: &str, value: f32) -> bool {
        match TUNING_FIELDS.iter().position(|field| field.key == key) {
            Some(index) => {
                self.set(index, value);
                true
            }
            None => false,
        }
    }

    pub fn offset_x(&self) -> f32 {
        self.get(0)
    }

    pub fn offset_y(&self) -> f32 {
        self.get(1)
    }

    pub fn bar_scale(&self) -> f32 {
        self.get(2)
    }

    pub fn bar_gap(&self) -> f32 {
        self.get(3)
    }

    pub fn name_offset_x(&self) -> f32 {
        self.get(4)
    }

    pub fn name_offset_y(&self) -> f32 {
        self.get(5)
    }

    pub fn name_size(&self) -> f32 {
        self.get(6)
    }

    pub fn name_row_gap(&self) -> f32 {
        self.get(7)
    }

    pub fn status_size(&self) -> f32 {
        self.get(8)
    }

    pub fn status_offset_x(&self) -> f32 {
        self.get(9)
    }

    pub fn status_offset_y(&self) -> f32 {
        self.get(10)
    }

    pub fn as_text(&self) -> String {
        TUNING_FIELDS
            .iter()
            .enumerate()
            .map(|(index, field)| format!("{} = {:.1}\n", field.key, self.get(index)))
            .collect()
    }

    pub fn parse_text(text: &str) -> Self {
        let mut tuning = Self::new();
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=')
                && let Ok(value) = value.trim().parse::<f32>()
            {
                tuning.set_by_key(key.trim(), value);
            }
        }
        tuning
    }
}

impl Default for PartyFrameTuning {
    fn default() -> Self {
        Self::new()
    }
}
