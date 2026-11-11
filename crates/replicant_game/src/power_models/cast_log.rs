#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastSummary {
    pub instance: u16,
    pub tier: &'static str,
    pub motions: Vec<u16>,
    pub lasted_ms: u64,
}

#[derive(Debug, Default)]
pub struct PowerCastLog {
    open: Vec<OpenCast>,
}

#[derive(Debug)]
struct OpenCast {
    instance: u16,
    tier: &'static str,
    started_ms: u64,
    motions: Vec<u16>,
}

impl CastSummary {
    pub fn describe(&self, whose: &str) -> String {
        let motions: Vec<String> = self.motions.iter().map(u16::to_string).collect();
        format!(
            "powers: {whose} hand cast tier {} #{} (motions [{}]), ended after {} ms",
            self.tier,
            self.instance,
            motions.join(", "),
            self.lasted_ms
        )
    }
}

impl PowerCastLog {
    /// Opens a cast for `instance` unless one is open; true when it is new.
    pub fn begin(&mut self, instance: u16, tier: &'static str, now_ms: u64) -> bool {
        if self.is_open(instance) {
            return false;
        }
        self.open.push(OpenCast {
            instance,
            tier,
            started_ms: now_ms,
            motions: Vec::new(),
        });
        true
    }

    /// Records `motion` for an open cast; true when the cast had not played it yet.
    pub fn motion(&mut self, instance: u16, motion: u16) -> bool {
        let Some(cast) = self.open.iter_mut().find(|cast| cast.instance == instance) else {
            return false;
        };
        if cast.motions.contains(&motion) {
            return false;
        }
        cast.motions.push(motion);
        true
    }

    pub fn end(&mut self, instance: u16, now_ms: u64) -> Option<CastSummary> {
        let at = self
            .open
            .iter()
            .position(|cast| cast.instance == instance)?;
        let cast = self.open.swap_remove(at);
        Some(CastSummary {
            instance,
            tier: cast.tier,
            motions: cast.motions,
            lasted_ms: now_ms.saturating_sub(cast.started_ms),
        })
    }

    /// Ends every open cast whose instance is not in `present`, oldest first.
    pub fn end_missing(&mut self, present: &[u16], now_ms: u64) -> Vec<CastSummary> {
        let missing: Vec<u16> = self
            .open
            .iter()
            .map(|cast| cast.instance)
            .filter(|instance| !present.contains(instance))
            .collect();
        missing
            .into_iter()
            .filter_map(|instance| self.end(instance, now_ms))
            .collect()
    }

    pub fn is_open(&self, instance: u16) -> bool {
        self.open.iter().any(|cast| cast.instance == instance)
    }
}
