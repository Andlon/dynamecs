use super::dummy_components::{A, B, C, D, E, F, G, H};
use cool_asserts::assert_panics;
use dynamecs::{Component, Universe};

type StorageFor<C> = <C as Component>::Storage;
type S<C> = StorageFor<C>;

#[test]
fn get_component_storages_compiles_for_tuple_arguments() {
    let universe = Universe::default();

    let (_,): (&S<A>,) = universe.get_component_storages::<(&A,)>();
    let (_, _): (&S<A>, &S<B>) = universe.get_component_storages::<(&A, &B)>();
    let (_, _, _): (&S<A>, &S<B>, &S<C>) = universe.get_component_storages::<(&A, &B, &C)>();
    let (_, _, _, _): (&S<A>, &S<B>, &S<C>, &S<D>) =
        universe.get_component_storages::<(&A, &B, &C, &D)>();
    let (_, _, _, _, _): (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>) =
        universe.get_component_storages::<(&A, &B, &C, &D, &E)>();
    let (_, _, _, _, _, _): (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>, &S<F>) =
        universe.get_component_storages::<(&A, &B, &C, &D, &E, &F)>();
    let (_, _, _, _, _, _, _): (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>, &S<F>, &S<G>) =
        universe.get_component_storages::<(&A, &B, &C, &D, &E, &F, &G)>();
    let (_, _, _, _, _, _, _, _): (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>, &S<F>, &S<G>, &S<H>) =
        universe.get_component_storages::<(&A, &B, &C, &D, &E, &F, &G, &H)>();
}

#[test]
fn get_component_storages_mut_compiles_for_tuple_arguments() {
    let mut universe = Universe::default();

    // 1-element tuple
    let _: (&S<A>,) = universe.get_component_storages_mut::<(&A,)>();
    let _: (&mut S<A>,) = universe.get_component_storages_mut::<(&mut A,)>();

    // 2-element tuple
    let _: (&S<A>, &S<B>) = universe.get_component_storages_mut::<(&A, &B)>();
    let _: (&mut S<A>, &S<B>) = universe.get_component_storages_mut::<(&mut A, &B)>();
    let _: (&S<A>, &mut S<B>) = universe.get_component_storages_mut::<(&A, &mut B)>();
    let _: (&mut S<A>, &mut S<B>) = universe.get_component_storages_mut::<(&mut A, &mut B)>();

    // 3-element tuple
    let _: (&S<A>, &S<B>, &S<C>) = universe.get_component_storages_mut::<(&A, &B, &C)>();
    let _: (&mut S<A>, &S<B>, &S<C>) = universe.get_component_storages_mut::<(&mut A, &B, &C)>();
    let _: (&S<A>, &mut S<B>, &S<C>) = universe.get_component_storages_mut::<(&A, &mut B, &C)>();
    let _: (&S<A>, &S<B>, &mut S<C>) = universe.get_component_storages_mut::<(&A, &B, &mut C)>();
    let _: (&mut S<A>, &mut S<B>, &S<C>) =
        universe.get_component_storages_mut::<(&mut A, &mut B, &C)>();
    let _: (&S<A>, &mut S<B>, &mut S<C>) =
        universe.get_component_storages_mut::<(&A, &mut B, &mut C)>();
    let _: (&mut S<A>, &S<B>, &mut S<C>) =
        universe.get_component_storages_mut::<(&mut A, &B, &mut C)>();
    let _: (&mut S<A>, &mut S<B>, &mut S<C>) =
        universe.get_component_storages_mut::<(&mut A, &mut B, &mut C)>();

    // For larger tuples the number of combinations become too large, therefore we only
    // test a few combinations

    // 4-element tuple
    let _: (&S<A>, &S<B>, &S<C>, &S<D>) = universe.get_component_storages_mut::<(&A, &B, &C, &D)>();
    let _: (&mut S<A>, &S<B>, &mut S<C>, &S<D>) =
        universe.get_component_storages_mut::<(&mut A, &B, &mut C, &D)>();
    let _: (&mut S<A>, &mut S<B>, &mut S<C>, &mut S<D>) =
        universe.get_component_storages_mut::<(&mut A, &mut B, &mut C, &mut D)>();

    // 5-element tuple
    let _: (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>) =
        universe.get_component_storages_mut::<(&A, &B, &C, &D, &E)>();
    let _: (&mut S<A>, &S<B>, &mut S<C>, &S<D>, &S<E>) =
        universe.get_component_storages_mut::<(&mut A, &B, &mut C, &D, &E)>();
    let _: (&mut S<A>, &mut S<B>, &mut S<C>, &mut S<D>, &mut S<E>) =
        universe.get_component_storages_mut::<(&mut A, &mut B, &mut C, &mut D, &mut E)>();

    // 6-element tuple
    let _: (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>, &S<F>) =
        universe.get_component_storages_mut::<(&A, &B, &C, &D, &E, &F)>();
    let _: (&mut S<A>, &S<B>, &mut S<C>, &S<D>, &S<E>, &mut S<F>) =
        universe.get_component_storages_mut::<(&mut A, &B, &mut C, &D, &E, &mut F)>();
    let _: (
        &mut S<A>,
        &mut S<B>,
        &mut S<C>,
        &mut S<D>,
        &mut S<E>,
        &mut S<F>,
    ) = universe.get_component_storages_mut::<(&mut A, &mut B, &mut C, &mut D, &mut E, &mut F)>();

    // 7-element tuple
    let _: (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>, &S<F>, &S<G>) =
        universe.get_component_storages_mut::<(&A, &B, &C, &D, &E, &F, &G)>();
    let _: (&mut S<A>, &S<B>, &mut S<C>, &S<D>, &S<E>, &mut S<F>, &S<G>) =
        universe.get_component_storages_mut::<(&mut A, &B, &mut C, &D, &E, &mut F, &G)>();
    let _: (
        &mut S<A>,
        &mut S<B>,
        &mut S<C>,
        &mut S<D>,
        &mut S<E>,
        &mut S<F>,
        &mut S<G>,
    ) = universe
        .get_component_storages_mut::<(&mut A, &mut B, &mut C, &mut D, &mut E, &mut F, &mut G)>();

    // 8-element tuple
    let _: (&S<A>, &S<B>, &S<C>, &S<D>, &S<E>, &S<F>, &S<G>, &S<H>) =
        universe.get_component_storages_mut::<(&A, &B, &C, &D, &E, &F, &G, &H)>();
    let _: (
        &mut S<A>,
        &S<B>,
        &mut S<C>,
        &S<D>,
        &S<E>,
        &mut S<F>,
        &S<G>,
        &mut S<H>,
    ) = universe.get_component_storages_mut::<(&mut A, &B, &mut C, &D, &E, &mut F, &G, &mut H)>();
    let _: (
        &mut S<A>,
        &mut S<B>,
        &mut S<C>,
        &mut S<D>,
        &mut S<E>,
        &mut S<F>,
        &mut S<G>,
        &mut S<H>,
    ) = universe.get_component_storages_mut::<(
        &mut A,
        &mut B,
        &mut C,
        &mut D,
        &mut E,
        &mut F,
        &mut G,
        &mut H,
    )>();
}

#[test]
fn get_component_storages_mut_panics_if_duplicate_arguments_provided() {
    let expected_msg =
        "Stopped attempt to obtain multiple mutable references to the same storage. \
    Can not simultaneously mutably borrow the same storage type multiple times.";

    assert_panics!(
        {
            let _ = Universe::default().get_component_storages_mut::<(&mut A, &mut A)>();
        },
        includes(expected_msg)
    );

    assert_panics!(
        {
            let _ = Universe::default().get_component_storages_mut::<(&mut A, &A)>();
        },
        includes(expected_msg)
    );

    assert_panics!(
        {
            let _ = Universe::default().get_component_storages_mut::<(&mut A, &B, &A, &mut C)>();
        },
        includes(expected_msg)
    );
}
