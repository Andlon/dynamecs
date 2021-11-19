use std::fmt;
use std::fmt::{Debug, Display};

use dynamecs::{System, Universe};

use crate::components::get_simulation_time;

pub trait SystemExt: System {
    /// Wraps the system such that it only runs if the [`SimulationTime`](`crate::components::SimulationTime`) reaches the specified time.
    ///
    /// The system runs only if `simulation_time >= activation_time`
    fn delay_until(self, activation_time: f64) -> DelayedSystem<Self>
    where
        Self: Sized,
    {
        DelayedSystem::new(self, activation_time)
    }
}

impl<S: System> SystemExt for S {}

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
        todo!("Should probably take name as an (optional) constructor input")
    }

    fn run(&mut self, data: &mut Universe) -> eyre::Result<()> {
        if get_simulation_time(data).0 >= self.activation_time {
            self.system.run(data)
        } else {
            Ok(())
        }
    }
}
