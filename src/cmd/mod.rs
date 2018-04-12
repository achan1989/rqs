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

use std::collections::HashMap;
use std::ops::Range;

use failure::Error;

use util;


/// Handler functions for commands must be of this type.
pub type CommandHandler = fn(Command, CmdSource) -> Result<(), Error>;

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
    cmd_text: String,
    /// The registered commands, and their handler functions.
    commands: HashMap<String, CommandHandler>,
    /// The aliases that have been created, and the text that they represent.
    aliases: HashMap<String, String>,
    /// Causes execution of the remainder of the command buffer to be delayed
    /// until next frame.
    wait: bool,
}

impl CommandCenter {
    /// Create a new `CommandCenter`.
    pub fn new() -> Self {
        Self {
            cmd_text: String::with_capacity(8192),
            commands: HashMap::with_capacity(200),
            aliases: HashMap::with_capacity(20),
            wait: false,
        }
    }

    /// Add text to the end of the command buffer.
    pub fn add_text(&mut self, text: &str) {
        // Don't care about limiting memory usage right now.
        self.cmd_text.push_str(text);
    }

    /// Insert text at the beginning of the command buffer, before any
    /// unexecuted commands.
    ///
    /// Useful when a command wants to issue other commands and have them
    /// processed immediately.
    pub fn insert_text(&mut self, text: &str) {
        self.cmd_text.insert_str(0, text);
    }

    /// Remove lines of text from the command buffer and execute them.
    ///
    /// Stops once the buffer is empty, or aborts with an `Err` if a command
    /// handler raises an error.
    pub fn execute_buffer(&mut self) -> Result<(), Error> {
        while !self.cmd_text.is_empty() {
            // Find a `\n` or `;` line break.
            let mut line = String::with_capacity(200);
            let mut quotes = 0;
            for c in self.cmd_text.chars() {
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
            if len == self.cmd_text.len() {
                // Hit the end of the text, no terminating char.
                self.cmd_text.clear();
            } else {
                // Remove the line plus the terminating char.
                self.cmd_text.drain(0..len+1);
            }

            self.execute_text(line, CmdSource::CmdBuffer)?;

            if self.wait {
                // Process the rest of the buffer in the next frame.
                self.wait = false;
                break;
            }
        }

        Ok(())
    }

    /// Try to execute one complete line of a command.
    pub fn execute_text(&mut self, text: String, source: CmdSource)
        -> Result<(), Error>
    {
        let command = Command::tokenise(text);
        if let None = command {
            return Ok(());
        }
        let command = command.unwrap();

        if let Some(handler) = self.commands.get(command.name()) {
            return handler(command, source);
        }

        if let Some(text) = self.aliases.get(command.name()) {
            self.cmd_text.insert_str(0, text);
            return Ok(());
        }

        unimplemented!("attempt to execute command as cvar");
    }

    /// Register a command name, to be handled by the given handler.
    ///
    /// Panics if the name has been registered already, or if the name clashes
    /// with the name of a cvar.
    pub fn register_command(&mut self, cmd_name: &str, handler: CommandHandler)
    {
        // Fail if the command is a variable name.
        unimplemented!("check if a cvar exists with this command's name");

        if let Some(_) = self.commands.get(cmd_name) {
            panic!("Can't add the command, it already exists: {}", cmd_name);
        }

        self.commands.insert(cmd_name.into(), handler);
    }

    /// Is there a registered command with this name?
    pub fn is_command_registered(&self, cmd_name: &str) -> bool {
        match self.commands.get(cmd_name) {
            Some(_) => true,
            None => false,
        }
    }

    /// Try to find a command name that matches the given partial string.
    pub fn complete_command(&self, partial: &str) -> Option<&String> {
        self.commands.keys().find(|name| name.starts_with(partial))
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
