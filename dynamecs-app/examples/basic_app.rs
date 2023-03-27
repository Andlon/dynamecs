use dynamecs_app::{dynamecs_main, Scenario};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {

}

fn initialize_scenario(_config: &Config) -> eyre::Result<Scenario> {
    Ok(Scenario::default_with_name("basic"))
}

dynamecs_main!(initialize_scenario);