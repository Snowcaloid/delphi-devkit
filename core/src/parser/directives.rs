use std::collections::HashMap;
use super::STRINGS;

#[derive(Debug, Clone)]
pub struct Directives {
    map: HashMap<lasso::Spur, DirectiveState>,
}

#[derive(Debug, Clone)]
pub enum DirectiveState {
    Unknown,
    Deferred,
    Evaluated(bool),
}

impl Directives {
    pub fn from_dproj(dproj: dproj_rs::Dproj) -> Self {
        let mut map = HashMap::new();
        if let Ok(active_group) = dproj.active_property_group() {
            let directives = active_group.dcc_options.define.unwrap_or("".to_string());
            let directives: Vec<&str> = directives.split(';').collect();

            for directive in directives {
                map.insert(STRINGS.get_or_intern(directive), DirectiveState::Evaluated(true));
            }
        }
        Self { map }
    }

    pub fn define(&mut self, directive: &str) {
        let key = STRINGS.get_or_intern(directive);
        if let Some(state) = self.map.get_mut(&key) {
            *state = DirectiveState::Evaluated(true);
        } else {
            self.map.insert(key, DirectiveState::Evaluated(true));
        }
    }

    pub fn undef(&mut self, directive: &str) {
        let key = STRINGS.get_or_intern(directive);
        if let Some(state) = self.map.get_mut(&key) {
            *state = DirectiveState::Evaluated(false);
        } else {
            self.map.insert(key, DirectiveState::Evaluated(false));
        }
    }

    pub fn is_defined(&self, directive: &str) -> bool {
        return matches!(self.map.get(&STRINGS.get_or_intern(directive)), Some(DirectiveState::Evaluated(true)));
    }
}