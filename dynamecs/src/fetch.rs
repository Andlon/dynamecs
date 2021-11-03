//! Helper traits to support the generic component storage "fetch" API.
use crate::{Component, Storage, Universe};
use std::any::TypeId;

pub trait FetchComponentStorages<'a> {
    type Storages;

    fn fetch_storages(universe: &'a Universe) -> Self::Storages;
}

/// Helper trait to enable the fetch syntax used by [`Universe::get_component_storages_mut`].
pub trait FetchComponentStoragesMut<'a> {
    type Storages;

    fn fetch_storages_mut(universe: &'a mut Universe) -> Self::Storages;
}

const MULTIPLE_MUTABLE_REF_ERROR: &'static str =
    "Stopped attempt to obtain multiple mutable references to the same storage. \
     Can not simultaneously mutably borrow the same storage type multiple times.";

/// Converts a mutable reference to a storage to a shared or mutable reference.
///
/// Helper trait to enable the fetch syntax used by [`Universe::get_component_storages_mut`].
pub trait ComponentStorageRefMut<'a> {
    type Storage: Storage;
    type RefMut;

    fn convert_storage_ref_mut(storage: &'a mut Self::Storage) -> Self::RefMut;
}

impl<'a, 'b, C: Component> ComponentStorageRefMut<'a> for &'b C {
    type Storage = C::Storage;
    type RefMut = &'a C::Storage;

    fn convert_storage_ref_mut(storage: &'a mut Self::Storage) -> Self::RefMut {
        &*storage
    }
}

impl<'a, 'b, C: Component> ComponentStorageRefMut<'a> for &'b mut C {
    type Storage = C::Storage;
    type RefMut = &'a mut C::Storage;

    fn convert_storage_ref_mut(storage: &'a mut Self::Storage) -> Self::RefMut {
        storage
    }
}

fn is_strictly_monotonic<T: Ord>(items: &[T]) -> bool {
    let mut iter = items.iter().peekable();
    while let Some(current) = iter.next() {
        if let Some(&next) = iter.peek() {
            if !(current < next) {
                return false;
            }
        }
    }
    true
}

impl<'a, 'b, C> FetchComponentStorages<'a> for &'a C
where
    C: Component,
    C::Storage: Default,
{
    type Storages = &'a C::Storage;

    fn fetch_storages(universe: &'a Universe) -> Self::Storages {
        universe.get_storage::<C::Storage>()
    }
}

impl<'a, 'b, C> FetchComponentStorages<'a> for &'a mut C
where
    C: Component,
    C::Storage: Default,
{
    type Storages = &'a C::Storage;

    fn fetch_storages(universe: &'a Universe) -> Self::Storages {
        universe.get_storage::<C::Storage>()
    }
}

macro_rules! impl_tuple_fetch_component_storages {
    ($($component:ident),+) => {
        impl<'a, 'b, $($component: Component),*> FetchComponentStorages<'a> for ($(&'b $component,)*)
        where
            $(<$component as Component>::Storage: Default),+
        {
            type Storages = ($(&'a $component::Storage,)*);

            fn fetch_storages(universe: &'a Universe) -> Self::Storages {
                ($(universe.get_storage::<$component::Storage>(),)*)
            }
        }
    }
}

impl_tuple_fetch_component_storages!(C1);
impl_tuple_fetch_component_storages!(C1, C2);
impl_tuple_fetch_component_storages!(C1, C2, C3);
impl_tuple_fetch_component_storages!(C1, C2, C3, C4);
impl_tuple_fetch_component_storages!(C1, C2, C3, C4, C5);
impl_tuple_fetch_component_storages!(C1, C2, C3, C4, C5, C6);
impl_tuple_fetch_component_storages!(C1, C2, C3, C4, C5, C6, C7);
impl_tuple_fetch_component_storages!(C1, C2, C3, C4, C5, C6, C7, C8);

impl<'a, 'b, C> FetchComponentStoragesMut<'a> for &'a mut C
where
    C: Component,
    C::Storage: Default,
{
    type Storages = &'a mut C::Storage;

    fn fetch_storages_mut(universe: &'a mut Universe) -> Self::Storages {
        universe.get_storage_mut::<C::Storage>()
    }
}

macro_rules! impl_tuple_fetch_component_storages_mut {
    ($($component:ident),+) => {
        impl<'a, 'b, $($component: ComponentStorageRefMut<'a>),*> FetchComponentStoragesMut<'a> for ($($component,)*)
        where
            $(<$component as ComponentStorageRefMut<'a>>::Storage: Default),+
        {
            type Storages = ($($component::RefMut,)*);

            fn fetch_storages_mut(universe: &'a mut Universe) -> Self::Storages {
                // SAFETY: Ensure that all type IDs are unique, so that the pointers are unique,
                // otherwise it would be possible to obtain multiple mutable references to the same
                // storage
                let mut type_ids = [$(TypeId::of::<$component::Storage>(),)*];
                type_ids.sort_unstable();
                assert!(is_strictly_monotonic(&type_ids), "{}", MULTIPLE_MUTABLE_REF_ERROR);

                // For each tuple entry, we obtain a mutable pointer to the corresponding storage
                // and convert this into a mutable reference in order to extend its lifetime.
                // Finally, we convert this reference into the appropriate shared or mutable
                // reference associated with the storage (depending on mutability qualifier
                // in the input)
                // SAFETY: This is sound because the returned mutable references have a lifetime
                // tied to the universe itself
                ($($component::convert_storage_ref_mut(
                    unsafe { &mut *(universe.get_storage_mut() as *mut $component::Storage) }
                ),)*)
            }
        }
    }
}

impl_tuple_fetch_component_storages_mut!(C1);
impl_tuple_fetch_component_storages_mut!(C1, C2);
impl_tuple_fetch_component_storages_mut!(C1, C2, C3);
impl_tuple_fetch_component_storages_mut!(C1, C2, C3, C4);
impl_tuple_fetch_component_storages_mut!(C1, C2, C3, C4, C5);
impl_tuple_fetch_component_storages_mut!(C1, C2, C3, C4, C5, C6);
impl_tuple_fetch_component_storages_mut!(C1, C2, C3, C4, C5, C6, C7);
impl_tuple_fetch_component_storages_mut!(C1, C2, C3, C4, C5, C6, C7, C8);
