#![allow(missing_docs)]

//! Because design is hard...

use cmd::CommandCenter;
use cvar::CvarManager;
use parms::Parms;


/// Holds all of the game state.
///
/// This trait stuff looks unnecessary (and actually *is* unnecessary for the
/// main code), but it makes unit testing much easier.
pub struct State {
    pub cvars: CvarManager,
    pub commands: CommandCenter,
    pub parms: Parms,
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

impl GetParms for State {
    fn parms(&self) -> &Parms {
        &self.parms
    }
}


pub trait GetCvars {
    fn cvars(&self) -> &CvarManager;
}

pub trait GetCommands {
    fn commands(&self) -> &CommandCenter;
}

pub trait GetParms {
    fn parms(&self) -> &Parms;
}
