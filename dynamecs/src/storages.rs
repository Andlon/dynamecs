//! Various component storages.

use std::collections::HashMap;

use crate::join::{IntoJoinable, Joinable};
use crate::serialization::{EntityDeserialize, EntitySerializationMap, SerializableEntity};
use crate::{BijectiveStorage, BijectiveStorageMut, Entity, InsertComponentForEntity};

#[derive(Clone, Debug, serde::Serialize)]
pub struct VecStorage<Component> {
    components: Vec<Component>,
    entities: Vec<Entity>,
    lookup_table: HashMap<Entity, usize>,
}

// Helper struct to ease implementation of deserialization
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct VecStorageCompanion<Component> {
    components: Vec<Component>,
    entities: Vec<SerializableEntity>,
    lookup_table: HashMap<SerializableEntity, usize>,
}

impl<Component> VecStorageCompanion<Component> {
    pub fn to_storage(self, id_map: &mut EntitySerializationMap) -> VecStorage<Component> {
        VecStorage {
            components: self.components,
            entities: self
                .entities
                .into_iter()
                .map(|id| id_map.deserialize_entity(id))
                .collect(),
            lookup_table: self
                .lookup_table
                .into_iter()
                .map(|(id, idx)| (id_map.deserialize_entity(id), idx))
                .collect(),
        }
    }
}

/// Stores component in a vector, with a one-to-one relationship between entities and components.
impl<Component> VecStorage<Component> {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            entities: Vec::new(),
            lookup_table: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        debug_assert_eq!(self.components.len(), self.entities.len());
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        debug_assert_eq!(self.components.is_empty(), self.entities.is_empty());
        self.components.is_empty()
    }

    pub fn get_index(&self, id: Entity) -> Option<usize> {
        self.lookup_table.get(&id).map(usize::to_owned)
    }

    pub fn get_component(&self, id: Entity) -> Option<&Component> {
        self.components.get(self.get_index(id)?)
    }

    pub fn get_component_mut(&mut self, id: Entity) -> Option<&mut Component> {
        let index = self.get_index(id)?;
        self.components.get_mut(index)
    }

    pub fn insert(&mut self, id: Entity, component: Component) -> usize {
        let len = self.len();
        let index = *self.lookup_table.entry(id).or_insert_with(|| len);

        if index < self.components.len() {
            *self.components.get_mut(index).unwrap() = component;
        } else {
            self.components.push(component);
            self.entities.push(id);
            debug_assert_eq!(index + 1, self.components.len());
        }

        index
    }

    /// Removes the component associated with the given entity, if it exists.
    ///
    /// Returns `true` if a component was removed, otherwise `false`.
    pub fn remove_entity(&mut self, id: &Entity) -> bool {
        if let Some(index) = self.lookup_table.remove(id) {
            self.entities.remove(index);
            self.components.remove(index);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.components.clear();
        self.lookup_table.clear();
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    pub fn components_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    pub fn entity_component_iter(&self) -> VecStorageEntityComponentIter<'_, Component> {
        VecStorageEntityComponentIter {
            inner_iter: self.entities.iter().copied().zip(self.components.iter()),
        }
    }

    pub fn entity_component_iter_mut(&mut self) -> VecStorageEntityComponentIterMut<'_, Component> {
        VecStorageEntityComponentIterMut {
            inner_iter: self
                .entities
                .iter()
                .copied()
                .zip(self.components.iter_mut()),
        }
    }
}

// TODO: Move to vec_storage module?
pub struct VecStorageEntityComponentIter<'a, Component> {
    // We keep the inner iterator as an implementation detail so that we can swap it out if required later on
    inner_iter: std::iter::Zip<std::iter::Copied<std::slice::Iter<'a, Entity>>, std::slice::Iter<'a, Component>>,
}

// TODO: Move to vec_storage module?
pub struct VecStorageEntityComponentIterMut<'a, Component> {
    // We keep the inner iterator as an implementation detail so that we can swap it out if required later on
    inner_iter: std::iter::Zip<std::iter::Copied<std::slice::Iter<'a, Entity>>, std::slice::IterMut<'a, Component>>,
}

impl<'a, Component> Iterator for VecStorageEntityComponentIter<'a, Component> {
    type Item = (Entity, &'a Component);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next()
    }
}

impl<'a, Component> Iterator for VecStorageEntityComponentIterMut<'a, Component> {
    type Item = (Entity, &'a mut Component);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next()
    }
}

impl<Component> Default for VecStorage<Component> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> InsertComponentForEntity<C> for VecStorage<C> {
    fn insert_component_for_entity(&mut self, entity: Entity, component: C) {
        self.insert(entity, component);
    }
}

impl<C> BijectiveStorage for VecStorage<C> {
    type Component = C;

    fn get_component_for_entity(&self, id: Entity) -> Option<&Self::Component> {
        self.components.get(self.get_index(id)?)
    }
}

impl<C> BijectiveStorageMut for VecStorage<C> {
    fn insert_component(&mut self, id: Entity, component: Self::Component) {
        self.insert(id, component);
    }

    fn get_component_for_entity_mut(&mut self, id: Entity) -> Option<&mut Self::Component> {
        let index = self.get_index(id)?;
        self.components.get_mut(index)
    }
}

impl<'de, Component> EntityDeserialize<'de> for VecStorage<Component>
where
    Component: serde::Deserialize<'de>,
{
    fn entity_deserialize<D>(deserializer: D, id_map: &mut EntitySerializationMap) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        let companion = VecStorageCompanion::<Component>::deserialize(deserializer)?;
        Ok(companion.to_storage(id_map))
    }
}

/// A Storage that stores a single component without any Entity relation.
#[derive(Debug, Copy, Clone, serde::Serialize, Default)]
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

impl<'de, Component> EntityDeserialize<'de> for SingularStorage<Component>
where
    Component: EntityDeserialize<'de>,
{
    fn entity_deserialize<D>(deserializer: D, id_map: &mut EntitySerializationMap) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            component: Component::entity_deserialize(deserializer, id_map)?,
        })
    }
}

/// A Storage that stores a single *immutable* component without any Entity relation.
#[derive(Debug, Copy, Clone, serde::Serialize, Default)]
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

impl<'de, Component> EntityDeserialize<'de> for ImmutableSingularStorage<Component>
where
    Component: EntityDeserialize<'de>,
{
    fn entity_deserialize<D>(deserializer: D, id_map: &mut EntitySerializationMap) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            component: Component::entity_deserialize(deserializer, id_map)?,
        })
    }
}

#[derive(Debug)]
pub struct VecStorageJoinable<'a, C> {
    lookup_table: &'a HashMap<Entity, usize>,
    components: *const C,
}

impl<'a, C: 'a> Joinable<'a> for VecStorageJoinable<'a, C> {
    type ComponentRef = &'a C;

    unsafe fn try_make_component_ref(&mut self, entity: Entity) -> Option<Self::ComponentRef> {
        self.lookup_table.get(&entity).map(|index| {
            // TODO: Check for overflow? Can this occur in practice? I don't think so according to docs
            // of ptr::add, assuming our insertion code is correct and the indices in the lookup table
            // point to a location in the component array
            &*self.components.add(*index)
        })
    }
}

impl<'a, C> IntoJoinable<'a> for &'a VecStorage<C> {
    type Joinable = VecStorageJoinable<'a, C>;

    fn into_joinable(self) -> Self::Joinable {
        VecStorageJoinable {
            lookup_table: &self.lookup_table,
            components: self.components.as_ptr(),
        }
    }
}

#[derive(Debug)]
pub struct VecStorageJoinableMut<'a, C> {
    lookup_table: &'a HashMap<Entity, usize>,
    components: *mut C,
}

impl<'a, C: 'a> Joinable<'a> for VecStorageJoinableMut<'a, C> {
    type ComponentRef = &'a mut C;

    unsafe fn try_make_component_ref(&mut self, entity: Entity) -> Option<Self::ComponentRef> {
        self.lookup_table.get(&entity).map(|index| {
            // TODO: Check for overflow? Can this occur in practice? I don't think so according to docs
            // of ptr::add, assuming our insertion code is correct and the indices in the lookup table
            // point to a location in the component array
            &mut *self.components.add(*index)
        })
    }
}

impl<'a, C> IntoJoinable<'a> for &'a mut VecStorage<C> {
    type Joinable = VecStorageJoinableMut<'a, C>;

    fn into_joinable(self) -> Self::Joinable {
        VecStorageJoinableMut {
            lookup_table: &self.lookup_table,
            components: self.components.as_mut_ptr(),
        }
    }
}
