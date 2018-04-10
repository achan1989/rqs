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

use std::ops::Range;

use util;


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
