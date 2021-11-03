use dynamecs::storages::VecStorage;
use dynamecs::{register_component, Component, Entity, Universe};

use std::error::Error;

use serde::{Deserialize, Serialize};

use serde_json;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TestComponent(pub usize);

impl Component for TestComponent {
    type Storage = VecStorage<Self>;
}

fn main() -> Result<(), Box<dyn Error>> {
    register_component::<TestComponent>()?;

    let mut universe = Universe::default();

    let storage = universe.get_component_storage_mut::<TestComponent>();
    storage.insert(Entity::new(), TestComponent(0));
    storage.insert(Entity::new(), TestComponent(1));

    let json = serde_json::to_string_pretty(&universe)?;

    println!("{}", json);

    let deserialized_universe: Universe = serde_json::from_str(&json)?;

    let storage = deserialized_universe.get_component_storage::<TestComponent>();
    dbg!(storage);

    Ok(())
}
