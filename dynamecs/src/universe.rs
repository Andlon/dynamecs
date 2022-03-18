use crate::fetch::{FetchComponentStorages, FetchComponentStoragesMut};
use crate::join::Join;
use crate::{
    register_component, Component, Entity, EntityFactory, GetComponentForEntity, GetComponentForEntityMut,
    InsertComponentForEntity, SerializableStorage, Storage,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

pub use universe_serialize::{register_serializer, register_storage, RegistrationStatus};

// Make universe_serialize a submodule of this module, so that it can still
// access private members of `StorageContainer`, without exposing this to the rest of the
// crate (using e.g. `pub(crate)`).
mod universe_serialize;

/// A container of component storages.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Universe {
    // Invariant: We never remove a storage from the hash map, so that the
    // Box<dyn Any> contained inside the type erased storage struct always points to the same
    // object in memory, until the Universe is destroyed. This allows us to safely
    // return (mutable) references by unsafely dereference pointers to the storages
    // (observing Rust's rules on references)
    // TODO: The current design is not fully sound due to pointer provenance (see various comments in method impls).
    // In order to hopefully get closer to a fully sound impl, a different design is required. One possiblity would
    // be to have:
    //  mapping: RefCell<HashMap<TypeId, usize>>,
    //  storages: UnsafeCell<Vec<TaggedTypeErasedStorage>>
    // That way at least we never have to use any unsafe code for interaction with the HashMap,
    // and through UnsafeCell we can soundly obtain a mutable reference to the vector in order to get mutable
    // pointers to the storages (although there are some provenance issues to be aware of here)
    storages: Storages,
    entity_factory: EntityFactory,
}

#[derive(Default)]
struct Storages {
    // TODO: Use a faster hash?
    storages: RefCell<HashMap<TypeId, TaggedTypeErasedStorage>>,
}

impl Deref for Storages {
    type Target = RefCell<HashMap<TypeId, TaggedTypeErasedStorage>>;

    fn deref(&self) -> &Self::Target {
        &self.storages
    }
}

impl DerefMut for Storages {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storages
    }
}

struct TaggedTypeErasedStorage {
    // tag is used for serialization/deserialization, obtained through the associated factory
    // of the Storage
    // TODO: Move tag to Storage trait, then provide tag as constructor parameter?
    tag: String,
    storage: Box<dyn Any>,
}

impl Universe {
    /// Create a new entity associated with this universe.
    pub fn new_entity(&self) -> Entity {
        self.entity_factory.new_entity()
    }

    /// Returns the provided storage if it already exists.
    pub fn try_get_storage<S: Storage>(&self) -> Option<&S> {
        self.storages
            .borrow()
            .get(&TypeId::of::<S>())
            .map(|type_erased_storage| &type_erased_storage.storage)
            .map(|boxed_storage| {
                boxed_storage
                    .downcast_ref::<S>()
                    .expect("Can always downcast since TypeIds match")
            })
            // SAFETY: We need to extend the lifetime beyond that of the RefCell's borrow.
            // This is sound because the pointer to the storage remains stable.
            .map(|storage_ref| unsafe { &*(storage_ref as *const _) })
    }

    pub fn try_get_component_storage<C: Component>(&self) -> Option<&C::Storage> {
        self.try_get_storage::<C::Storage>()
    }

    /// Returns a reference to the given storage.
    ///
    /// Storages are lazily constructed on demand: if the storage has not been accessed so far,
    /// it will be initialized with its [`Default`] implementation.
    ///
    /// The storage is stable in memory: For as long as the universe is alive, the pointer to the
    /// storage will remain valid.
    pub fn get_storage<S: Storage + Default>(&self) -> &S {
        // We must take some care here to not accidentally construct a mutable reference
        // to the storage through e.g. the `Entry` API of `HashMap`. This is important, because
        // if we've already given out an immutable reference to it, then we are not permitted to
        // obtain a mutable reference without invoking UB. Therefore we first
        // try to look up the storage in the hash map through "immutable means", and only
        // insert if it does not exist.
        let mut storages = self.storages.borrow_mut();

        // TODO: This is possibly UB due to pointer provenance. It's difficult to come up with a fool-proof solution
        // here because the provenance rules are generally unclear. At the very least, we should probably move the
        // storages themselves to something like a Vec<>, which is easier to reason about, so that we only do
        // "standard lookups" for indices in the hash map.

        let storage_ptr = if let Some(type_erased_storage) = storages.get(&TypeId::of::<S>()) {
            let storage_ref = type_erased_storage
                .storage
                .downcast_ref()
                .expect("Can always downcast since TypeIds match");
            storage_ref as *const _
        } else {
            // TODO: Obtain tag directly through storage?
            let tag = S::tag();
            let storage_ref = storages
                .entry(TypeId::of::<S>())
                .or_insert(TaggedTypeErasedStorage {
                    tag,
                    storage: Box::new(S::default()),
                })
                // Here it's OK that we have a mutable reference as we know nobody else can
                // have a mutable reference to this storage as we *just* inserted it
                .storage
                .downcast_ref()
                .expect("Can always downcast since TypeIds match");
            storage_ref as *const _
        };

        // SAFETY: We need unsafe here in order to extend the lifetime beyond that provided
        // by RefCell. This is sound because the pointer to the storage is valid for as long as
        // the universe exists, and changes to the hash map does not invalidate the pointer,
        // since we never remove entries.
        unsafe { &*storage_ptr }
    }

    /// Inserts the given storage into the container.
    ///
    /// If a storage of the same type was already present, it is returned. Otherwise `None` is returned.
    pub fn insert_storage<S: Storage>(&mut self, storage: S) -> Option<S> {
        let tag = S::tag();
        self.storages
            .get_mut()
            .insert(
                TypeId::of::<S>(),
                TaggedTypeErasedStorage {
                    tag,
                    storage: Box::new(storage),
                },
            )
            .map(|tagged_storage| {
                let boxed = tagged_storage
                    .storage
                    .downcast::<S>()
                    .expect("Downcast cannot fail since TypeIDs match");
                *boxed
            })
    }

    /// Same as [`insert_storage`](Self::insert_storage), but additionally registers the storage for deserialization.
    pub fn register_insert_storage<S: SerializableStorage>(&mut self, storage: S) -> Option<S> {
        register_storage::<S>();
        self.insert_storage(storage)
    }

    /// Returns a mutable reference to the given storage.
    ///
    /// Storages are lazily constructed on demand: if the storage has not been accessed so far,
    /// it will be initialized with its [`Default`] implementation.
    ///
    /// The storage is stable in memory: For as long as the universe is alive, the pointer to the
    /// storage will remain valid.
    pub fn get_storage_mut<S: Storage + Default>(&mut self) -> &mut S {
        let mut storages = self.storages.borrow_mut();
        let ref_mut = storages
            .entry(TypeId::of::<S>())
            .or_insert_with(|| TaggedTypeErasedStorage {
                tag: S::tag(),
                storage: Box::new(S::default()),
            })
            .storage
            .downcast_mut()
            .expect("Can always downcast since TypeIds match");

        // SAFETY: Because of the RefCell, we cannot return a reference with the same lifetime as the
        // storage. However, we can soundly extend this lifetime because of the invariant that we
        // never remove an entry from the hash map. This means in particular that the
        // data associated with the Box<_> does not move in memory for as long as the universe
        // exists, so we can create a reference to the storage with the lifetime of &mut self by
        // dereferencing this pointer
        // TODO: This reasoning is flawed because of pointer provenance, therefore it might be UB
        let ptr = ref_mut as *mut _;
        unsafe { &mut *ptr }
    }

    pub fn get_component_storage<C: Component>(&self) -> &C::Storage
    where
        C::Storage: Default,
    {
        self.get_storage::<C::Storage>()
    }

    pub fn get_component_storage_mut<C: Component>(&mut self) -> &mut C::Storage
    where
        C::Storage: Default,
    {
        self.get_storage_mut::<C::Storage>()
    }

    /// Fetch (shared or mutable) references to the storages of the requested components.
    ///
    /// This method must be used when mutable access to at least one component storage is required.
    /// However, all storages do not need to be mutably fetched. The mutability of each storage
    /// is determined by the associated mutability qualifier on the component reference provided.
    /// See the examples for how to use this method.
    ///
    /// The list of components must be distinct, regardless of whether they are accessed mutably
    /// or not.
    ///
    /// # Examples
    /// ```rust
    ///# use dynamecs::{Component, Universe};
    ///# use dynamecs::storages::VecStorage;
    ///# use std::default::Default;
    ///# use serde::{Serialize, Deserialize};
    ///# #[derive(Serialize, Deserialize)]
    ///# struct A; impl Component for A { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct B; impl Component for B { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct C; impl Component for C { type Storage = VecStorage<Self>; };
    ///#
    ///# let mut universe = Universe::default();
    /// let (a_storage, b_storage, c_storage) =
    ///     universe.get_component_storages_mut::<(&A, &mut B, &mut C)>();
    /// ```
    pub fn get_component_storages_mut<'a, Fetch>(&'a mut self) -> Fetch::Storages
    where
        Fetch: FetchComponentStoragesMut<'a>,
    {
        Fetch::fetch_storages_mut(self)
    }

    /// Fetch shared references to the storages of the requested components.
    ///
    /// You can use this method when you do not need mutable access to any of the component
    /// storages. Otherwise, you must use
    /// [`get_component_storages_mut`](Self::get_component_storages_mut). See the examples
    /// for usage instructions.
    ///
    /// # Examples
    /// ```rust
    ///# use dynamecs::{Component, Universe};
    ///# use dynamecs::storages::VecStorage;
    ///# use std::default::Default;
    ///# use serde::{Serialize, Deserialize};
    ///# #[derive(Serialize, Deserialize)]
    ///# struct A; impl Component for A { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct B; impl Component for B { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct C; impl Component for C { type Storage = VecStorage<Self>; };
    ///#
    ///# let mut universe = Universe::default();
    /// let (a_storage, b_storage, c_storage) = universe.get_component_storages::<(&A, &B, &C)>();
    /// ```
    pub fn get_component_storages<'a, Fetch>(&'a self) -> Fetch::Storages
    where
        Fetch: FetchComponentStorages<'a>,
    {
        Fetch::fetch_storages(self)
    }

    /// Performs an immutable join operation on the storages associated with the given components.
    ///
    /// This means that only shared references to components can be obtained. Use
    /// [`join_mut`](Self::join_mut) if mutable access is required.
    ///
    /// # Examples
    ///
    /// ```
    ///# use dynamecs::{Component, Universe};
    ///# use dynamecs::storages::VecStorage;
    ///# use std::default::Default;
    ///# use serde::{Serialize, Deserialize};
    ///# #[derive(Serialize, Deserialize)]
    ///# struct A; impl Component for A { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct B; impl Component for B { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct C; impl Component for C { type Storage = VecStorage<Self>; };
    ///#
    ///# let universe = Universe::default();
    /// for (entity, a, b, c) in universe.join::<(&A, &B, &C)>() {
    ///     // Process components
    /// }
    /// ```
    pub fn join<'a, Fetch>(&'a self) -> <Fetch::Storages as Join>::Iter
    where
        Fetch: FetchComponentStorages<'a>,
        Fetch::Storages: 'a + Join,
    {
        let storages = Fetch::fetch_storages(self);
        storages.join()
    }

    /// Performs a join operation on the storages associated with the given components, possibly giving mutable
    /// access to components.
    ///
    /// This means that both shared and mutable references to components can be obtained. The mutability is determined
    /// by the qualifier associated with each component. See the example below for usage.
    ///
    /// # Examples
    ///
    /// ```
    ///# use dynamecs::{Component, Universe};
    ///# use dynamecs::storages::VecStorage;
    ///# use std::default::Default;
    ///# use serde::{Serialize, Deserialize};
    ///# #[derive(Serialize, Deserialize)]
    ///# struct A; impl Component for A { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct B; impl Component for B { type Storage = VecStorage<Self>; };
    ///# #[derive(Serialize, Deserialize)]
    ///# struct C; impl Component for C { type Storage = VecStorage<Self>; };
    ///#
    ///# let mut universe = Universe::default();
    /// // In this example we only need mutable access to A and C, not B
    /// for (entity, a, b, c) in universe.join_mut::<(&mut A, &B, &mut C)>() {
    ///     // Process components
    /// }
    /// ```
    pub fn join_mut<'a, Fetch>(&'a mut self) -> <Fetch::Storages as Join>::Iter
    where
        Fetch: FetchComponentStoragesMut<'a>,
        Fetch::Storages: 'a + Join,
    {
        let storages = Fetch::fetch_storages_mut(self);
        storages.join()
    }

    pub fn insert_component<C: Component>(&mut self, component: C, entity: Entity)
    where
        C::Storage: Default + InsertComponentForEntity<C>,
    {
        self.get_component_storage_mut::<C>()
            .insert_component_for_entity(entity, component)
    }

    /// Same as [`insert_component`](Self::insert_component), but additionally registers the component
    /// for deserialization.
    pub fn register_insert_component<C: Component>(&mut self, component: C, entity: Entity)
    where
        C::Storage: SerializableStorage + Default + InsertComponentForEntity<C>,
    {
        register_component::<C>();
        self.insert_component(component, entity);
    }

    #[deprecated = "Use register_component instead"]
    pub fn insert_component_for_entity<C: Component>(&mut self, entity: Entity, component: C)
    where
        C::Storage: Default + InsertComponentForEntity<C>,
    {
        self.get_component_storage_mut::<C>()
            .insert_component_for_entity(entity, component)
    }

    pub fn get_component_for_entity<C: Component>(&self, entity: Entity) -> Option<&C>
    where
        C::Storage: Default + GetComponentForEntity<C>,
    {
        self.get_component_storage::<C>()
            .get_component_for_entity(entity)
    }

    pub fn get_component_for_entity_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C>
    where
        C::Storage: Default + GetComponentForEntityMut<C>,
    {
        self.get_component_storage_mut::<C>()
            .get_component_for_entity_mut(entity)
    }
}

impl Debug for Universe {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let storage_tags: Vec<_> = self
            .storages
            .borrow()
            .iter()
            .map(|(_, tagged_storage)| tagged_storage.tag.clone())
            .collect();
        f.debug_struct("Universe")
            .field("storage_tags", &storage_tags.as_slice())
            .finish()
    }
}
