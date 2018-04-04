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
// Pulled the filesystem-y parts out of common.c

//! Things relating to the Quake filesystem.

pub mod pack;
pub mod reader;
pub use self::reader::FsReader;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use failure::Error;

use defs;
use parms::Parms;


/// Represents the Quake filesystem.
///
/// The `FileSys` has an underlying set of search paths, which specify the
/// directories and/or .pak files that may contain the game's files.
///
/// `FileSys` methods allow a "file" to be opened and read in various ways.
/// Under the hood, these methods use the search path to find the file, which
/// may be stored on disk or inside a .pak file.
pub struct FileSys {
    /// This is considered to be the main game directory.
    game_dir: PathBuf,
    /// Use Quake 1 progs with Quake 2 maps?
    use_proghack: bool,
    /// The `SearchPath`s that will be used when looking for a file.
    /// The items are stored in reverse priority order.
    search_paths: Vec<SearchPath>,
}

impl FileSys {
    /// Create and initialise the file system.
    pub fn new(parms: &Parms) -> Result<Self, Error> {
        let game_dir = PathBuf::new();
        let use_proghack = parms.has("-proghack");
        let search_paths = Vec::with_capacity(5);
        let mut fs = Self {
            game_dir,
            use_proghack,
            search_paths,
        };
        fs.init(parms)?;
        Ok(fs)
    }

    fn init(&mut self, parms: &Parms) -> Result<(), Error> {
        // `-basedir <path>` overrides the base directory (usually the
        // directory that contains the Quake executable?).
        let basedir = match parms.value("-basedir") {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(parms.cwd()),
        };

        let subdir = |subdir: &str| -> PathBuf {
            let mut d = basedir.clone();
            d.push(subdir);
            d
        };

        // cachedir will not be supported.

        // Automatically generate the search path.  Even if this is overridden
        // later with the -path command, we still need to do this to determine
        // the game_dir.

        // Always use the default GAMENAME.
        self.add_game_dir(subdir(defs::GAMENAME))?;

        // Any mission packs that are being used.
        if parms.is_rogue() {
            self.add_game_dir(subdir("rogue"))?;
        }
        if parms.is_hipnotic() {
            self.add_game_dir(subdir("hipnotic"))?;
        }

        // `-game <gamedir>` adds basedir/<gamedir> as an override game.
        if let Some(gamedir) = parms.value("-game") {
            self.add_game_dir(subdir(gamedir))?;
        }

        // `-path <dir or packfile> [<dir or packfile>] ...` lets the user fully
        // specify the exact search path, overriding the one we just generated.
        if let Some(path_overrides) = parms.values("-path") {
            for path in path_overrides.iter() {
                let search = match path.ends_with(".pak") {
                    true => {
                        let pak = pack::Pack::load(path.into())?;
                        if pak.is_none() {
                            // If the user explicitly asked us to use a pak
                            // then it must exist.
                            bail!("Couldn't load packfile {}", path);
                        }
                        SearchPath::Pack(pak.unwrap())
                    },
                    false => SearchPath::Directory(path.into()),
                };
                self.search_paths.push(search);
            }
        }

        Ok(())
    }

    /// Add the given directory to the start of the search path, make it the
    /// main game directory, then add any .pak files that it contains to the
    /// start of the search path.
    fn add_game_dir(&mut self, path: PathBuf) -> Result<(), Error> {
        // Add the directory to the search path, lower priority.
        self.search_paths.push(SearchPath::Directory(path.clone()));

        // Add any .pak files contained in the directory, higher priority.
        // Look for pak0.pak, pak1.pak, ..., and stop when the file is not
        // found.
        for i in 0.. {
            let mut path = path.clone();
            let filename = format!("pak{}.pak", i);
            path.push(filename);
            match pack::Pack::load(path) {
                Err(e) => return Err(e),
                Ok(None) => break,  // File not found.
                Ok(Some(pak)) => self.search_paths.push(SearchPath::Pack(pak)),
            }
        }

        // This is now the main game directory.
        self.game_dir = path;
        Ok(())
    }

    /// Find a file by name, returning a `FsReader` of the file if found.
    fn find_file(&mut self, name: &str) -> Result<Option<FsReader>, Error> {
        // Conditionally skip the first entry, because "gross hack to use quake
        // 1 progs with quake 2 maps".
        let skip_n = match self.use_proghack {
            true => 1,
            false => 0,
        };
        // Search paths are stored in reverse priority order.
        for search in self.search_paths.iter_mut().rev().skip(skip_n) {
            match search {
                &mut SearchPath::Pack(ref mut pak) => {
                    // Try to find a matching file contained in the pack.
                    match pak.file(name) {
                        Err(e) => return Err(e),
                        Ok(None) => (),
                        Ok(Some(reader)) => return Ok(Some(reader)),
                    }
                },
                &mut SearchPath::Directory(ref dir) => {
                    // Try to find a matching file on the disk.
                    let mut path = dir.clone();
                    path.push(name);
                    match path.is_file() {
                        false => (),
                        true => {
                            let file = File::open(path)?;
                            return Ok(Some(FsReader::for_file(file)?))
                        },
                    }
                },
            }
        }

        Ok(None)
    }

    /// Load a file. This method allocates the buffer that the data is returned
    /// in.
    pub fn load_file(&mut self, name: &str) -> Result<Option<Vec<u8>>, Error> {
        match self.find_file(name)? {
            None => Ok(None),
            Some(mut fsr) => {
                let mut buf = Vec::with_capacity(fsr.len());
                fsr.read_to_end(&mut buf)?;
                Ok(Some(buf))
            }
        }
    }

    /// TODO: load a file into the given cache.
    pub fn load_file_into_cache(&self, _name: &str, _cache: ()) {
        unimplemented!();
    }

    /// TODO: load a file into the given buffer.
    pub fn load_file_into_buf(&self, _name: &str, _buf: &mut [u8]) {
        unimplemented!();
    }
}

/// A place to look for resources.
#[derive(Debug)]
enum SearchPath {
    /// Files in a directory.
    Directory(PathBuf),
    /// Files in a .pak file.
    Pack(pack::Pack),
}


#[cfg(test)]
mod tests {
    use super::*;
    use test_common as common;

    /// Create a new FileSys using the base directory of a full version of
    /// standard Quake 1.
    /// Expect a particular search path.
    #[test]
    fn fs_search_path() {
        let parms = Parms::new(
            vec!["-basedir".into(), common::base_dir().to_string_lossy().to_string()],
            "cwd".into());
        let fs = FileSys::new(&parms).unwrap();
        assert_eq!(fs.search_paths.len(), 3);

        match fs.search_paths[0] {
            SearchPath::Directory(ref path) => {
                assert!(path.to_str().unwrap().ends_with("id1"));
            },
            ref x => panic!("expected a directory search path for 'id1', got {:?}", x),
        }

        match fs.search_paths[1] {
            SearchPath::Pack(ref p) => {
                assert!(p.path().to_str().unwrap().ends_with("pak0.pak"));
            },
            ref x => panic!("expected a pack search path for 'pak0.pak', got {:?}", x),
        }

        match fs.search_paths[2] {
            SearchPath::Pack(ref p) => {
                assert!(p.path().to_str().unwrap().ends_with("pak1.pak"));
            },
            ref x => panic!("expected a pack search path for 'pak1.pak', got {:?}", x),
        }
    }

    #[test]
    /// Load a resource from the file system.
    fn fs_load_file() {
        let parms = Parms::new(
            vec!["-basedir".into(), common::base_dir().to_string_lossy().to_string()],
            "cwd".into());
        let mut fs = FileSys::new(&parms).unwrap();
        let wav = fs.load_file("sound/items/r_item1.wav").unwrap().unwrap();

        // Simple check: does it look like the WAV file we expect?
        assert_eq!(wav.len(), 6822);
        assert_eq!(&wav[0..4], b"RIFF");
    }
}
