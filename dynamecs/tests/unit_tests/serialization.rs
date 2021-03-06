use dynamecs::storages::VecStorage;
use dynamecs::{register_component, Component, Entity, Universe};

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Foo(i32);

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bar(i32);

impl Component for Foo {
    type Storage = VecStorage<Foo>;
}

impl Component for Bar {
    type Storage = VecStorage<Bar>;
}

struct TestData {
    universe: Universe,
    e1: Entity,
    e2: Entity,
    e3: Entity,
}

impl Default for TestData {
    fn default() -> Self {
        register_component::<Foo>();
        register_component::<Bar>();

        let mut universe = Universe::default();

        let e1 = universe.new_entity();
        let e2 = universe.new_entity();
        let e3 = universe.new_entity();

        {
            let foo_storage = universe.get_component_storage_mut::<Foo>();
            foo_storage.insert(e2, Foo(1));
            foo_storage.insert(e1, Foo(2));

            let bar_storage = universe.get_component_storage_mut::<Bar>();
            bar_storage.insert(e2, Bar(3));
            bar_storage.insert(e3, Bar(4));
            bar_storage.insert(e1, Bar(5));
        }

        Self { universe, e1, e2, e3 }
    }
}

#[test]
fn json_roundtrip() {
    let TestData { universe, e1, e2, e3 } = TestData::default();

    let json = serde_json::to_string_pretty(&universe).unwrap();

    // Drop universe so that we make sure we don't accidentally reference it later
    drop(universe);

    let deserialized_universe: Universe = serde_json::from_str(&json).unwrap();

    let foo_storage = deserialized_universe.get_component_storage::<Foo>();
    let bar_storage = deserialized_universe.get_component_storage::<Bar>();

    let foos = foo_storage.components();
    let bars = bar_storage.components();

    assert_eq!(foos, &[Foo(1), Foo(2)]);
    assert_eq!(bars, &[Bar(3), Bar(4), Bar(5)]);

    // Entities should be
    assert_eq!(foo_storage.entities(), &[e2, e1]);
    assert_eq!(bar_storage.entities(), &[e2, e3, e1]);

    // Entities describe relations, and we can therefore
    // check that the components that shared the same entities still do after
    // serialization and deserialization.
    let foo_ids = foo_storage.entities();
    let bar_ids = bar_storage.entities();
    assert_eq!(foo_ids[0], bar_ids[0]);
    assert_eq!(foo_ids[1], bar_ids[2]);

    // Check that a new entity is different from all our existing entities
    let new_entity = deserialized_universe.new_entity();
    assert_ne!(new_entity, e1);
    assert_ne!(new_entity, e2);
    assert_ne!(new_entity, e3);
}

#[test]
fn bincode_test() {
    // Basically the same as the JSON roundtrip test, but simplified/not as elaborate
    let TestData { universe, e1, e2, e3 } = TestData::default();
    let bincode = bincode::serialize(&universe).unwrap();
    let universe2: Universe = bincode::deserialize(&bincode).unwrap();

    assert_eq!(
        universe2.get_component_storage::<Foo>(),
        universe.get_component_storage::<Foo>()
    );
    assert_eq!(
        universe2.get_component_storage::<Bar>(),
        universe.get_component_storage::<Bar>()
    );
    let new_entity = universe2.new_entity();
    assert_ne!(new_entity, e1);
    assert_ne!(new_entity, e2);
    assert_ne!(new_entity, e3);
}
