use std::any::{Any, TypeId};
use std::error::Error;
use std::fmt::Debug;

pub use entity::*;
pub use universe::*;

use crate::serialization::{EntityDeserialize, EntitySerializationMap, GenericFactory};

pub mod adapters;
mod entity;
pub mod fetch;
pub mod serialization;
pub mod storages;
mod universe;

pub mod join;

pub trait StorageFactory: Send + Sync {
    fn storage_tag(&self) -> String;

    fn serializable_storage<'a>(&self, storage: &'a dyn Any)
        -> Result<&'a dyn erased_serde::Serialize, Box<dyn Error>>;

    fn deserialize_storage(
        &self,
        deserializer: &mut dyn erased_serde::Deserializer,
        id_map: &mut EntitySerializationMap,
    ) -> Result<Box<dyn Any>, Box<dyn Error>>;

    fn storage_type_id(&self) -> TypeId;
}

pub trait Storage: 'static + serde::Serialize + for<'de> EntityDeserialize<'de> {
    fn new_factory() -> Box<dyn StorageFactory> {
        let factory = GenericFactory::<Self>::new();
        Box::new(factory)
    }
}

impl<S> Storage for S where S: 'static + serde::Serialize + for<'de> EntityDeserialize<'de> {}

pub trait InsertComponentForEntity<C> {
    fn insert_component_for_entity(&mut self, entity: Entity, component: C);
}

/// Storage that represents a one-to-one (bijective) correspondence between entities and components.
pub trait BijectiveStorage {
    // TODO: Move associated type to `Storage`?
    type Component;

    fn get_component_for_entity(&self, id: Entity) -> Option<&Self::Component>;
}

pub trait BijectiveStorageMut: BijectiveStorage {
    /// Inserts a component associated with the entity, overwriting any existing component
    /// that may already be associated with the given entity.
    fn insert_component(&mut self, id: Entity, component: Self::Component);

    fn get_component_for_entity_mut(&mut self, id: Entity) -> Option<&mut Self::Component>;
}

pub trait Component: 'static {
    type Storage: Storage;
}

pub fn register_component<C>() -> Result<RegistrationStatus, Box<dyn Error>>
where
    C: Component,
{
    register_storage::<C::Storage>()
}

pub trait System: Debug {
    fn name(&self) -> String;

    fn run(&mut self, data: &mut Universe) -> Result<(), Box<dyn Error>>;
}

/// A [`System`] that only has immutable access to the data.
pub trait ObserverSystem: Debug {
    fn name(&self) -> String;

    fn run(&mut self, data: &Universe) -> Result<(), Box<dyn Error>>;
}

impl<S: ObserverSystem> System for S {
    fn name(&self) -> String {
        <S as ObserverSystem>::name(self)
    }

    fn run(&mut self, data: &mut Universe) -> Result<(), Box<dyn Error>> {
        <S as ObserverSystem>::run(self, data)
    }
}

#[derive(Debug, Default)]
pub struct Systems {
    systems: Vec<Box<dyn System>>,
}

impl Systems {
    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push(system);
    }

    pub fn run_all(&mut self, data: &mut Universe) -> Result<(), Box<dyn Error>> {
        for system in &mut self.systems {
            system.run(data)?;
        }
        Ok(())
    }
}

pub fn join<Joinables: crate::join::Join>(joinables: Joinables) -> Joinables::Iter {
    joinables.join()
}
