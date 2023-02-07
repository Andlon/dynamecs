use crate::unit_tests::dummy_components::{A, B, C};
use dynamecs::join::{Join, Optional};
use dynamecs::storages::VecStorage;
use dynamecs::{Entity, Universe};

#[test]
#[rustfmt::skip]
fn join_compiles() {
    // Just check that the Join machinery actually compiles and gives expected types

    let (mut a_storage, mut b_storage, mut c_storage): (VecStorage<A>, VecStorage<B>, VecStorage<C>) = Default::default();

    // A
    for tuple in (&a_storage).join() { let _: (Entity, &A) = tuple; }

    // A, B
    for tuple in (&a_storage, &b_storage).join() { let _: (Entity, &A, &B) = tuple; }
    // for tuple in (&mut a_storage, &b_storage).join() { let _: (Entity, &mut A, &B) = tuple; }
    for tuple in (&a_storage, &mut b_storage).join() { let _: (Entity, &A, &mut B) = tuple; }
    // for tuple in (&mut a_storage, &mut b_storage).join() { let _: (Entity, &mut A, &mut B) = tuple; }

    // A, B, C
    for tuple in (&a_storage, &b_storage, &c_storage).join() { let _: (Entity, &A, &B, &C) = tuple; }
    for tuple in (&mut a_storage, &b_storage, &c_storage).join() { let _: (Entity, &mut A, &B, &C) = tuple; }
    for tuple in (&a_storage, &mut b_storage, &c_storage).join() { let _: (Entity, &A, &mut B, &C) = tuple; }
    for tuple in (&a_storage, &b_storage, &mut c_storage).join() { let _: (Entity, &A, &B, &mut C) = tuple; }
    for tuple in (&mut a_storage, &mut b_storage, &c_storage).join() { let _: (Entity, &mut A, &mut B, &C) = tuple; }
    for tuple in (&a_storage, &mut b_storage, &mut c_storage).join() { let _: (Entity, &A, &mut B, &mut C) = tuple; }
    for tuple in (&mut a_storage, &b_storage, &mut c_storage).join() { let _: (Entity, &mut A, &B, &mut C) = tuple; }
    for tuple in (&mut a_storage, &mut b_storage, &mut c_storage).join() { let _: (Entity, &mut A, &mut B, &mut C) = tuple; }

    // Optional components
    // A, B
    for tuple in (&a_storage, Optional(&b_storage)).join() { let _: (Entity, &A, Option<&B>) = tuple; }
}

struct TestData {
    v: Entity,
    x: Entity,
    y: Entity,
    z: Entity,

    a_storage: VecStorage<A>,
    b_storage: VecStorage<B>,
    c_storage: VecStorage<C>,
}

impl TestData {
    pub fn new_for_universe(universe: &Universe) -> Self {
        let v = universe.new_entity();
        let x = universe.new_entity();
        let y = universe.new_entity();
        let z = universe.new_entity();

        let mut a_storage = VecStorage::default();
        a_storage.insert(v, A(1));
        a_storage.insert(x, A(2));
        a_storage.insert(y, A(3));
        a_storage.insert(z, A(4));

        let mut b_storage = VecStorage::default();
        b_storage.insert(v, B(1));
        b_storage.insert(x, B(2));
        b_storage.insert(z, B(3));

        let mut c_storage = VecStorage::default();
        c_storage.insert(v, C(1));
        c_storage.insert(x, C(2));
        c_storage.insert(y, C(3));

        Self {
            v,
            x,
            y,
            z,
            a_storage,
            b_storage,
            c_storage,
        }
    }
}

#[test]
#[rustfmt::skip]
fn join_multiple_storages() {
    // Construct several storages, try to join different combinations and compare with expected results
    let universe = Universe::default();
    let TestData { v, x, y, z, mut a_storage, mut b_storage, mut c_storage } = TestData::new_for_universe(&universe);

    macro_rules! assert_join_eq {
        ($storages:expr, $expected:expr) => {
            assert_eq!($storages.join().collect::<Vec<_>>(),
                       $expected);
        }
    }

    // A
    {
        assert_join_eq!(&a_storage, vec![(v, &A(1)), (x, &A(2)), (y, &A(3)), (z, &A(4))]);
        assert_join_eq!((&a_storage,), vec![(v, &A(1)), (x, &A(2)), (y, &A(3)), (z, &A(4))]);
        assert_join_eq!(&mut a_storage, vec![(v, &mut A(1)), (x, &mut A(2)), (y, &mut A(3)), (z, &mut A(4))]);
        assert_join_eq!((&mut a_storage,), vec![(v, &mut A(1)), (x, &mut A(2)), (y, &mut A(3)), (z, &mut A(4))]);
    }

    // A-B
    {
        // Test all combinations of mutability as these lead to different code paths
        assert_join_eq!((&a_storage, &b_storage),
                        vec![(v, &A(1), &B(1)), (x, &A(2), &B(2)), (z, &A(4), &B(3))]);

        assert_join_eq!((&mut a_storage, &b_storage),
                        vec![(v, &mut A(1), &B(1)), (x, &mut A(2), &B(2)), (z, &mut A(4), &B(3))]);

        assert_join_eq!((&a_storage, &mut b_storage),
                        vec![(v, &A(1), &mut B(1)), (x, &A(2), &mut B(2)), (z, &A(4), &mut B(3))]);

        assert_join_eq!((&mut a_storage, &mut b_storage),
                        vec![(v, &mut A(1), &mut B(1)), (x, &mut A(2), &mut B(2)), (z, &mut A(4), &mut B(3))]);
    }

    // We don't check all possible codepaths for BC and AC, only the basic join mechanism
    assert_join_eq!((&b_storage, &c_storage), vec![(v, &B(1), &C(1)), (x, &B(2), &C(2))]);
    assert_join_eq!((&a_storage, &c_storage), vec![(v, &A(1), &C(1)), (x, &A(2), &C(2)), (y, &A(3), &C(3))]);

    // A-B-C
    {
        let abc_join: Vec<_> = (&a_storage, &b_storage, &c_storage).join().collect();
        assert_eq!(abc_join, vec![(v, &A(1), &B(1), &C(1)), (x, &A(2), &B(2), &C(2))]);

        let a_mut_bc_join: Vec<_> = (&mut a_storage, &b_storage, &c_storage).join().collect();
        assert_eq!(a_mut_bc_join, vec![(v, &mut A(1), &B(1), &C(1)), (x, &mut A(2), &B(2), &C(2))]);

        let ab_mut_c_join: Vec<_> = (&a_storage, &mut b_storage, &c_storage).join().collect();
        assert_eq!(ab_mut_c_join, vec![(v, &A(1), &mut B(1), &C(1)), (x, &A(2), &mut B(2), &C(2))]);

        let abc_mut_join: Vec<_> = (&a_storage, &b_storage, &mut c_storage).join().collect();
        assert_eq!(abc_mut_join, vec![(v, &A(1), &B(1), &mut C(1)), (x, &A(2), &B(2), &mut C(2))]);

        // We don't check *all* combinations here, so let's just skip to the "all mut" case
        let abc_all_mut_join: Vec<_> = (&mut a_storage, &mut b_storage, &mut c_storage).join().collect();
        assert_eq!(abc_all_mut_join, vec![(v, &mut A(1), &mut B(1), &mut C(1)), (x, &mut A(2), &mut B(2), &mut C(2))]);
    }

    // A-B with optional components
    {
        assert_join_eq!((&a_storage, Optional(&b_storage)),
                        vec![(v, &A(1), Some(&B(1))), (x, &A(2), Some(&B(2))), (y, &A(3), None), (z, &A(4), Some(&B(3)))]);
        assert_join_eq!((&a_storage, Optional(&mut b_storage)),
                        vec![(v, &A(1), Some(&mut B(1))), (x, &A(2), Some(&mut B(2))), (y, &A(3), None), (z, &A(4), Some(&mut B(3)))]);
    }

    // A-B-C with optional components
    {
        assert_join_eq!((&a_storage, Optional(&b_storage), Optional(&c_storage)),
                        vec![(v, &A(1), Some(&B(1)), Some(&C(1))), (x, &A(2), Some(&B(2)), Some(&C(2))), (y, &A(3), None, Some(&C(3))), (z, &A(4), Some(&B(3)), None)]);
        assert_join_eq!((&a_storage, Optional(&mut b_storage), Optional(&c_storage)),
                        vec![(v, &A(1), Some(&mut B(1)), Some(&C(1))), (x, &A(2), Some(&mut B(2)), Some(&C(2))), (y, &A(3), None, Some(&C(3))), (z, &A(4), Some(&mut B(3)), None)]);
        assert_join_eq!((&a_storage, Optional(&b_storage), Optional(&mut c_storage)),
                        vec![(v, &A(1), Some(&B(1)), Some(&mut C(1))), (x, &A(2), Some(&B(2)), Some(&mut C(2))), (y, &A(3), None, Some(&mut C(3))), (z, &A(4), Some(&B(3)), None)]);
        assert_join_eq!((&a_storage, Optional(&mut b_storage), Optional(&mut c_storage)),
                        vec![(v, &A(1), Some(&mut B(1)), Some(&mut C(1))), (x, &A(2), Some(&mut B(2)), Some(&mut C(2))), (y, &A(3), None, Some(&mut C(3))), (z, &A(4), Some(&mut B(3)), None)]);

        assert_join_eq!((&a_storage, &b_storage, Optional(&c_storage)),
                        vec![(v, &A(1), &B(1), Some(&C(1))), (x, &A(2), &B(2), Some(&C(2))), (z, &A(4), &B(3), None)]);
        assert_join_eq!((&a_storage, &mut b_storage, Optional(&c_storage)),
                        vec![(v, &A(1), &mut B(1), Some(&C(1))), (x, &A(2), &mut B(2), Some(&C(2))), (z, &A(4), &mut B(3), None)]);
        assert_join_eq!((&a_storage, &b_storage, Optional(&mut c_storage)),
                        vec![(v, &A(1), &B(1), Some(&mut C(1))), (x, &A(2), &B(2), Some(&mut C(2))), (z, &A(4), &B(3), None)]);
        assert_join_eq!((&a_storage, &mut b_storage, Optional(&mut c_storage)),
                        vec![(v, &A(1), &mut B(1), Some(&mut C(1))), (x, &A(2), &mut B(2), Some(&mut C(2))), (z, &A(4), &mut B(3), None)]);

        assert_join_eq!((&a_storage, Optional(&b_storage), &c_storage),
                        vec![(v, &A(1), Some(&B(1)), &C(1)), (x, &A(2), Some(&B(2)), &C(2)), (y, &A(3), None, &C(3))]);
        assert_join_eq!((&a_storage, Optional(&mut b_storage), &c_storage),
                        vec![(v, &A(1), Some(&mut B(1)), &C(1)), (x, &A(2), Some(&mut B(2)), &C(2)), (y, &A(3), None, &C(3))]);
        assert_join_eq!((&a_storage, Optional(&b_storage), &mut c_storage),
                        vec![(v, &A(1), Some(&B(1)), &mut C(1)), (x, &A(2), Some(&B(2)), &mut C(2)), (y, &A(3), None, &mut C(3))]);
        assert_join_eq!((&a_storage, Optional(&mut b_storage), &mut c_storage),
                        vec![(v, &A(1), Some(&mut B(1)), &mut C(1)), (x, &A(2), Some(&mut B(2)), &mut C(2)), (y, &A(3), None, &mut C(3))]);

        assert_join_eq!((&mut a_storage, Optional(&b_storage), Optional(&c_storage)),
                        vec![(v, &mut A(1), Some(&B(1)), Some(&C(1))), (x, &mut A(2), Some(&B(2)), Some(&C(2))), (y, &mut A(3), None, Some(&C(3))), (z, &mut A(4), Some(&B(3)), None)]);
        assert_join_eq!((&mut a_storage, Optional(&mut b_storage), Optional(&c_storage)),
                        vec![(v, &mut A(1), Some(&mut B(1)), Some(&C(1))), (x, &mut A(2), Some(&mut B(2)), Some(&C(2))), (y, &mut A(3), None, Some(&C(3))), (z, &mut A(4), Some(&mut B(3)), None)]);
        assert_join_eq!((&mut a_storage, Optional(&b_storage), Optional(&mut c_storage)),
                        vec![(v, &mut A(1), Some(&B(1)), Some(&mut C(1))), (x, &mut A(2), Some(&B(2)), Some(&mut C(2))), (y, &mut A(3), None, Some(&mut C(3))), (z, &mut A(4), Some(&B(3)), None)]);
        assert_join_eq!((&mut a_storage, Optional(&mut b_storage), Optional(&mut c_storage)),
                        vec![(v, &mut A(1), Some(&mut B(1)), Some(&mut C(1))), (x, &mut A(2), Some(&mut B(2)), Some(&mut C(2))), (y, &mut A(3), None, Some(&mut C(3))), (z, &mut A(4), Some(&mut B(3)), None)]);

        assert_join_eq!((&mut a_storage, &b_storage, Optional(&c_storage)),
                        vec![(v, &mut A(1), &B(1), Some(&C(1))), (x, &mut A(2), &B(2), Some(&C(2))), (z, &mut A(4), &B(3), None)]);
        assert_join_eq!((&mut a_storage, &mut b_storage, Optional(&c_storage)),
                        vec![(v, &mut A(1), &mut B(1), Some(&C(1))), (x, &mut A(2), &mut B(2), Some(&C(2))), (z, &mut A(4), &mut B(3), None)]);
        assert_join_eq!((&mut a_storage, &b_storage, Optional(&mut c_storage)),
                        vec![(v, &mut A(1), &B(1), Some(&mut C(1))), (x, &mut A(2), &B(2), Some(&mut C(2))), (z, &mut A(4), &B(3), None)]);
        assert_join_eq!((&mut a_storage, &mut b_storage, Optional(&mut c_storage)),
                        vec![(v, &mut A(1), &mut B(1), Some(&mut C(1))), (x, &mut A(2), &mut B(2), Some(&mut C(2))), (z, &mut A(4), &mut B(3), None)]);

        assert_join_eq!((&mut a_storage, Optional(&b_storage), &c_storage),
                        vec![(v, &mut A(1), Some(&B(1)), &C(1)), (x, &mut A(2), Some(&B(2)), &C(2)), (y, &mut A(3), None, &C(3))]);
        assert_join_eq!((&mut a_storage, Optional(&mut b_storage), &c_storage),
                        vec![(v, &mut A(1), Some(&mut B(1)), &C(1)), (x, &mut A(2), Some(&mut B(2)), &C(2)), (y, &mut A(3), None, &C(3))]);
        assert_join_eq!((&mut a_storage, Optional(&b_storage), &mut c_storage),
                        vec![(v, &mut A(1), Some(&B(1)), &mut C(1)), (x, &mut A(2), Some(&B(2)), &mut C(2)), (y, &mut A(3), None, &mut C(3))]);
        assert_join_eq!((&mut a_storage, Optional(&mut b_storage), &mut c_storage),
                        vec![(v, &mut A(1), Some(&mut B(1)), &mut C(1)), (x, &mut A(2), Some(&mut B(2)), &mut C(2)), (y, &mut A(3), None, &mut C(3))]);
    }
}

#[test]
fn universe_join_is_consistent_with_join() {
    let universe = Universe::default();
    let TestData {
        v,
        x,
        a_storage,
        b_storage,
        c_storage,
        ..
    } = TestData::new_for_universe(&universe);

    let mut universe = Universe::default();
    universe.insert_storage(a_storage);
    universe.insert_storage(b_storage);
    universe.insert_storage(c_storage);

    // Just check that the results are consistent with what we expect for one example of each case shared/mutable.
    // The join operation itself is more thoroughly tested separately
    let abc_join: Vec<_> = universe.join::<(&A, &B, &C)>().collect();
    assert_eq!(abc_join, vec![(v, &A(1), &B(1), &C(1)), (x, &A(2), &B(2), &C(2))]);

    let abc_join_mut: Vec<_> = universe.join_mut::<(&mut A, &mut B, &mut C)>().collect();
    assert_eq!(
        abc_join_mut,
        vec![
            (v, &mut A(1), &mut B(1), &mut C(1)),
            (x, &mut A(2), &mut B(2), &mut C(2))
        ]
    );
}
