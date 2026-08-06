use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use pystral_core::ui_log::{Logger, LogCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagId(pub u64);

#[derive(Debug, Clone)]
pub struct TagDef {
    pub id: TagId,
    pub max_stacks: u8,
}

pub struct TagRegistry {
    pub defs: HashMap<TagId, TagDef>,
}

#[derive(Default, Debug, Clone)]
pub struct TagBag {
    pub counts: HashMap<TagId, u8>,
}

impl TagBag {
    pub fn emit(&mut self, tag: TagId, n: u8, defs: &TagRegistry, logger: &mut Logger) {
        if let Some(def) = defs.defs.get(&tag) {
            let current = self.counts.entry(tag).or_insert(0);
            *current = (*current).saturating_add(n).min(def.max_stacks);
        } else {
            logger.apply_command(LogCommand::Log(format!("Attempted to emit undefined tag: {:?}", tag)));
        }
    }

    pub fn consume(&mut self, tag: TagId, n: u8) -> u8 {
        if let Some(current) = self.counts.get_mut(&tag) {
            let consumed = (*current).min(n);
            *current -= consumed;
            consumed
        } else {
            0
        }
    }
}
