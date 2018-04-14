// Copyright (C) 1996-1997 Id Software, Inc.
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; either version 2
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
//
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, write to the Free Software
// Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA  02111-1307, USA.

// Modified by Adrian Chan, April 2018

//! Things related to cvars (console variables).

use std::cell::{Ref, RefMut, RefCell};
use std::io;

use failure::Error;

use state::{GetCommands};
use util;


/// Holds the registered cvars, and allows operations on them.
pub struct CvarManager {
    /// The cvars that have been registered.
    /// Stored in reverse priority order.
    vars: RefCell<Vec<Cvar>>,
}

impl CvarManager {
    /// Create a new `CvarManager`.
    pub fn new() -> Self {
        Self {
            vars: RefCell::new(Vec::with_capacity(20)),
        }
    }

    /// Get the cvar with the given name, if it exists.
    pub fn find(&self, var_name: &str) -> Option<Ref<Cvar>> {
        let vars = self.vars.borrow();
        match vars.iter().position(|cv| cv.name == var_name) {
            None => None,
            Some(i) => Some(Ref::map(vars, |vars| &vars[i])),
        }
    }

    /// Get the cvar with the given name, if it exists.
    ///
    /// The returned `Cvar` may be modified.
    fn find_mut(&self, var_name: &str) -> Option<RefMut<Cvar>> {
        let vars = self.vars.borrow_mut();
        match vars.iter().position(|cv| &cv.name == var_name) {
            None => None,
            Some(i) => Some(RefMut::map(vars, |vars| &mut vars[i]))
        }
    }

    /// Try to get the float value of the cvar with the given name.
    ///
    /// Returns `None` if that cvar is not defined, or if the value is not
    /// numeric.
    pub fn variable_value(&self, var_name: &str) -> Option<f32> {
        match self.find(var_name) {
            None => None,
            Some(cv) => util::atof(&cv.string_val),
        }
    }

    /// Try to get the string value of the cvar with the given name.
    ///
    /// Returns `None` if that cvar is not defined.
    pub fn variable_str(&self, var_name: &str) -> Option<Ref<String>> {
        match self.find(var_name) {
            None => None,
            Some(rcv) => Some(Ref::map(rcv, |cv| &cv.string_val)),
        }
    }

    /// Attempts to match a partial variable name for command line completion.
    ///
    /// Returns `None` if nothing fits.
    pub fn complete_variable(&self, partial_name: &str) -> Option<Ref<String>>
    {
        if partial_name == "" {
            return None;
        }

        // Make sure to iterate in the proper order for this one.
        let vars = self.vars.borrow();
        match vars.iter().rposition(|cv| cv.name.starts_with(partial_name)) {
            None => None,
            Some(i) => Some(Ref::map(vars, |vars| &vars[i].name)),
        }
    }

    /// Set the string value of the cvar with the given name.
    ///
    /// Equivalent to `<name> "<value>"` typed at the console, but only
    /// intended for use internally by the engine.
    ///
    /// # Panics
    ///
    /// Panics if the given cvar is not defined.
    pub fn set_string(&self, var_name: &str, value: &str) {
        // Apparently this is a logic error, and should never happen.
        let mut cv = self.find_mut(var_name).expect(
            &format!("cvar {} not found", var_name));

        let changed = cv.string_val != value;
        cv.string_val = value.to_string();
        cv.value = match util::atof(value) {
            Some(f) => f,
            None => 0.0,
        };

        if cv.server && changed {
            // TODO: if running as a server
            unimplemented!("server broadcasts cvar change to clients");
        }
    }

    /// Set the float value of the cvar with the given name.
    ///
    /// Under the hood, formats the value as a string then calls `set_string`.
    pub fn set_float(&self, var_name: &str, value: f32) {
        let str_val = format!("{}", value);
        self.set_string(var_name, &str_val);
    }

    /// Add a `Cvar` to the registry, via the given `CvarBuilder`.
    ///
    /// # Panics
    /// Panics if another `Cvar` with the same name has already been registered,
    /// or if the `Cvar`'s name collides with the name of a command.
    ///
    /// This differs from the original Quake implementation, which would just
    /// print an error message and continue.
    pub fn register<S: GetCommands>(&self, builder: CvarBuilder, state: &S) {
        if let Some(_) = self.find(&builder.name) {
            panic!("Can't register cvar '{}', it's already defined",
                   builder.name);
        }

        if state.commands().is_command_registered(&builder.name) {
            panic!(
                "Can't register cvar '{}', it clashes with a command of \
                the same name",
                builder.name);
        }

        let value = match util::atof(&builder.string_val) {
            Some(f) => f,
            None => 0.0,
        };
        self.vars.borrow_mut().push(
            Cvar {
                name: builder.name,
                string_val: builder.string_val,
                value,
                archive: builder.archive,
                server: builder.server,
            });
    }

    /// Serialise the `Cvar`s that need archiving.
    ///
    /// Writes lines containing `<name> "<value>"` for all registered `Cvar`s
    /// where the `archive` field is `true`.
    pub fn write_cvars<W: io::Write>(&self, writer: &mut W) -> Result<(), Error>
    {
        for cv in self.vars.borrow().iter().rev().filter(|cv| cv.archive) {
            write!(writer, "{} \"{}\"\n", cv.name, cv.string_val)?;
        }
        Ok(())
    }
}

/// A named variable that can hold a float or string value.
///
/// `Cvar`s are named variables that can hold a float or string value. They can
/// be changed or displayed at the console or prog code, as well as accessed
/// directly in Rust code.
///
/// It is sufficient to initialize a `Cvar` with just the `name` and
/// `string_val` fields, or you can set the `archive` field to `true` for
/// variables that you want saved to the configuration file when the game is
/// quit (see the example below).
///
/// `Cvar`s must be registered before use, otherwise their value cannot be
/// accessed.
/// This differs from the original Quake implementation, where they have a `0`
/// value instead of the float interpretation of the string.  Generally, all
/// `Cvar`s should be registered in the apropriate init function before any
/// console commands are executed:
///
/// ```
/// // TODO: example of declaration and registration.
/// ```
///
/// # Accessing `Cvar`s
///
/// Rust code must reference a cvar via the `CvarManager`:
///
/// ```
/// // TODO: example with CvarManager.set_string("r_draworder", "some value")
/// // and CvarManager.set_float("r_draworder", 1.5)
/// ```
///
/// It could optionally ask for the value to be looked up for a string name:
///
/// ```
/// // TODO: example with CvarManager.variable_value("r_draworder")
/// ```
///
/// Interpreted prog code can access `Cvar`s with the `cvar(name)` or
/// `cvar_set(name, value)` internal functions:
///
/// ```c
/// teamplay = cvar("teamplay");
/// cvar_set ("registered", "1");
/// ```
///
/// The user can access cvars from the console in two ways:
///
/// ```text
/// r_draworder         prints the current value
/// r_draworder 0       sets the current value to 0
/// ```
///
/// `Cvar`s are restricted from having the same names as commands to keep this
/// interface from being ambiguous.
///
/// # Notes
///
/// I think a cvar nominally has a string type value or a float type value, and
/// it's up to the definer/user to know which it should be?
/// This suggests that `string_val` and `value` could be combined into a single
/// enum type, but I'm not sure that we can safely assume that cvar types will
/// be used that way under all circumstances.
#[derive(Debug)]
pub struct Cvar {
    name: String,
    string_val: String,
    value: f32,
    archive: bool,
    server: bool,
}

impl Cvar {
    /// Define a new `Cvar` with the given name and initial value.
    ///
    /// A shortcut for the [`CvarBuilder::new`] method.
    ///
    /// The definition does not become "active" until it is registered with a
    /// `CvarManager`.
    ///
    /// [`CvarBuilder::new`]: struct.CvarBuilder.html
    pub fn define(name: &str, string_val: &str) -> CvarBuilder {
        CvarBuilder::new(name, string_val)
    }
}

/// Represents the definition of a new `Cvar`.
///
/// The definition does not become "active" until it is registered with a
/// `CvarManager`.
///
/// # Example
///
/// ```
/// use rqs::cvar::{Cvar, CvarManager};
/// use rqs::state::State;
///
/// fn module_init(cvm: &mut CvarManager, state: &State) {
///     cvm.register(
///         Cvar::define("cl_thingy", "123")
///               .archive(),
///         state
///     );
/// }
/// ```
pub struct CvarBuilder {
    name: String,
    string_val: String,
    archive: bool,
    server: bool,
}

impl CvarBuilder {
    /// Define a new `Cvar` with the given name and initial value.
    pub fn new(name: &str, string_val: &str) -> Self {
        Self {
            name: name.to_string(),
            string_val: string_val.to_string(),
            archive: false,
            server: false,
        }
    }

    /// Mark this cvar as "needs archiving".
    ///
    /// A cvar marked in this way will be saved to disk when Quake is shut down.
    pub fn archive(mut self) -> Self {
        self.archive = true;
        self
    }

    /// Mark this cvar as a server variable.
    ///
    /// If a server cvar is changed, the server will broadcast the change to
    /// all connected clients.
    pub fn server(mut self) -> Self {
        self.server = true;
        self
    }
}


#[cfg(test)]
mod tests_cvar_stuff {
    use super::*;
    use cmd::CommandCenter;

    struct MockState {
        cvars: CvarManager,
        commands: CommandCenter,
    }

    impl GetCommands for MockState {
        fn commands(&self) -> &CommandCenter {
            &self.commands
        }
    }

    fn setup_state() -> MockState {
        let state = MockState {
            cvars: CvarManager::new(),
            commands: CommandCenter::new(),
        };

        state.cvars.register(
            Cvar::define("cv_foo", "hello"),
            &state);
        state.cvars.register(
            Cvar::define("cv_bar", "123.0")
                  .archive(),
            &state);
        state
    }

    #[test]
    fn basic() {
        let state = setup_state();
        let cvars = &state.cvars;

        assert_eq!(*cvars.variable_str("cv_foo").unwrap(), "hello");
        assert!(cvars.variable_value("cv_foo").is_none());
        assert_eq!(*cvars.variable_str("cv_bar").unwrap(), "123.0");
        assert_eq!(cvars.variable_value("cv_bar").unwrap(), 123.0);

        if let None = cvars.find("cv_foo") {
            panic!("got None result for find('cv_foo')");
        }
        if let Some(x) = cvars.find("potato") {
            panic!("expected None result for find('potato'), got {:?}", x);
        }

        cvars.set_string("cv_foo", "world");
        assert_eq!(*cvars.variable_str("cv_foo").unwrap(), "world");
        cvars.set_float("cv_bar", -14.83);
        assert_eq!(cvars.variable_value("cv_bar").unwrap(), -14.83);
    }
}
