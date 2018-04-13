#![allow(missing_docs)]

//! Because design is hard...

use cmd::CommandCenter;
use cvar::CvarManager;


/// Holds all of the game state.
///
/// This trait stuff looks unnecessary (and actually *is* unnecessary for the
/// main code), but it makes unit testing much easier.
pub struct State {
    pub cvars: CvarManager,
    pub commands: CommandCenter,
}

impl GetCvars for State {
    fn cvars(&self) -> &CvarManager {
        &self.cvars
    }
}

impl GetCommands for State {
    fn commands(&self) -> &CommandCenter {
        &self.commands
    }
}


pub trait GetCvars {
    fn cvars(&self) -> &CvarManager;
}

pub trait GetCommands {
    fn commands(&self) -> &CommandCenter;
}