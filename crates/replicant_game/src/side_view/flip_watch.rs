#[derive(Debug, Default)]
pub struct SideViewWatch {
    last: Option<bool>,
}

impl SideViewWatch {
    pub const fn new() -> Self {
        Self { last: None }
    }

    pub fn observe(&mut self, side_view_on: bool) -> Option<bool> {
        let previous = self.last.replace(side_view_on);
        previous.filter(|was| *was != side_view_on)?;
        Some(side_view_on)
    }
}
