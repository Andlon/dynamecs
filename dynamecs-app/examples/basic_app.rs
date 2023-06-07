use dynamecs::{System, Universe};
use dynamecs_app::{dynamecs_main, Scenario};
use serde::{Deserialize, Serialize};
use tracing::{debug, debug_span};

#[derive(Serialize, Deserialize)]
struct Config {}

fn initialize_scenario(_config: &Config) -> eyre::Result<Scenario> {
    let mut scenario = Scenario::default_with_name("basic");
    scenario.post_systems.add_system(SystemThatLogs);
    scenario.post_systems.add_system(OtherSystemThatLogs);
    Ok(scenario)
}

#[derive(Debug)]
struct SystemThatLogs;

#[derive(Debug)]
struct OtherSystemThatLogs;

impl System for SystemThatLogs {
    fn run(&mut self, _data: &mut Universe) -> eyre::Result<()> {
        let _span = debug_span!("system_that_logs", something = "test").entered();
        debug!(number = 42, "some debug info");
        Ok(())
    }
}

impl System for OtherSystemThatLogs {
    fn run(&mut self, _data: &mut Universe) -> eyre::Result<()> {
        let _span = debug_span!("other_system_that_logs", something = "other test").entered();
        debug!(digits = 17, "some other debug info");
        {
            let _span = debug_span!("subspan1").entered();
            {
                {
                    let _span = debug_span!("assembly").entered();
                    {
                        let _span = debug_span!("vector").entered();
                    }
                    {
                        let _span = debug_span!("matrix").entered();
                    }
                }
                {
                    let _span = debug_span!("solve").entered();
                }
            }
        }
        {
            let _span = debug_span!("subspan2").entered();
            {
                {
                    let _span2 = debug_span!("assembly").entered();
                }
                {
                    let _span3 = debug_span!("solve").entered();
                }
            }
        }
        Ok(())
    }
}

dynamecs_main!(initialize_scenario);
