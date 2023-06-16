//! Various component storages.
use crate::Entity;
use std::collections::HashMap;

pub mod vec_storage;

/// A storage that stores its components in a [`Vec`].
///
/// TODO: Currently doesn't support removal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecStorage<Component> {
    components: Vec<Component>,
    entities: Vec<Entity>,
    lookup_table: HashMap<Entity, usize>,
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
