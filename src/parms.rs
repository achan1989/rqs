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

// Modified by Adrian Chan, March 2018
// Pulled the command line parameter handling out of common.c

use std;

use defs;

const SAFE_ARGVS: [&'static str; 7] =
    ["-stdvid", "-nolan", "-nosound", "-nocdaudio", "-nojoy", "-nomouse",
     "-dibonly"];


/// Represents the command line parameters that Quake was started with.
///
/// Provides an interface for querying whether parameters exist, and their
/// associated value(s), if any.
/// Also provides shortcuts for some common "parameter exists" queries.
///
/// The convention for command line parameters is as follows:
///
/// * Commands starts with a `+`
/// * TODO: what comes after a command?
/// * Parameters start with a `-`
/// * Parameters can be followed by zero or more values
/// * Parameter values cannot start with `-` or `+`

// Don't intend to support cachedir.
pub struct Parms {
    argv: Vec<String>,
    cwd: String,
    cmdline: String,
    is_dedicated: bool,
    is_standard_quake: bool,
    is_rogue: bool,
    is_hipnotic: bool
}

impl Parms {
    pub fn new(mut argv: Vec<String>, cwd: String) -> Self {
        argv.truncate(defs::MAX_NUM_ARGVS);
        // Reconstitute the command line for the cmdline externally visible
        // cvar.
        let mut cmdline: String = argv[..].join(" ");
        cmdline.truncate(defs::CMDLINE_LENGTH);

        let safe = argv.contains(&"-safe".to_string());
        if safe {
            // Force all the safe-mode switches.
            for &arg in &SAFE_ARGVS {
                argv.push(arg.to_string());
            }
        }

        let mut parms = Parms {
            argv,
            cwd,
            cmdline: String::new(),
            is_dedicated: false,
            is_standard_quake: true,
            is_rogue: false,
            is_hipnotic: false
        };
        parms.detect_features();
        parms
    }

    /// Is Quake running as a dedicated server?
    pub fn is_dedicated(&self) -> bool {
        self.is_dedicated
    }

    /// Is this standard Quake i.e. not using any mission packs?
    pub fn is_standard_quake(&self) -> bool {
        self.is_standard_quake
    }

    /// Are we using the Dissolution of Eternity mission pack?
    pub fn is_rogue(&self) -> bool {
        self.is_rogue
    }

    /// Are we using the Scourge of Armagon mission pack?
    pub fn is_hipnotic(&self) -> bool {
        self.is_hipnotic
    }

    fn detect_features(&mut self) {
        if self.has("-rogue") {
            self.is_rogue = true;
            self.is_standard_quake = false;
        }
        if self.has("-hipnotic") {
            self.is_hipnotic = true;
            self.is_standard_quake = false;
        }
        if self.has("-dedicated") {
            self.is_dedicated = true;
        }
    }

    /// Is this command line parameter present?
    pub fn has(&self, parm: &str) -> bool {
        match self.index(parm) {
            Some(_i) => true,
            None => false
        }
    }

    /// The index of the command line parameter, if present.
    pub fn index(&self, parm: &str) -> Option<usize> {
        self.argv.iter().position(|ref p| p.as_str() == parm)
    }

    /// Parse the value following the command line parameter, if present.
    ///
    /// # Example
    ///
    /// ```
    /// use rqs::Parms;
    ///
    /// let p = Parms::new(
    ///     vec!("-foo".into(), "3".into()),
    ///     "cwd".into());
    /// let value: i32 = p.parse_value("-foo").unwrap();
    /// assert_eq!(value, 3);
    /// ```
    pub fn parse_value<F: std::str::FromStr>(&self, parm: &str) -> Option<F> {
        match self.index(parm) {
            None => None,
            Some(parm_idx) => {
                let value_idx = parm_idx + 1;
                match self.argv.get(value_idx) {
                    None => None,
                    Some(val) => {
                        val.parse().ok()
                    }
                }
            }
        }
    }

    /// Get the single str value following the command line parameter, if
    /// present.
    ///
    /// # Example
    ///
    /// ```
    /// use rqs::Parms;
    ///
    /// let p = Parms::new(
    ///     vec!("-foo".into(), "some_text".into()),
    ///     "cwd".into());
    /// let value = p.value("-foo").unwrap();
    /// assert_eq!(value, "some_text");
    /// ```
    pub fn value(&self, parm: &str) -> Option<&str> {
        match self.index(parm) {
            None => None,
            Some(parm_idx) => {
                let value_idx = parm_idx + 1;
                match self.argv.get(value_idx) {
                    None => None,
                    Some(val) => {
                        Some(&val)
                    }
                }
            }
        }
    }

    /// Get the String values following the command line parameter, if present.
    ///
    /// # Example
    ///
    /// ```
    /// use rqs::Parms;
    ///
    /// let p = Parms::new(
    ///     vec!("-foo".into(), "a".into(), "b".into()),
    ///     "cwd".into());
    /// let values = p.values("-foo").unwrap();
    /// assert_eq!(values, &["a", "b"]);
    /// ```
    pub fn values(&self, parm: &str) -> Option<&[String]> {
        match self.index(parm) {
            None => None,
            Some(parm_idx) => {
                let start = parm_idx + 1;
                let mut end = start;
                for val in self.argv.iter().skip(start) {
                    match val.chars().nth(0) {
                        None => break,
                        Some('+') => break,
                        Some('-') => break,
                        Some(_) => end += 1
                    }
                }

                if start == end {
                    None
                } else {
                    Some(&self.argv[start..end])
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_detection() {
        // None.
        {
            let p = Parms::new(
                vec!(),
                "cwd".into());
            assert_eq!(p.is_dedicated(), false);
            assert_eq!(p.is_standard_quake(), true);
            assert_eq!(p.is_rogue(), false);
            assert_eq!(p.is_hipnotic(), false);
        }

        // All.
        {
            let p = Parms::new(
                vec!("-dedicated".into(),
                     "-rogue".into(),
                     "-hipnotic".into()),
                "cwd".into());
            assert_eq!(p.is_dedicated(), true);
            assert_eq!(p.is_standard_quake(), false);
            assert_eq!(p.is_rogue(), true);
            assert_eq!(p.is_hipnotic(), true);
        }
    }

    #[test]
    fn parm_find() {
        {
            let p = Parms::new(
                vec!("zero".into(),
                     "-one".into()),
                "cwd".into());
            assert_eq!(p.has("zero"), true);
            assert_eq!(p.index("zero"), Some(0));
            assert_eq!(p.has("-one"), true);
            assert_eq!(p.index("-one"), Some(1));
            assert_eq!(p.has("nope"), false);
            assert_eq!(p.index("nope"), None);
        }
    }

    #[test]
    fn value() {
        {
            let p = Parms::new(
                vec!("-foo".into(),
                     "bar".into()),
                "cwd".into());
            let val = p.value("-foo");
            assert_eq!(val, Some("bar"));
        }
    }

    #[test]
    fn values() {
        // No param.
        {
            let p = Parms::new(
                vec!("-foo".into(),
                     "a".into()),
                "cwd".into());
            let values = p.values("nope");
            assert_eq!(values, None);
        }

        // No value.
        {
            let p = Parms::new(
                vec!("-foo".into()),
                "cwd".into());
            let values = p.values("-foo");
            assert_eq!(values, None);
        }

        // No value, two params.
        {
            let p = Parms::new(
                vec!("-foo".into(),
                     "-bar".into()),
                "cwd".into());
            let values = p.values("-foo");
            assert_eq!(values, None);
        }

        // Single value.
        {
            let p = Parms::new(
                vec!("-foo".into(),
                     "a".into()),
                "cwd".into());
            let values = p.values("-foo").unwrap();
            assert_eq!(values, &["a"]);
        }

        // Two values.
        {
            let p = Parms::new(
                vec!("-foo".into(),
                     "a".into(),
                     "b".into()),
                "cwd".into());
            let values = p.values("-foo").unwrap();
            assert_eq!(values, &["a", "b"]);
        }

        // Stop at next param.
        {
            let p = Parms::new(
                vec!("-foo".into(),
                     "a".into(),
                     "b".into(),
                     "-bar".into()),
                "cwd".into());
            let values = p.values("-foo").unwrap();
            assert_eq!(values, &["a", "b"]);
        }

        // Stop at command.
        {
            let p = Parms::new(
                vec!("-foo".into(),
                     "a".into(),
                     "b".into(),
                     "+bar".into()),
                "cwd".into());
            let values = p.values("-foo").unwrap();
            assert_eq!(values, &["a", "b"]);
        }
    }
}
