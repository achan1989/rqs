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
// Parts of cmd.c

//! Things related to commands.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Range;

use failure::Error;

use state::{State, GetCommands, GetCvars, GetParms};
use util;


/// Handler functions for commands must be of this type.
pub type CommandHandler = fn(Command, CmdSource, &State) -> Result<(), Error>;

/// A placeholder command handler function. Allows the program to compile, but
/// causes a panic when called.
pub fn dummy_command_handler_fn(_command: Command, _source: CmdSource)
    -> Result<(), Error>
{
    unimplemented!("this handler function has not been implemented");
}

/// Where did a command come from?
#[derive(Debug, PartialEq)]
pub enum CmdSource {
    /// From a client.
    Client,
    /// From the command buffer.
    CmdBuffer,
}

/// The `CommandCenter` is a combination of the original Quake implementation's:
///
/// * Command buffer, which holds incoming commands (in the form of text) from
///   script files, remote clients, stdin, keybindings, etc.
/// * "Command registry", which tracks the valid commands and their associated
///   handler functions.
///
/// To execute a command in the command buffer, a line of text (`\n` or `;`
/// terminated) is removed from the buffer. It is tokenised into command and
/// argument parts, and an appropriate handler function is looked for. If
/// found, the handler function is called with the arguments.
///
/// Alternatively, another part of the engine can make the `CommandCenter`
/// execute a command directly. This works exactly like executing a command in
/// the command buffer, only without the buffer :)
pub struct CommandCenter {
    /// The command buffer.
    cmd_text: RefCell<String>,
    /// The registered commands, and their handler functions.
    commands: RefCell<HashMap<String, CommandHandler>>,
    /// The aliases that have been created, and the text that they represent.
    aliases: HashMap<String, String>,
    /// Causes execution of the remainder of the command buffer to be delayed
    /// until next frame.
    wait: Cell<bool>,
}

impl CommandCenter {
    /// Create a new `CommandCenter`.
    pub fn new() -> Self {
        Self {
            cmd_text: RefCell::new(String::with_capacity(8192)),
            commands: RefCell::new(HashMap::with_capacity(200)),
            aliases: HashMap::with_capacity(20),
            wait: Cell::new(false),
        }
    }

    /// Perform one-time initialization.
    pub fn init(&self, state: &State) {
        self.register_command("stuffcmds", Self::stuffcmds_handler, state);
    }

    /// Add text to the end of the command buffer.
    pub fn add_text(&self, text: &str) {
        // Don't care about limiting memory usage right now.
        self.cmd_text.borrow_mut().push_str(text);
    }

    /// Insert text at the beginning of the command buffer, before any
    /// unexecuted commands.
    ///
    /// Useful when a command wants to issue other commands and have them
    /// processed immediately.
    pub fn insert_text(&self, text: &str) {
        self.cmd_text.borrow_mut().insert_str(0, text);
    }

    /// Remove lines of text from the command buffer and execute them.
    ///
    /// Stops once the buffer is empty, or aborts with an `Err` if a command
    /// handler raises an error.
    pub fn execute_buffer(&self, state: &State) -> Result<(), Error> {
        loop {
            // Remove a line of text from the buffer. Must release ownership
            // of the buffer once this is done...
            let line = {
                let mut cmd_text = self.cmd_text.borrow_mut();
                if cmd_text.is_empty() {
                    break;
                }

                // Find a `\n` or `;` line break.
                let mut line = String::with_capacity(200);
                let mut quotes = 0;
                for c in cmd_text.chars() {
                    match c {
                        '"' => quotes += 1,
                        // Don't break if inside a quoted string.
                        ';' if (quotes % 2 == 0) => break,
                        '\n' => break,
                        _ => (),
                    }

                    // Take everything except the terminating char.
                    line.push(c);
                }

                // Delete the line from the buffer and move remaining text down.
                let len = line.len();
                if len == cmd_text.len() {
                    // Hit the end of the text, no terminating char.
                    cmd_text.clear();
                } else {
                    // Remove the line plus the terminating char.
                    cmd_text.drain(0..len+1);
                }

                line
            };

            // ...because we might want to add things to the buffer when a
            // command is executed.
            self.execute_text(line, CmdSource::CmdBuffer, state)?;

            if self.wait.get() {
                // Process the rest of the buffer in the next frame.
                self.wait.set(false);
                break;
            }
        }

        Ok(())
    }

    /// Try to execute one complete line of a command.
    pub fn execute_text(&self, text: String, source: CmdSource, state: &State)
        -> Result<(), Error>
    {
        let command = Command::tokenise(text);
        if let None = command {
            return Ok(());
        }
        let command = command.unwrap();

        if let Some(handler) = self.commands.borrow().get(command.name()) {
            return handler(command, source, state);
        }

        if let Some(text) = self.aliases.get(command.name()) {
            self.insert_text(text);
            return Ok(());
        }

        if !state.cvars().handle_console(command) {
            unimplemented!("print 'unknown command' to console");
        }

        Ok(())
    }

    /// Register a command name, to be handled by the given handler.
    ///
    /// Panics if the name has been registered already, or if the name clashes
    /// with the name of a cvar.
    pub fn register_command(&self, cmd_name: &str, handler: CommandHandler, state: &State)
    {
        if let Some(_) = state.cvars().find(cmd_name) {
            panic!(
                "Can't register command '{}', it clashes with a cvar of the \
                same name", cmd_name);
        }

        let mut commands = self.commands.borrow_mut();
        if let Some(_) = commands.get(cmd_name) {
            panic!("Can't add the command, it already exists: {}", cmd_name);
        }

        commands.insert(cmd_name.into(), handler);
    }

    /// Is there a registered command with this name?
    pub fn is_command_registered(&self, cmd_name: &str) -> bool {
        match self.commands.borrow().get(cmd_name) {
            Some(_) => true,
            None => false,
        }
    }

    /// Try to find a command name that matches the given partial string.
    pub fn complete_command(&self, partial: &str) -> Option<String> {
        let commands = self.commands.borrow();
        match commands.keys().find(|name| name.starts_with(partial)) {
            None => None,
            Some(name) => Some(name.clone())
        }
    }

    // Handler functions for commands related to the CommandCenter.

    /// Add command line parameters as script statements.
    ///
    /// Commands lead with a `+`, and continue until a `-` or another `+`
    ///
    /// ```text
    /// quake +prog jctest.qp +cmd amlev1
    /// quake -nosound +cmd amlev1
    /// ```
    fn stuffcmds_handler(cmd: Command, _source: CmdSource, state: &State)
        -> Result<(), Error>
    {
        if let Some(_) = cmd.args() {
            unimplemented!("stuffcmds help in console, return Ok");
        }

        let args = state.parms().args();
        if args.len() == 0 {
            return Ok(());
        }
        // Build the combined string to parse from.
        let text = args.join(" ");
        let mut build = String::with_capacity(text.len());

        let mut sp = util::StrProcessor::new(&text);
        loop {
            sp.skip_to_command();
            match sp.consume_command() {
                None => break,
                Some(range) => {
                    build.push_str(&text[range]);
                    build.push_str("\n");
                },
            }
        }

        if !build.is_empty() {
            state.commands().insert_text(&build);
        }
        Ok(())
    }
}


/// Represents some received command input, possibly with arguments, tokenised
/// from a string.
pub struct Command {
    /// The original text, unaltered.
    full_text: String,
    /// The substring range that holds all of the arguments, as received (if
    /// there were any).
    args_range: Option<Range<usize>>,
    /// Each individual token -- command then arguments.
    tokens: Vec<String>,
}

impl Command {
    /// Get the name of the command that was given.
    pub fn name(&self) -> &str {
        &self.tokens[0]
    }

    /// Get the command's arguments, if any were given.
    pub fn args(&self) -> Option<&[String]> {
        match self.tokens.len() {
            0 => unreachable!(),
            1 => None,
            _ => Some(&self.tokens[1..]),
        }
    }

    /// Try to form a `Command` from the given text.
    ///
    /// If this returns `Some`, the resulting `Command` is guaranteed to have
    /// at least one token (`tokens.len() > 0`).
    fn tokenise(text: String) -> Option<Self> {
        const MAX_ARGS: usize = 80;
        let mut argc = 0;
        let mut args_range = None;
        let mut tokens = Vec::with_capacity(20);

        {
            let mut sp = util::StrProcessor::new(&text);
            loop {
                sp.skip_whitespace_until_newline();
                match sp.remainder() {
                    None => break,
                    Some(remainder) => {
                        // A newline seperates commands in the buffer.
                        if (&text[remainder.clone()]).starts_with("\n") {
                            break;
                        }
                        if argc == 1 {
                            args_range = Some(remainder);
                        }
                    }
                }
    
                let token_range = sp.consume_token();
                if let None = token_range {
                    break;
                }
    
                if argc < MAX_ARGS {
                    tokens.push(String::from(&text[token_range.unwrap()]));
                    argc += 1;
                }
            }
        }

        match argc {
            0 => None,
            _ => Some(Self {
                full_text: text,
                args_range,
                tokens,
            })
        }
    }
}


#[cfg(test)]
mod tests_command {
    use super::*;

    #[test]
    fn basic() {
        let cmd = Command::tokenise("kill humans --all".to_string()).unwrap();
        assert_eq!(cmd.name(), "kill");
        assert_eq!(cmd.args().unwrap(), &["humans", "--all"]);
    }
}

#[cfg(test)]
mod tests_command_center {
    use super::*;
    use cvar::CvarManager;
    use parms::Parms;

    thread_local!( static FOO_DATA: RefCell<Vec<String>> = RefCell::new(Vec::new()));

    fn foo_handler(cmd: Command, source: CmdSource, state: &State)
        -> Result<(), Error>
    {
        FOO_DATA.with(|data| {
            data.borrow_mut().push(cmd.args().unwrap()[0].clone())
        });
        Ok(())
    }

    fn reset_foo_handler() {
        FOO_DATA.with(|data| {
            data.borrow_mut().clear()
        });
    }

    fn setup_state() -> State {
        reset_foo_handler();
        let state = State {
            cvars: CvarManager::new(),
            commands: CommandCenter::new(),
            parms: Parms::new(
                vec!("-nosound".into(), "+cmd".into(), "amlev1".into()),
                "cwd".into()),
        };
        state.commands.init(&state);
        state
    }

    #[test]
    fn basic() {
        let mut state = setup_state();
        state.commands.register_command("foo", foo_handler, &state);
        state.commands.add_text("foo 123\n");
        state.commands.execute_buffer(&state).unwrap();
        FOO_DATA.with(|data| {
            assert_eq!(&data.borrow()[..], &["123"]);
        });
    }

    #[test]
    fn stuffcmds() {
        let mut state = setup_state();
        state.commands.execute_text("stuffcmds\n".into(), CmdSource::CmdBuffer, &state);
        assert_eq!(*state.commands.cmd_text.borrow(), "cmd amlev1\n");
    }
}
