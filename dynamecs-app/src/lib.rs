//! Opinionated framework for building simulation apps with `dynamecs`.
use checkpointing::{compressed_binary_checkpointing_system, restore_checkpoint_file};
use clap::Parser;
use cli::CliOptions;
use dynamecs::components::{
    get_simulation_time, get_step_index, register_default_components, DynamecsAppSettings, SimulationTime, StepIndex,
    TimeStep,
};
use dynamecs::storages::{ImmutableSingularStorage, SingularStorage};
use dynamecs::{register_component, Component, System, Systems, Universe};
use eyre::{eyre, Context};
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use tracing::{debug, info, info_span, instrument, warn};

pub extern crate eyre;
pub extern crate serde;
pub extern crate tracing;

mod checkpointing;
mod cli;
mod config_override;
mod tracing_impl;

pub use tracing_impl::register_signal_handler;
pub use tracing_impl::setup_tracing;

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
    pub fn from_config_and_app_settings(config: Config) -> Self {
        Self {
            config,
            scenario: None,
            dt_override: None,
            max_steps: None,
            restore_from_checkpoint: None,
            checkpoint_system: None,
        }
    }

    pub fn with_scenario_initializer<I>(mut self, initializer: I) -> eyre::Result<Self>
    where
        I: FnOnce(&Config) -> eyre::Result<Scenario>,
    {
        let mut scenario = initializer(&self.config)?;

        let scenario_name = scenario.name().to_string();
        let app_settings = DynamecsAppSettings {
            scenario_output_dir: get_output_dir().join(&scenario_name),
            scenario_name,
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

    /// Enables or disables writing checkpoints for the app.
    pub fn write_checkpoints(mut self, enable_write_checkpoints: bool) -> Self {
        self.checkpoint_system = enable_write_checkpoints.then(|| compressed_binary_checkpointing_system().into());
        self
    }

    /// Restores a checkpoint from the given file when the app is run.
    pub fn restore_checkpoint<P: Into<PathBuf>>(mut self, checkpoint_path: P) -> Self {
        self.restore_from_checkpoint = Some(checkpoint_path.into());
        self
    }

    #[instrument(level = "info", skip_all)]
    pub fn run(mut self) -> eyre::Result<()> {
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

                // Note: We enter the step span *after* checking if we should abort the loop,
                // so that we don't get an additional step span in the logs
                let _span = info_span!("step", step_index).entered();


                if step_index == 0 {
                    // Post systems must run on the initial state in order to do post-initialization
                    // For example, a system that outputs data after every simulation step should
                    // also output the initial state
                    debug!("Running post-systems for initial state");
                    {
                        let _span = info_span!("post_systems").entered();
                        scenario.post_systems.run_all(state)?;
                    }
                }

                // TODO: Use some more better formatting here...
                info!(
                    "Starting step {} at simulation time {:3.5} (dt = {:3.5e})",
                    step_index, sim_time, dt
                );
                {
                    let _span = info_span!("pre_systems").entered();
                    scenario.pre_systems.run_all(state)?;
                }
                {
                    let _span = info_span!("simulation_systems").entered();
                    scenario.simulation_systems.run_all(state)?;
                }

                sim_time += dt;
                set_singular_component(state, SimulationTime(sim_time));
                set_singular_component(state, StepIndex(step_index + 1));

                {
                    let _span = info_span!("post_systems").entered();
                    scenario.post_systems.run_all(state)?;
                }

                if let Some(checkpoint_system) = &mut self.checkpoint_system {
                    checkpoint_system
                        .run(state)
                        .wrap_err("failed to run checkpointing system")?;
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

impl DynamecsApp<()> {
    pub fn configure_from_cli<Config>() -> eyre::Result<DynamecsApp<Config>>
    where
        Config: Serialize,
        for<'de> Config: Deserialize<'de>,
    {
        let opt = CliOptions::parse();

        info!("Output base path: {}", opt.output_dir.display());

        if opt.config_file.is_some() && opt.config_string.is_some() {
            return Err(eyre!("config file and config string are mutually exclusive"));
        }

        let initial_config: Config = if let Some(path) = opt.config_file {
            info!("Reading config file from {}.", path.display());
            let config_str =
                read_to_string(&path).wrap_err_with(|| format!("failed to read config file at {}", path.display()))?;
            json5::from_str(&config_str).wrap_err("failed to deserialize supplied JSON5 configuration file")
        } else if let Some(config_str) = opt.config_string {
            info!("Using configuration provided from CLI interface");
            json5::from_str(&config_str).wrap_err("failed to deserialize supplied JSON5 configuration string")
        } else {
            let default_config_str = "{}";
            info!(
                r#"No configuration specified. Trying to use the empty document {} as default."#,
                default_config_str
            );
            Ok(json5::from_str("{}").wrap_err(
                "failed to deserialize configuration from empty document {}. \
            You need to either provide all required configuration parameters, \
            or make sure that your configuration can be deserialized from an empty document,",
            )?)
        }?;

        let mut config_json =
            serde_json::to_value(initial_config).wrap_err("failed to serialize initial config as JSON")?;

        if !opt.overrides.is_empty() {
            let overridden_config: serde_json::Value =
                config_override::apply_config_overrides(config_json, &opt.overrides)?;
            config_json = serde_json::from_value(overridden_config).wrap_err_with(|| {
                "invalid config overrides: cannot deserialize configuration from \
                overridden configuration"
            })?;
        }

        // Emit warnings whenever we run into JSON fields that are not part of the
        // configuration
        let config: Config = serde_ignored::deserialize(&config_json, |path| {
            warn!(
                "Ignored unknown field {} during deserialization of configuration",
                path.to_string()
            );
        })
        .wrap_err_with(|| {
            let json_str = serde_json::to_string_pretty(&config_json)
                .unwrap_or_else(|err| format!("<failed to serialize to JSON: {err}>"));
            format!(
                "failed to deserialize the following JSON configuration \
                into a valid configuration: \n{json_str}"
            )
        })?;

        // TODO: We use serde_json because json5 cannot pretty-print JSON, and unfortunately
        // its serializer is limited to producing JSON
        let config_json_str = serde_json::to_string_pretty(&config)?;
        info!("Using configuration: \n{}", config_json_str);

        if let Some(dt) = opt.dt {
            if dt <= 0.0 {
                return Err(eyre!("time step dt must be positive"));
            }
        }

        let checkpoint_system = opt
            .write_checkpoints
            .then(|| compressed_binary_checkpointing_system().into());

        Ok(DynamecsApp {
            config,
            scenario: None,
            dt_override: opt.dt,
            max_steps: opt.max_steps,
            restore_from_checkpoint: opt.restore_checkpoint,
            checkpoint_system,
        })
    }
}

/// Returns the intended root directory for app output.
///
/// The returned path is relative to the current working directory.
pub fn get_output_dir() -> PathBuf {
    let cli_args = CliOptions::parse();
    cli_args.output_dir
}

/// Returns the *default* intended root directory for app output.
///
/// The returned path is relative to the current working directory.
///
/// This is the default path used when not overriden through the command-line interface.
/// Users would probably usually want to use [`get_output_dir`] instead.
pub fn get_default_output_dir() -> &'static Path {
    Path::new("output")
}

/// Convenience macro for generating an appropriate main function for use with `dynamecs-app`.
///
/// The macro sets up logging through the `tracing` integration, sets up a signal handler
/// to ensure clean log termination, configures a `DynamecsApp`
/// based on CLI arguments and runs the scenario defined by the given scenario initializer.
///
/// For example, consider the following program.
/// ```no_run
/// use serde::{Deserialize, Serialize};
/// use dynamecs_app::{dynamecs_main, Scenario};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct Config {
///     resolution: usize
/// }
///
/// fn initialize_scenario(_config: &Config) -> eyre::Result<Scenario> {
///     todo!()
/// }
///
/// dynamecs_main!(initialize_scenario);
/// ```
#[macro_export]
macro_rules! dynamecs_main {
    ($scenario:expr) => {
        fn main() -> Result<(), Box<dyn std::error::Error>> {
            let _tracing_guard = $crate::setup_tracing()?;
            $crate::register_signal_handler()?;
            fn main_internal() -> Result<(), Box<dyn std::error::Error>> {
                $crate::DynamecsApp::configure_from_cli()?
                    .with_scenario_initializer($scenario)?
                    .run()?;
                Ok(())
            }

            main_internal().map_err(|err| {
                let msg = if let Some(source) = err.source() {
                    format!("{err:#},\ncaused by: {}", source)
                } else {
                    format!("{err:#}")
                };
                $crate::tracing::error!("{msg}");
                err
            })
        }
    };
}
