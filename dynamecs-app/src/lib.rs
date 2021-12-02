//! Staging area for developing things that later will be moved to dynamecs
use checkpointing::{compressed_binary_checkpointing_system, restore_checkpoint_file};
use dynamecs::components::{
    get_simulation_time, get_step_index, register_default_components, DynamecsAppSettings, SimulationTime, StepIndex,
    TimeStep,
};
use dynamecs::storages::{ImmutableSingularStorage, SingularStorage};
use dynamecs::{register_component, Component, System, Systems, Universe};
use eyre::{eyre, Context};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

mod checkpointing;

#[derive(Debug)]
pub struct Scenario {
    name: String,
    pub duration: Option<f64>,
    pub state: Universe,
    pub pre_systems: Systems,
    pub simulation_systems: Systems,
    pub post_systems: Systems,
}

impl Scenario {
    pub fn default_with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration: None,
            state: Default::default(),
            pre_systems: Default::default(),
            simulation_systems: Default::default(),
            post_systems: Default::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct DynamecsApp<Config = ()> {
    app_settings: DynamecsAppSettings,
    config: Config,
    scenario: Option<Scenario>,
    /// Optionally override the time step dt (otherwise use scenario-provided or default)
    dt_override: Option<f64>,
    max_steps: Option<usize>,
    /// Optionally restore the simulation state from the given checkpoint file
    restore_from_checkpoint: Option<PathBuf>,
    /// Optional system for writing checkpoints
    checkpoint_system: Option<Box<dyn System>>,
}

impl<Config> DynamecsApp<Config> {
    pub fn from_config_and_app_settings(config: Config, app_settings: DynamecsAppSettings) -> Self {
        Self {
            app_settings,
            config,
            scenario: None,
            dt_override: None,
            max_steps: None,
            restore_from_checkpoint: None,
            checkpoint_system: None,
        }
    }

    pub fn with_scenario_initializer<I>(&mut self, initializer: I) -> eyre::Result<&mut Self>
    where
        I: FnOnce(&Config) -> eyre::Result<Scenario>,
    {
        let mut scenario = initializer(&self.config)?;

        let scenario_name = scenario.name().to_string();
        let app_settings = DynamecsAppSettings {
            output_folder: self.app_settings.output_folder.join(scenario_name),
            scenario_name: scenario.name().to_string(),
        };

        scenario
            .state
            .insert_storage(ImmutableSingularStorage::new(app_settings));

        if let Some(dt) = self.dt_override {
            info!("Overriding time step dt = {}", dt);
            scenario
                .state
                .insert_storage(SingularStorage::new(TimeStep(dt)));
        }

        self.scenario = Some(scenario);
        Ok(self)
    }

    /// Enables writing checkpoints for the app.
    pub fn with_write_checkpoints(&mut self) -> &mut Self {
        let checkpoint_system = compressed_binary_checkpointing_system();
        self.checkpoint_system = Some(checkpoint_system.into());
        self
    }

    /// Restores a checkpoint from the given file when the app is run.
    pub fn with_restore_checkpoint<P: Into<PathBuf>>(&mut self, checkpoint_path: P) -> &mut Self {
        self.restore_from_checkpoint = Some(checkpoint_path.into());
        self
    }

    pub fn run(&mut self) -> eyre::Result<()> {
        if let Some(scenario) = &mut self.scenario {
            // Register components of all systems
            register_default_components();
            register_component::<DynamecsAppSettings>();
            scenario.pre_systems.register_components();
            scenario.simulation_systems.register_components();
            scenario.post_systems.register_components();

            if let Some(checkpoint_path) = &self.restore_from_checkpoint {
                let universe = restore_checkpoint_file(checkpoint_path)?;
                scenario.state = universe;

                let step_index = get_step_index(&scenario.state).0;
                info!(
                    "Restored simulation state with step index {} from file \"{}\"",
                    step_index,
                    checkpoint_path.display()
                );
            }

            info!("Starting simulation of scenario \"{}\"", scenario.name());
            loop {
                let state = &mut scenario.state;
                let SimulationTime(mut sim_time) = get_simulation_time(&*state);
                let StepIndex(step_index) = get_step_index(&*state);
                let TimeStep(dt) = get_time_step_or_set_default(state);

                if let Some(max_steps) = self.max_steps {
                    if step_index > max_steps {
                        break;
                    }
                } else if let Some(duration) = scenario.duration {
                    if sim_time >= duration {
                        break;
                    }
                }

                if step_index == 0 {
                    // Post systems must run on the initial state in order to do post-initialization
                    // For example, a system that outputs data after every simulation step should
                    // also output the initial state
                    debug!("Running post-systems for initial state");
                    scenario.post_systems.run_all(state)?;
                }

                // TODO: Use some more better formatting here...
                info!(
                    "Starting step {} at simulation time {:3.5} (dt = {:3.5e})",
                    step_index, sim_time, dt
                );
                scenario.pre_systems.run_all(state)?;
                scenario.simulation_systems.run_all(state)?;

                sim_time += dt;
                set_singular_component(state, SimulationTime(sim_time));
                set_singular_component(state, StepIndex(step_index + 1));

                scenario.post_systems.run_all(state)?;

                if let Some(checkpoint_system) = &mut self.checkpoint_system {
                    checkpoint_system.run(state).wrap_err("failed to run checkpointing system")?;
                }
            }

            info!("Simulation ended");
            Ok(())
        } else {
            Err(eyre!("cannot run scenario: no scenario initializer provided",))
        }
    }
}

fn set_singular_component<C>(state: &mut Universe, component: C)
where
    C: Serialize + for<'de> Deserialize<'de>,
    C: Component<Storage = SingularStorage<C>>,
{
    state.insert_storage(SingularStorage::new(component));
}

fn get_time_step_or_set_default(state: &mut Universe) -> TimeStep {
    if let Some(storage) = state.try_get_component_storage::<TimeStep>() {
        storage.get_component().clone()
    } else {
        let default_dt = state.get_component_storage::<TimeStep>().get_component();
        info!("No time step configured. Using default dt = {}", default_dt.0);
        default_dt.clone()
    }
}

#[derive(StructOpt)]
struct CliOptions {
    #[structopt(
        short,
        long,
        help = "The path (relative or absolute) to a scenario-specific configuration file."
    )]
    config_file: Option<PathBuf>,
    #[structopt(
        short = "o",
        long = "output-dir",
        help = "Output base directory, relative or absolute.",
        default_value = "output"
    )]
    output_dir: PathBuf,
    #[structopt(long = "dt", help = "Override the time step used for the simulation.")]
    dt: Option<f64>,
    #[structopt(
        long = "max-steps",
        help = "Maximum number of simulation steps to take (by default infinite)"
    )]
    max_steps: Option<usize>,
    #[structopt(
        long = "write-checkpoints",
        help = "Write a checkpoint file to disk after every timestep"
    )]
    write_checkpoints: bool,
    #[structopt(
        long = "restore-checkpoint",
        help = "Restore the simulation state from a checkpoint file and continue the simulation"
    )]
    restore_checkpoint: Option<PathBuf>,
}

impl DynamecsApp<()> {
    pub fn configure_from_cli<Config>() -> eyre::Result<DynamecsApp<Config>>
    where
        Config: Default + Serialize,
        for<'de> Config: Deserialize<'de>,
    {
        let opt = CliOptions::from_args();

        info!("Output base path: {}", opt.output_dir.display());

        let config = if let Some(path) = opt.config_file {
            let file = File::open(&path)?;
            let config = serde_json::from_reader(file)?;
            info!("Read config file from {}.", path.display());
            config
        } else {
            info!("No configuration specified. Using default configuration.");
            Config::default()
        };
        let config_json_str = serde_json::to_string_pretty(&config)?;
        info!("Using configuration: \n{}", config_json_str);

        let app_settings = DynamecsAppSettings {
            output_folder: opt.output_dir,
            scenario_name: "Unnamed".to_string(),
        };

        if let Some(dt) = opt.dt {
            if dt <= 0.0 {
                return Err(eyre!("time step dt must be positive"));
            }
        }

        let checkpoint_system = opt
            .write_checkpoints
            .then(|| compressed_binary_checkpointing_system().into());

        // TODO: Support configuration string from CLI
        Ok(DynamecsApp {
            app_settings,
            config,
            scenario: None,
            dt_override: opt.dt,
            max_steps: opt.max_steps,
            restore_from_checkpoint: opt.restore_checkpoint,
            checkpoint_system,
        })
    }
}
