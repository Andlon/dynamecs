//! Functionality related to serialization of component storages.
use std::collections::HashMap;

use serde::de::Deserialize;

use crate::Entity;

mod generic_factory;
pub use generic_factory::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SerializableEntity(pub(crate) u64);

impl<'de> EntityDeserialize<'de> for Entity {
    fn entity_deserialize<D>(
        deserializer: D,
        id_map: &mut EntitySerializationMap,
    ) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserializable = SerializableEntity::deserialize(deserializer)?;
        let entity = id_map.deserialize_entity(deserializable);
        Ok(entity)
    }
}

impl<'a, 'de> serde::de::DeserializeSeed<'de> for &'a mut EntitySerializationMap {
    type Value = Entity;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserializable = SerializableEntity::deserialize(deserializer)?;
        let entity = self.deserialize_entity(deserializable);
        Ok(entity)
    }
}

pub struct EntitySerializationMap {
    map: HashMap<SerializableEntity, Entity>,
}

impl EntitySerializationMap {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn deserialize_entity(&mut self, id: SerializableEntity) -> Entity {
        *self.map.entry(id).or_insert_with(Entity::new)
    }
}

/// An extension of serde's `Deserialize` that allows deserialization of types containing
/// instances `Entity` (which are not deserializable)
pub trait EntityDeserialize<'de>: Sized {
    fn entity_deserialize<D>(
        deserializer: D,
        id_map: &mut EntitySerializationMap,
    ) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>;
}

impl<'de, T> EntityDeserialize<'de> for T
where
    T: serde::Deserialize<'de>,
{
    fn entity_deserialize<D>(
        deserializer: D,
        _: &mut EntitySerializationMap,
    ) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer)
    }
}
