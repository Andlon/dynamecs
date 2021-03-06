//! Generic adapters for systems.
use eyre::eyre;
use std::fmt;
use std::fmt::{Debug, Display};

use crate::components::get_simulation_time;
use crate::{System, Universe};

/// Adapts a `Fn` or `FnMut` closure as a [`System`].
pub struct FnSystem<F>
where
    F: FnMut(&mut Universe) -> eyre::Result<()>,
{
    name: String,
    fun: F,
}

/// Adapts a `FnOnce` closure as a [`System`].
///
/// The closure is only run once and dropped afterwards.
pub struct FnOnceSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    name: String,
    closure: Option<F>,
    has_run: bool,
}

/// Wrapper system that ensures the wrapped [`System`] can run only once.
///
/// The wrapped system is guaranteed to be run only once. Subsequent calls to the `run` function will just return `Ok`
/// without running the wrapped system.
pub struct SingleShotSystem<S: System> {
    /// Wrapped system.
    pub system: S,
    has_run: bool,
}

/// Filter system that uses a closure to determine if the wrapped system should be run each timestep.
pub struct FilterSystem<P, S>
where
    P: FnMut(&Universe) -> eyre::Result<bool>,
    S: System,
{
    /// Wrapped system.
    pub system: S,
    /// Predicate closure used for the filtering.
    pub predicate: P,
}

/// Wrapper system that only runs starting from the timestep when the [`SimulationTime`](`crate::components::SimulationTime`) reached the specified activation time.
pub struct DelayedSystem<S: System> {
    system: S,
    activation_time: f64,
}

impl<S: System> DelayedSystem<S> {
    pub fn new(system: S, activation_time: f64) -> Self {
        DelayedSystem {
            system,
            activation_time,
        }
    }
}

/// Wrapper to store a vector of systems that are run in sequence.
pub struct SystemCollection(pub Vec<Box<dyn System>>);

impl<F> FnSystem<F>
where
    F: FnMut(&mut Universe) -> eyre::Result<()>,
{
    pub fn new<S: Into<String>>(name: S, f: F) -> Self {
        Self {
            name: name.into(),
            fun: f,
        }
    }
}

impl<F> Debug for FnSystem<F>
where
    F: FnMut(&mut Universe) -> eyre::Result<()>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FnOnceSystem(name: {})", self.name)
    }
}

impl<F> Display for FnSystem<F>
where
    F: FnMut(&mut Universe) -> eyre::Result<()>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FnSystem(name: {})", self.name)
    }
}

impl<F> System for FnSystem<F>
where
    F: FnMut(&mut Universe) -> eyre::Result<()>,
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        (self.fun)(data)
    }
}

impl<F> FnOnceSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    /// Constructs a new system with the given name from the given `FnOnce` closure.
    pub fn new<S: Into<String>>(name: S, f: F) -> Self {
        Self {
            name: name.into(),
            closure: Some(f),
            has_run: false,
        }
    }

    /// Returns whether the system already has run.
    pub fn has_run(&self) -> bool {
        self.has_run
    }
}

impl<F> Debug for FnOnceSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FnOnceSystem(name: {}, has_run: {})", self.name, self.has_run)
    }
}

impl<F> Display for FnOnceSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FnOnceSystem(name: {}, has_run: {})", self.name, self.has_run)
    }
}

impl<F> System for FnOnceSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if !self.has_run {
            let ret = (self.closure.take().ok_or_else(|| eyre!("closure gone"))?)(data)?;
            self.has_run = true;
            Ok(ret)
        } else {
            Ok(())
        }
    }
}

impl<S: System> SingleShotSystem<S> {
    pub fn new(system: S) -> Self {
        SingleShotSystem {
            system: system,
            has_run: false,
        }
    }

    /// Returns whether the system already has run.
    pub fn has_run(&self) -> bool {
        self.has_run
    }
}

impl<S: System> Debug for SingleShotSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SingleShotSystem(has_run: {})", self.has_run)
    }
}

impl<S: System> Display for SingleShotSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SingleShotSystem(has_run: {})", self.has_run)
    }
}

impl<S: System> System for SingleShotSystem<S> {
    fn name(&self) -> String {
        format!("SingleShotSystem({})", self.system.name())
    }

    fn register_components(&self) {
        self.system.register_components();
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if !self.has_run {
            let ret = self.system.run(data)?;
            self.has_run = true;
            Ok(ret)
        } else {
            Ok(())
        }
    }
}

impl<P, S> FilterSystem<P, S>
where
    P: FnMut(&Universe) -> eyre::Result<bool>,
    S: System,
{
    pub fn new(system: S, predicate: P) -> Self {
        Self { system, predicate }
    }
}

impl<P, S> Debug for FilterSystem<P, S>
where
    P: FnMut(&Universe) -> eyre::Result<bool>,
    S: System,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Filter({:?})", self.system)
    }
}

impl<P, S> Display for FilterSystem<P, S>
where
    P: FnMut(&Universe) -> eyre::Result<bool>,
    S: System,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Filter({})", self.system.name())
    }
}

impl<P, S> System for FilterSystem<P, S>
where
    P: FnMut(&Universe) -> eyre::Result<bool>,
    S: System,
{
    fn name(&self) -> String {
        format!("FilterSystem({})", self.system.name())
    }

    fn register_components(&self) {
        self.system.register_components();
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if (self.predicate)(data)? {
            self.system.run(data)
        } else {
            Ok(())
        }
    }
}

impl<S: System> Debug for DelayedSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DelayedSystem(activation_time: {})", self.activation_time)
    }
}

impl<S: System> Display for DelayedSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DelayedSystem(activation_time: {})", self.activation_time)
    }
}

impl<S: System> System for DelayedSystem<S> {
    fn name(&self) -> String {
        format!("DelayedSystem({})", self.system.name())
    }

    fn register_components(&self) {
        self.system.register_components();
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if get_simulation_time(data).0 >= self.activation_time {
            self.system.run(data)
        } else {
            Ok(())
        }
    }
}

impl Debug for SystemCollection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SystemCollection({:?})", self.0)
    }
}

impl System for SystemCollection {
    fn name(&self) -> String {
        let mut collection_name = String::new();
        collection_name.push_str("System collection: ");
        let mut system_names_iter = self.0.iter().map(|system| system.name()).peekable();

        if let Some(name) = system_names_iter.next() {
            collection_name.push_str(&name);
        }

        for name in system_names_iter {
            collection_name.push_str(", ");
            collection_name.push_str(&name);
        }

        collection_name
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        for s in self.0.iter_mut() {
            s.run(data)?;
        }
        Ok(())
    }
}

impl From<Vec<Box<dyn System>>> for SystemCollection {
    fn from(vec: Vec<Box<dyn System>>) -> Self {
        Self(vec)
    }
}

impl<S> FromIterator<S> for SystemCollection
where
    S: Into<Box<dyn System>>,
{
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        SystemCollection(iter.into_iter().map(|s| s.into()).collect())
    }
}
