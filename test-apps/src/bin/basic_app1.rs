use std::fmt::{Debug, Formatter};
use dynamecs::{System, Universe};
use dynamecs::components::TimeStep;
use dynamecs::storages::SingularStorage;
use dynamecs_app::{dynamecs_main, eyre, Scenario};
use dynamecs_app::serde::{Deserialize, Serialize};
use dynamecs_app::serde as serde;
use dynamecs_app::tracing::{info, debug, info_span, trace};

#[derive(Serialize, Deserialize)]
struct Config {

}

fn scenario(_: &Config) -> eyre::Result<Scenario> {
    let mut scenario = Scenario::default_with_name("basic_app1");
    scenario.duration = Some(0.2);
    scenario.state.insert_storage(SingularStorage::new(TimeStep(0.1)));
    // TODO: Also exercise pre/post systems
    scenario.simulation_systems.add_system(System1);
    info!("Initializing scenario");
    Ok(scenario)
}

#[derive(Debug)]
struct System1;

impl System for System1 {
    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        debug!(target: "target1", answer = 42, "debug-test");
        let _span = info_span!(target: "target1", "span1").entered();
        let _span2 = info_span!(target: "target2", "span2").entered();
        trace!(target: "target2", question = "jeopardy", "trace-test");
        Ok(())
    }
}

dynamecs_main!(scenario);
