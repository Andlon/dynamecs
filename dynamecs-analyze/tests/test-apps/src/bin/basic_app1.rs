use std::fmt::{Debug, Formatter};
use dynamecs::{System, Universe};
use dynamecs::components::TimeStep;
use dynamecs::storages::SingularStorage;
use dynamecs_app::{dynamecs_main, eyre, Scenario};
use dynamecs_app::serde::{Deserialize, Serialize};
use dynamecs_app::serde as serde;
use dynamecs_app::tracing::info;

#[derive(Serialize, Deserialize)]
struct Config {

}

fn scenario(_: &Config) -> eyre::Result<Scenario> {
    let mut scenario = Scenario::default_with_name("basic_app1");
    scenario.duration = Some(0.2);
    scenario.state.insert_storage(SingularStorage::new(TimeStep(0.1)));
    info!("Initializing scenario");
    Ok(scenario)
}

#[derive(Debug)]
struct System1;

impl System for System1 {
    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        Ok(())
    }
}

dynamecs_main!(scenario);
