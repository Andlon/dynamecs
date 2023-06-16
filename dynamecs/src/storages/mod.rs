//! Various component storages.
use crate::Entity;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::marker::PhantomData;

pub mod vec_storage;
pub mod versioned_vec_storage;

/// A storage that stores its components in a [`Vec`].
///
/// TODO: Currently doesn't support removal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecStorage<Component> {
    components: Vec<Component>,
    entities: Vec<Entity>,
    lookup_table: HashMap<Entity, usize>,
}

#[derive(Debug, Eq, Hash, Ord, serde::Serialize, serde::Deserialize)]
pub struct Version<T> {
    version: u64,
    marker: PhantomData<T>,
}

impl<T> PartialEq for Version<T> {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl<T> PartialOrd for Version<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.version.partial_cmp(&other.version)
    }
}

impl<T> Default for Version<T> {
    fn default() -> Self {
        Self {
            version: u64::default(),
            marker: PhantomData,
        }
    }
}

impl<T> Clone for Version<T> {
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            marker: PhantomData,
        }
    }
}

impl<T> Copy for Version<T> {}

impl<T> Version<T> {
    pub fn new() -> Self {
        Self {
            version: 0,
            marker: PhantomData,
        }
    }

    pub fn next(&self) -> Self {
        let new_rev = self
            .version
            .checked_add(1)
            .expect("Revision overflowed u64");
        Self {
            version: new_rev,
            ..*self
        }
    }

    pub fn advance(&mut self) {
        *self = self.next()
    }
}

/// A *versioned* variant of [`VecStorage`].
///
/// TODO: Currently doesn't support removal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VersionedVecStorage<Component> {
    storage: VecStorage<Component>,
    versions: Vec<Version<Component>>,
    storage_version: Version<Self>,
}

/// A Storage that stores a single component without any Entity relation.
#[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SingularStorage<Component> {
    component: Component,
}

impl<Component> SingularStorage<Component> {
    pub fn new(component: Component) -> Self {
        Self { component }
    }

    pub fn get_component(&self) -> &Component {
        &self.component
    }

    pub fn get_component_mut(&mut self) -> &mut Component {
        &mut self.component
    }
}

/// A Storage that stores a single *immutable* component without any Entity relation.
#[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImmutableSingularStorage<Component> {
    component: Component,
}

impl<Component> ImmutableSingularStorage<Component> {
    pub fn new(component: Component) -> Self {
        Self { component }
    }

    pub fn get_component(&self) -> &Component {
        &self.component
    }
}
