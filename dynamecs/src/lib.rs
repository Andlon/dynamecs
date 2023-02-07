use crate::serialization::GenericStorageSerializer;
use adapters::{DelayedSystem, FilterSystem, SingleShotSystem};
use eyre::Context;
use std::any::{Any, TypeId};
use std::fmt::Debug;

pub use entity::*;
pub use universe::*;
use crate::join::Optional;

pub mod adapters;
pub mod components;
mod entity;
pub mod fetch;
pub mod join;
#[doc(hidden)]
pub mod serialization;
pub mod storages;
mod universe;

pub trait StorageSerializer: Send + Sync {
    fn storage_tag(&self) -> String;

    fn serializable_storage<'a>(&self, storage: &'a dyn Any) -> Option<&'a dyn erased_serde::Serialize>;

    fn deserialize_storage(
        &self,
        deserializer: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Any>, erased_serde::Error>;

    fn storage_type_id(&self) -> TypeId;
}

pub trait Storage: 'static {
    fn tag() -> String {
        // TODO: Ideally type_name should not be used for this purpose, so perhaps we should
        // force components to provide a tag?
        std::any::type_name::<Self>().to_string()
    }
}

impl<S: 'static> Storage for S {}

pub trait SerializableStorage: Storage + serde::Serialize + for<'de> serde::Deserialize<'de> {
    fn create_serializer() -> Box<dyn StorageSerializer> {
        let serializer = GenericStorageSerializer::<Self>::new();
        Box::new(serializer)
    }
}

impl<S> SerializableStorage for S where S: Storage + serde::Serialize + for<'de> serde::Deserialize<'de> {}

pub trait InsertComponentForEntity<C> {
    fn insert_component_for_entity(&mut self, entity: Entity, component: C);
}

/// Get a single component associated with the given entity, if it exists.
pub trait GetComponentForEntity<C> {
    fn get_component_for_entity(&self, id: Entity) -> Option<&C>;
}

pub trait GetComponentForEntityMut<C> {
    fn get_component_for_entity_mut(&mut self, id: Entity) -> Option<&mut C>;
}

pub trait Component: 'static {
    type Storage: Storage;
}

pub fn register_component<C>() -> RegistrationStatus
where
    C: Component,
    C::Storage: SerializableStorage,
{
    register_storage::<C::Storage>()
}

pub trait System: Debug {
    fn name(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }

    /// Registers components used by this system for serialization and deserialization
    fn register_components(&self) {}

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()>;

    /// Wraps the system such that can only run once.
    fn single_shot(self) -> SingleShotSystem<Self>
    where
        Self: Sized,
    {
        SingleShotSystem::new(self)
    }

    /// Wraps the system with a filter such that it only runs if the given predicate returns `true`.
    fn filter<P>(self, predicate: P) -> FilterSystem<P, Self>
    where
        Self: Sized,
        P: FnMut(&Universe) -> eyre::Result<bool>,
    {
        FilterSystem::new(self, predicate)
    }

    /// Wraps the system such that it only runs if the [`SimulationTime`](`crate::components::SimulationTime`) reaches the specified time.
    ///
    /// The system runs only if `simulation_time >= activation_time`
    fn delay_until(self, activation_time: f64) -> DelayedSystem<Self>
    where
        Self: Sized,
    {
        DelayedSystem::new(self, activation_time)
    }
}

/// A [`System`] that only has immutable access to the data.
pub trait ObserverSystem: Debug {
    fn name(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }

    /// Registers components used by this system for serialization and deserialization
    fn register_components(&self) {}

    fn run(&mut self, data: &Universe) -> eyre::Result<()>;
}

impl<S: ObserverSystem> System for S {
    fn name(&self) -> String {
        <S as ObserverSystem>::name(self)
    }

    fn register_components(&self) {
        <S as ObserverSystem>::register_components(self)
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        <S as ObserverSystem>::run(self, data)
    }
}

impl<S: System + 'static> From<S> for Box<dyn System> {
    fn from(system: S) -> Box<dyn System> {
        Box::new(system)
    }
}

#[derive(Debug, Default)]
pub struct Systems {
    systems: Vec<Box<dyn System>>,
}

impl Systems {
    pub fn add_system<S: Into<Box<dyn System>>>(&mut self, system: S) -> &mut Self {
        self.systems.push(system.into());
        self
    }

    pub fn register_components(&self) {
        for system in &self.systems {
            system.register_components();
        }
    }

    pub fn run_all(&mut self, data: &mut Universe) -> eyre::Result<()> {
        for system in &mut self.systems {
            system
                .run(data)
                .wrap_err_with(|| format!("failed to run system \"{}\"", system.name()))?;
        }
        Ok(())
    }
}

pub fn join<Joinables: crate::join::Join>(joinables: Joinables) -> Joinables::Iter {
    joinables.join()
}
