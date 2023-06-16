//! Helpers for caching values.
use std::collections::HashMap;
use crate::Entity;

/// A per-entity cache designed to work with [`Version`](crate::storages::Version)
/// and [`VersionedVecStorage`](crate::storages::VersionedVecStorage).
///
/// TODO: Really need some examples to show how it's useful.
///
/// TODO: Currently we never evict anything from the cache. Need to make it possible
/// to track what has been touched or not.
#[derive(Debug, Clone)]
pub struct VersionedEntityCache<Version, T> {
    map: HashMap<Entity, (Version, T)>
}

impl<Version, T> Default for VersionedEntityCache<Version, T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<Version, T> VersionedEntityCache<Version, T> {
    /// If the version of the cached value for the given entity does not match the provided version,
    /// then update the cache with the provided callable.
    pub fn update_if_outdated<E>(&mut self,
                                 entity: Entity,
                                 version: Version,
                                 value_fn: impl FnOnce() -> Result<T, E>
    ) -> Result<(), E>
        where
            Version: Eq
    {
        if let Some((cache_versions, value)) = self.map.get_mut(&entity) {
            if version != *cache_versions {
                *value = value_fn()?;
                *cache_versions = version;
            }
        } else {
            self.map.insert(entity, (version, value_fn()?));
        }
        Ok(())
    }

    /// Return the cached value for the given entity, if any.
    pub fn get_cached(&self, entity: &Entity) -> Option<&T> {
        self.map.get(entity)
            .map(|(_, value)| value)
    }
}
