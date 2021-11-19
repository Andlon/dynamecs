//! Generic adapters for systems.
use eyre::eyre;
use std::fmt;
use std::fmt::{Debug, Display};

use crate::{System, Universe};

/// Adapts a `FnOnce` closure as a [`System`].
///
/// The closure is only run once and dropped afterwards.
pub struct RunOnceClosureSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    closure: Option<F>,
    has_run: bool,
}

/// Wraps a [`System`] and runs it only once.
///
/// The system is guaranteed to be run only once and is dropped afterwards.
pub struct RunOnceSystem<S: System> {
    system: Option<S>,
    has_run: bool,
}

/// Filter system that uses a closure to determine if the wrapped system should be run.
pub struct FilterSystem<P, S>
where
    P: FnMut(&Universe) -> eyre::Result<bool>,
    S: System,
{
    system: S,
    predicate: P,
}

/// Wrapper to store a vector of systems that are run in sequence.
pub struct SystemCollection(pub Vec<Box<dyn System>>);

impl<F> RunOnceClosureSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    pub fn new(closure: F) -> Self {
        RunOnceClosureSystem {
            closure: Some(closure),
            has_run: false,
        }
    }
}

impl<F> Debug for RunOnceClosureSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunOnceClosureSystem(has_run: {})", self.has_run)
    }
}

impl<F> Display for RunOnceClosureSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunOnceClosureSystem(has_run: {})", self.has_run)
    }
}

impl<F> System for RunOnceClosureSystem<F>
where
    F: FnOnce(&mut Universe) -> eyre::Result<()>,
{
    fn name(&self) -> String {
        todo!("Should probably take name as an (optional) constructor input")
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

impl<S: System> RunOnceSystem<S> {
    pub fn new(system: S) -> Self {
        RunOnceSystem {
            system: Some(system),
            has_run: false,
        }
    }
}

impl<S: System> Debug for RunOnceSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunOnceSystem(has_run: {})", self.has_run)
    }
}

impl<S: System> Display for RunOnceSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunOnceSystem(has_run: {})", self.has_run)
    }
}

impl<S: System> System for RunOnceSystem<S> {
    fn name(&self) -> String {
        todo!("Should probably take name as an (optional) constructor input")
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if !self.has_run {
            let ret = self
                .system
                .take()
                .ok_or_else(|| eyre!("system gone"))?
                .run(data)?;
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
        todo!("Should probably take name as optional parameter to constructor")
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if (self.predicate)(data)? {
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
