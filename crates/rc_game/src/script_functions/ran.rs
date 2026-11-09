use crate::script_functions::MAX_RAN_FUNCTIONS;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionsRan {
    phase: Option<u16>,
    names: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhaseLoadWatch {
    loads_seen: u32,
}

impl PhaseLoadWatch {
    pub const fn new() -> Self {
        Self { loads_seen: 0 }
    }

    pub fn is_new_load(&mut self, loads: u32) -> bool {
        let changed = self.loads_seen != loads;
        self.loads_seen = loads;
        changed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateCopy {
    RunIt,
    RanHereTraced,
}

impl FunctionsRan {
    pub const fn new() -> Self {
        Self {
            phase: None,
            names: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.phase = None;
        self.names.clear();
    }

    pub fn enter_phase(&mut self, phase: Option<u16>) {
        if self.phase != phase {
            self.names.clear();
            self.phase = phase;
        }
    }

    pub fn note(&mut self, phase: u16, name: &str) -> bool {
        self.enter_phase(Some(phase));
        if self.contains(phase, name) {
            return false;
        }
        if self.names.len() >= MAX_RAN_FUNCTIONS {
            self.names.remove(0);
        }
        self.names.push(name.to_owned());
        true
    }

    pub fn contains(&self, phase: u16, name: &str) -> bool {
        self.phase == Some(phase) && self.names.iter().any(|known| known == name)
    }

    pub fn names_in(&self, phase: u16) -> impl Iterator<Item = &str> {
        let names: &[String] = if self.phase == Some(phase) {
            &self.names
        } else {
            &[]
        };
        names.iter().map(String::as_str)
    }
}

pub fn late_copy_verdict(ran: &FunctionsRan, phase: u16, name: &str) -> LateCopy {
    if ran.contains(phase, name) {
        LateCopy::RanHereTraced
    } else {
        LateCopy::RunIt
    }
}
