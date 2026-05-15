use std::collections::HashMap;

use serde_json::Value;

/// Per-document awareness state. Maps user_id -> opaque JSON state. We treat
/// the state as a black box (cursor pos, selection range, etc. — clients
/// agree on its shape).
#[derive(Default, Debug, Clone)]
pub struct Awareness {
    states: HashMap<String, Value>,
}

impl Awareness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, user_id: String, state: Value) {
        self.states.insert(user_id, state);
    }

    pub fn remove(&mut self, user_id: &str) {
        self.states.remove(user_id);
    }

    pub fn snapshot(&self) -> HashMap<String, Value> {
        self.states.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn update_inserts_and_overwrites() {
        let mut a = Awareness::new();
        a.update("u1".into(), json!({ "cursor": 0 }));
        a.update("u1".into(), json!({ "cursor": 5 }));
        assert_eq!(a.snapshot()["u1"]["cursor"], 5);
    }

    #[test]
    fn remove_clears_user_state() {
        let mut a = Awareness::new();
        a.update("u1".into(), json!({}));
        a.update("u2".into(), json!({}));
        a.remove("u1");
        assert!(!a.snapshot().contains_key("u1"));
        assert!(a.snapshot().contains_key("u2"));
    }
}
