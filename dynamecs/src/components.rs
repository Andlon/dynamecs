//! Predefined components commonly used by simulators.

use crate::storages::{ImmutableSingularStorage, SingularStorage, VecStorage};
use crate::{register_component, Component, Universe};
use eyre::eyre;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::ops::Deref;
use std::path::PathBuf;

/// Registers the "default" components [`Name`], [`TimeStep`], [`SimulationTime`] and [`StepIndex`].
pub fn register_default_components() -> eyre::Result<()> {
    register_component::<Name>()?;
    register_component::<TimeStep>()?;
    register_component::<SimulationTime>()?;
    register_component::<StepIndex>()?;

    Ok(())
}

/// Associates an entity with a name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeStep(pub f64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationTime(pub f64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepIndex(pub usize);

impl Component for Name {
    type Storage = VecStorage<Self>;
}

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> From<&'a str> for Name {
    fn from(s: &'a str) -> Self {
        Name(String::from(s))
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Name(s)
    }
}

impl Default for TimeStep {
    fn default() -> Self {
        Self(1.0 / 60.0)
    }
}

impl Component for TimeStep {
    type Storage = SingularStorage<Self>;
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Component for SimulationTime {
    type Storage = SingularStorage<Self>;
}

impl Default for StepIndex {
    fn default() -> Self {
        Self(0)
    }
}

impl Component for StepIndex {
    type Storage = SingularStorage<Self>;
}

pub fn get_simulation_time(state: &Universe) -> SimulationTime {
    state
        .get_component_storage::<SimulationTime>()
        .get_component()
        .clone()
}

pub fn get_step_index(state: &Universe) -> StepIndex {
    state
        .get_component_storage::<StepIndex>()
        .get_component()
        .clone()
}

pub fn try_get_timestep(state: &Universe) -> eyre::Result<TimeStep> {
    let storage = state
        .try_get_component_storage::<TimeStep>()
        .ok_or_else(|| eyre!("component TimeStep not found in Universe instance"))?;
    Ok(storage.get_component().clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamecsAppSettings {
    pub output_folder: PathBuf,
    pub scenario_name: String,
}

impl Component for DynamecsAppSettings {
    type Storage = ImmutableSingularStorage<Self>;
}

pub fn try_get_settings(state: &Universe) -> eyre::Result<&DynamecsAppSettings> {
    let storage = state
        .try_get_component_storage::<DynamecsAppSettings>()
        .ok_or_else(|| eyre!("component DynamecsAppSettings not found in Universe instance"))?;
    Ok(storage.get_component())
}
