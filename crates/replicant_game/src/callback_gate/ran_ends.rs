#[derive(Debug, Clone, Default)]
pub struct RanEnds {
    phase: Option<u16>,
    names: Vec<String>,
}

impl RanEnds {
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

    pub fn note(&mut self, name: &str, phase: u16) {
        if self.phase != Some(phase) {
            self.names.clear();
            self.phase = Some(phase);
        }
        if !self.names.iter().any(|known| known == name) {
            self.names.push(name.to_owned());
        }
    }

    pub fn contains(&self, name: &str, phase: u16) -> bool {
        self.phase == Some(phase) && self.names.iter().any(|known| known == name)
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.names.iter().any(|known| known == name)
    }
}
