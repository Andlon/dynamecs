use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Serialize, Deserialize)]
pub(crate) struct EntityFactory {
    next_entity: AtomicU64,
}

impl Default for EntityFactory {
    fn default() -> Self {
        Self {
            next_entity: AtomicU64::new(0),
        }
    }
}

impl EntityFactory {
    pub fn new_entity(&self) -> Entity {
        Entity(self.next_entity.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity(u64);

impl Display for Entity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
