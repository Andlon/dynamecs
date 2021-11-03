//! Generic adapters for systems.
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display};

use crate::{System, Universe};

/// A system that runs only once and executes its contained closure
pub struct RunOnceSystem<F>
where
    F: FnOnce(&Universe) -> Result<(), Box<dyn Error>>,
{
    pub closure: Option<F>,
    has_run: bool,
}

/// System that uses a closure to determine if a system should be run
pub struct FilterSystem<P, S>
where
    P: FnMut(&Universe) -> Result<bool, Box<dyn Error>>,
    S: System,
{
    pub predicate: P,
    pub system: S,
}

/// Wrapper to store a vector of systems that are run in sequence
pub struct SystemCollection(pub Vec<Box<dyn System>>);

impl<F> Debug for RunOnceSystem<F>
where
    F: FnOnce(&Universe) -> Result<(), Box<dyn Error>>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunOnceSystem(has_run: {})", self.has_run)
    }
}

impl<F> RunOnceSystem<F>
where
    F: FnOnce(&Universe) -> Result<(), Box<dyn Error>>,
{
    pub fn new(closure: F) -> Self {
        RunOnceSystem {
            closure: Some(closure),
            has_run: false,
        }
    }
}

impl<F> Display for RunOnceSystem<F>
where
    F: FnOnce(&Universe) -> Result<(), Box<dyn Error>>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunOnceSystem(has_run: {})", self.has_run)
    }
}

impl<F> System for RunOnceSystem<F>
where
    F: FnOnce(&Universe) -> Result<(), Box<dyn Error>>,
{
    fn name(&self) -> String {
        todo!("Should probably take name as an (optional) constructor input")
    }

    fn run(&mut self, data: &mut Universe) -> Result<(), Box<dyn Error>> {
        if !self.has_run {
            let ret = (self
                .closure
                .take()
                .ok_or_else(|| Box::<dyn Error>::from("Closure gone"))?)(data)?;
            self.has_run = true;
            Ok(ret)
        } else {
            Ok(())
        }
    }
}

impl<P, S> Debug for FilterSystem<P, S>
where
    P: FnMut(&Universe) -> Result<bool, Box<dyn Error>>,
    S: System,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Filter({:?})", self.system)
    }
}

impl<P, S> Display for FilterSystem<P, S>
where
    P: FnMut(&Universe) -> Result<bool, Box<dyn Error>>,
    S: System,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Filter({})", self.system.name())
    }
}

impl<P, S> System for FilterSystem<P, S>
where
    P: FnMut(&Universe) -> Result<bool, Box<dyn Error>>,
    S: System,
{
    fn name(&self) -> String {
        todo!("Should probably take name as optional parameter to constructor")
    }

    fn run(&mut self, data: &mut Universe) -> Result<(), Box<dyn Error>> {
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

    fn run(&mut self, data: &mut Universe) -> Result<(), Box<dyn Error>> {
        for s in self.0.iter_mut() {
            s.run(data)?;
        }
        Ok(())
    }
}
