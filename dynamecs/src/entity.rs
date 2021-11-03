use crate::serialization::SerializableEntity;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ENTITY: AtomicU64 = AtomicU64::new(0);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Entity(u64);

impl Display for Entity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Entity {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Entity(NEXT_ENTITY.fetch_add(1, Ordering::SeqCst))
    }
}

impl From<Entity> for SerializableEntity {
    fn from(id: Entity) -> Self {
        Self(id.0)
    }
}
