use std::fmt;
use std::fmt::{Debug, Display};

use dynamecs::{System, Universe};

use crate::components::get_simulation_time;

pub trait SystemExt: System {
    /// Wraps the system such that it only runs if the [`SimulationTime`](`crate::components::SimulationTime`) reaches the specified time.
    fn with_delay(self, time: f64) -> DelayedSystem<Self>
    where
        Self: Sized,
    {
        DelayedSystem::new(self, time)
    }
}

impl<S: System> SystemExt for S {}

pub struct DelayedSystem<S: System> {
    system: S,
    time: f64,
}

impl<S: System> DelayedSystem<S> {
    pub fn new(system: S, time: f64) -> Self {
        DelayedSystem { system, time }
    }
}

impl<S: System> Debug for DelayedSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DelayedSystem(time: {})", self.time)
    }
}

impl<S: System> Display for DelayedSystem<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DelayedSystem(time: {})", self.time)
    }
}

impl<S: System> System for DelayedSystem<S> {
    fn name(&self) -> String {
        todo!("Should probably take name as an (optional) constructor input")
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if self.time < get_simulation_time(data).0 {
            self.system.run(data)
        } else {
            Ok(())
        }
    }
}
