use crate::unit_tests::dummy_components::{A, B, C};
use dynamecs::storages::VersionedVecStorage;
use dynamecs::{Component, Universe};
use std::array;

#[test]
fn test_basic_use() {
    let mut universe = Universe::default();
    let [e1, e2, e3] = array::from_fn(|_| universe.new_entity());
    let storage = universe.get_storage_mut::<VersionedVecStorage<A>>();

    storage.insert(e1, A(1));
    storage.insert(e2, A(2));
    storage.insert(e3, A(3));

    let [v1, v2, v3] = [e1, e2, e3].map(|entity| storage.get_component_version(entity).unwrap());
    let v_storage = storage.storage_version();
    assert_eq!(storage.components(), &[A(1), A(2), A(3)]);
    assert_eq!(storage.entities(), &[e1, e2, e3]);
    assert_eq!(storage.versions(), &[v1, v2, v3]);
    assert_eq!(storage.storage_version(), v_storage);

    // Access components mutably: leads to version bump for all components
    assert_eq!(storage.components_mut(), &mut [A(1), A(2), A(3)]);
    assert_eq!(storage.entities(), &[e1, e2, e3]);
    assert!(storage.storage_version() > v_storage);
    for (v_old, v_new) in [v1, v2, v3].iter().zip(storage.versions()) {
        assert!(v_new > v_old);
    }

    let [v1, v2, v3] = [e1, e2, e3].map(|entity| storage.get_component_version(entity).unwrap());
    let v_storage = storage.storage_version();
    // Accessing a single component mutably should advance the version of the
    // component, plus the version of the storage
    let _c2 = storage.get_component_mut(e2);
    assert!(v_storage < storage.storage_version());
    // Check that only the second element got a version bump
    assert_eq!(v1, storage.get_component_version(e1).unwrap());
    assert_eq!(v3, storage.get_component_version(e3).unwrap());
    assert!(v2 < storage.get_component_version(e2).unwrap());
}

#[test]
fn test_versioned_vec_storage_join() {
    let universe = Universe::default();

    struct Versioned<C>(pub C);

    impl<C: 'static> Component for Versioned<C> {
        type Storage = VersionedVecStorage<C>;
    }

    for (_entity, _) in universe.join::<&Versioned<A>>() {}
    for (_entity, _, _) in universe.join::<(&Versioned<A>, &Versioned<B>)>() {}
    for (_entity, _, _, _) in universe.join::<(&Versioned<A>, &Versioned<B>, &Versioned<C>)>() {}

    // Mixed with VecStorage
    for (_entity, _, _) in universe.join::<(&Versioned<A>, &B)>() {}
    for (_entity, _, _) in universe.join::<(&A, &Versioned<B>)>() {}
    for (_entity, _, _, _) in universe.join::<(&Versioned<A>, &B, &Versioned<C>)>() {}

    // TODO: In the above tests, we have only checked that some join statements type check
    // but we have not checked actual correctness. Should do this
}
