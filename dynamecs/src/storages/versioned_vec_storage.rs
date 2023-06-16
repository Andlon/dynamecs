use crate::join::IntoJoinable;
use crate::storages::vec_storage::VecStorageJoinable;
use crate::storages::{VecStorage, Version, VersionedVecStorage};
use crate::{Entity, GetComponentForEntity, GetComponentForEntityMut, InsertComponentForEntity};
use std::ops::Deref;

impl<Component> Default for VersionedVecStorage<Component> {
    fn default() -> Self {
        Self {
            storage: Default::default(),
            versions: Default::default(),
            storage_version: Default::default(),
        }
    }
}

impl<Component> Deref for VersionedVecStorage<Component> {
    type Target = VecStorage<Component>;

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<Component> VersionedVecStorage<Component> {
    /// Inserts a component associated with the given entity, and returns the index in the
    /// storage.
    ///
    /// If a component already exists, it is replaced, and the version associated with the entity
    /// is advanced.
    pub fn insert(&mut self, entity: Entity, component: Component) -> usize {
        self.storage_version.advance();
        let idx = self.storage.insert(entity, component);
        // idx can be one-past the current length, but not greater
        if let Some(rev) = self.versions.get_mut(idx) {
            rev.advance();
        } else {
            assert_eq!(idx, self.versions.len());
            self.versions.push(Version::new());
        }
        idx
    }

    /// Returns a mutable reference to the component associated with the given entity.
    ///
    /// If the component exists, the storage version and the version associated with the
    /// component are advanced.
    pub fn get_component_mut(&mut self, id: Entity) -> Option<&mut Component> {
        self.storage.get_index(id).map(|idx| {
            self.storage_version.advance();
            self.versions[idx].advance();
            &mut self.storage.components_mut()[idx]
        })
    }

    /// Returns a mutable slice to the components.
    ///
    /// Advances the storage version and *all* component versions.
    pub fn components_mut(&mut self) -> &mut [Component] {
        self.storage_version.advance();
        for version in &mut self.versions {
            version.advance();
        }
        self.storage.components_mut()
    }

    pub fn get_component_version(&self, id: Entity) -> Option<Version<Component>> {
        self.storage
            .get_index(id)
            .map(|idx| self.versions[idx].clone())
    }

    pub fn storage_version(&self) -> Version<Self> {
        self.storage_version
    }

    pub fn versions(&self) -> &[Version<Component>] {
        &self.versions
    }
}

impl<'a, Component> IntoJoinable<'a> for &'a VersionedVecStorage<Component> {
    type Joinable = VecStorageJoinable<'a, Component>;

    fn into_joinable(self) -> Self::Joinable {
        self.storage.into_joinable()
    }
}

impl<C> GetComponentForEntity<C> for VersionedVecStorage<C> {
    fn get_component_for_entity(&self, id: Entity) -> Option<&C> {
        self.get_component(id)
    }
}

impl<C> GetComponentForEntityMut<C> for VersionedVecStorage<C> {
    fn get_component_for_entity_mut(&mut self, id: Entity) -> Option<&mut C> {
        self.get_component_mut(id)
    }
}

impl<C> InsertComponentForEntity<C> for VersionedVecStorage<C> {
    fn insert_component_for_entity(&mut self, entity: Entity, component: C) {
        self.insert(entity, component);
    }
}
