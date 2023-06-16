//! Various component storages.
use crate::Entity;
use std::collections::HashMap;
use std::marker::PhantomData;

mod version_impl;

pub mod vec_storage;
pub mod versioned_vec_storage;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound = "")]
pub struct Version<T> {
    version: u64,
    marker: PhantomData<T>,
}

/// A storage that stores its components in a [`Vec`].
///
/// TODO: Currently doesn't support removal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecStorage<Component> {
    components: Vec<Component>,
    entities: Vec<Entity>,
    lookup_table: HashMap<Entity, usize>,
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
