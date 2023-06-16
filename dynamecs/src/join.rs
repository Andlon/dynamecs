//! Functionality that enables the Join API.
use crate::storages::{
    vec_storage::{VecStorageEntityComponentIter, VecStorageEntityComponentIterMut},
    VecStorage,
};
use crate::Entity;

pub trait IntoJoinable<'a> {
    type Joinable: Joinable<'a>;

    fn into_joinable(self) -> Self::Joinable;
}

pub trait Joinable<'a> {
    type ComponentRef;

    /// Makes a reference to the component associated with the given entity, or `None` if no such component exists.
    ///
    /// # Safety
    ///
    /// This function may never be called more than once with the same entity throughout the lifetime of the Joinable.
    unsafe fn try_make_component_ref(&mut self, entity: Entity) -> Option<Self::ComponentRef>;
}

pub struct JoinIter<Joinables> {
    joinables: Joinables,
}

/// Base macro for generating Iterator impls for JoinIter for various tuple combinations
///
/// This is used to construct macros for the distinct immutable/mutable cases
macro_rules! impl_join_iter_base {
    ($iter:ty, $component_ref:ty, $($joinables:ident),*) => {
        #[allow(non_snake_case)]
        #[allow(unused_parens)]
        #[allow(irrefutable_let_patterns)]
        impl<'a, C, $($joinables),*> Iterator for JoinIter<($iter $(, $joinables)*)>
        where
            $($joinables : Joinable<'a>),*
        {
            type Item = (Entity, $component_ref $(, $joinables::ComponentRef)*);

            fn next(&mut self) -> Option<Self::Item> {
                // Re-use the type name as a variable name in order to unpack tuple
                // (so e.g. J1 becomes the joinable v ariable associated with the J1 type)
                let (ref mut storage $(, ref mut $joinables)*) = self.joinables;
                while let Some((entity, c0)) = storage.next() {
                    // SAFETY: VecStorageEntityComponentIter is guaranteed never to repeat an entity,
                    // so we can uphold the safety invariant of the joinable

                    // Re-use/shadow variable names *again* so that now J1, J2 etc. correspond to
                    // Option<JX::ComponentRef>
                    $(let $joinables = unsafe { $joinables.try_make_component_ref(entity) };)*

                    // Return if all joinables have components associated with the current entity,
                    // otherwise we keep iterating
                    if let ($(Some($joinables)),*) = ($($joinables),*) {
                        // Shadow *again* so that now J1, J2 etc. correspond to the individual component references
                        return Some((entity, c0 $(, $joinables)*));
                    }
                }

                None
            }
        }
    }
}

/// Macro for generating JoinIter impls where
macro_rules! impl_join_iter {
    ($($joinables:ident),*) => {
        impl_join_iter_base!(VecStorageEntityComponentIter<'a, C>, &'a C, $($joinables),*);
    }
}

macro_rules! impl_join_iter_mut {
    ($($joinables:ident),*) => {
        impl_join_iter_base!(VecStorageEntityComponentIterMut<'a, C>, &'a mut C, $($joinables),*);
    }
}

impl_join_iter!();
impl_join_iter!(J1);
impl_join_iter!(J1, J2);
impl_join_iter!(J1, J2, J3);
impl_join_iter!(J1, J2, J3, J4);
impl_join_iter!(J1, J2, J3, J4, J5);
impl_join_iter!(J1, J2, J3, J4, J5, J6);
impl_join_iter!(J1, J2, J3, J4, J5, J6, J7);

impl_join_iter_mut!();
impl_join_iter_mut!(J1);
impl_join_iter_mut!(J1, J2);
impl_join_iter_mut!(J1, J2, J3);
impl_join_iter_mut!(J1, J2, J3, J4);
impl_join_iter_mut!(J1, J2, J3, J4, J5);
impl_join_iter_mut!(J1, J2, J3, J4, J5, J6);
impl_join_iter_mut!(J1, J2, J3, J4, J5, J6, J7);

pub trait Join {
    type Iter: Iterator;

    fn join(self) -> Self::Iter;
}

/// Common base macro for implementing Join for tuples starting with a VecStorage reference (mutable/immutable)
macro_rules! impl_vec_storage_tuple_join_base {
    ($storage_ref:ty, $entity_component_iter:ty, $storage_var:ident => $entity_component_expr:expr, $($joinables:ident),*) => {
        #[allow(unused_parens)]
        impl<'a, C, $($joinables),*> Join for ($storage_ref, $($joinables),*)
        where
            $($joinables: IntoJoinable<'a>),*
        {
            type Iter = JoinIter<($entity_component_iter $(, $joinables::Joinable)*)>;

            #[allow(non_snake_case)]
            fn join(self) -> Self::Iter {
                // This unpacks the tuple by defining variables with the same names as the types,
                // which we can iterate on
                let ($storage_var, $($joinables),*) = self;
                JoinIter {
                    joinables: ($entity_component_expr $(, $joinables.into_joinable())*)
                }
            }
        }
    }
}

macro_rules! impl_vec_storage_tuple_join {
    ($($joinables:ident),*) => {
        impl_vec_storage_tuple_join_base!(&'a VecStorage<C>,
            VecStorageEntityComponentIter<'a, C>,
            storage => storage.entity_component_iter(),
            $($joinables),*);
    }
}

macro_rules! impl_vec_storage_tuple_join_mut {
    ($($joinables:ident),*) => {
        impl_vec_storage_tuple_join_base!(
            &'a mut VecStorage<C>,
            VecStorageEntityComponentIterMut<'a, C>,
            storage => storage.entity_component_iter_mut(),
            $($joinables),*);
    }
}

impl_vec_storage_tuple_join!();
impl_vec_storage_tuple_join!(J1);
impl_vec_storage_tuple_join!(J1, J2);
impl_vec_storage_tuple_join!(J1, J2, J3);
impl_vec_storage_tuple_join!(J1, J2, J3, J4);
impl_vec_storage_tuple_join!(J1, J2, J3, J4, J5);
impl_vec_storage_tuple_join!(J1, J2, J3, J4, J5, J6);
impl_vec_storage_tuple_join!(J1, J2, J3, J4, J5, J6, J7);

impl_vec_storage_tuple_join_mut!();
impl_vec_storage_tuple_join_mut!(J1);
impl_vec_storage_tuple_join_mut!(J1, J2);
impl_vec_storage_tuple_join_mut!(J1, J2, J3);
impl_vec_storage_tuple_join_mut!(J1, J2, J3, J4);
impl_vec_storage_tuple_join_mut!(J1, J2, J3, J4, J5);
impl_vec_storage_tuple_join_mut!(J1, J2, J3, J4, J5, J6);
impl_vec_storage_tuple_join_mut!(J1, J2, J3, J4, J5, J6, J7);

impl<'a, C> Join for &'a mut VecStorage<C> {
    type Iter = VecStorageEntityComponentIterMut<'a, C>;

    fn join(self) -> Self::Iter {
        self.entity_component_iter_mut()
    }
}

impl<'a, C> Join for &'a VecStorage<C> {
    type Iter = VecStorageEntityComponentIter<'a, C>;

    fn join(self) -> Self::Iter {
        self.entity_component_iter()
    }
}
