use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct HitSparkNames {
    seen: HashSet<String>,
}

impl HitSparkNames {
    pub fn is_first(&mut self, name: &str) -> bool {
        self.seen.insert(name.to_owned())
    }
}

pub fn spawn_is_sent(outermost: bool, local_hit_in_flight: bool) -> bool {
    outermost || local_hit_in_flight
}

pub fn hit_spark_log_line(name: &str, via: &str) -> String {
    format!("effects: hit spark {name} via {via}")
}
