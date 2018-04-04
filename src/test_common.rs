use std::path::PathBuf;

use defs;


/// Get the base Quake directory (contains the quake executable).
pub fn base_dir() -> PathBuf {
    use std::env;
    use std::env::VarError;

    let var = env::var("QUAKE_DIR");
    match var {
        Err(VarError::NotPresent) =>
            panic!("Must set QUAKE_DIR to run these tests"),
        Err(VarError::NotUnicode(v)) =>
            panic!("QUAKE_DIR is not valid unicode: '{:?}'", v),
        Ok(s) =>
            PathBuf::from(s),
    }
}

/// Get the game directory (basedir/GAMENAME).
pub fn game_dir() -> PathBuf {
    let mut dir = base_dir();
    dir.push(defs::GAMENAME);
    dir
}

/// Get the path of the game's pak0.pak file.
pub fn pak0_path() -> PathBuf {
    let mut p = game_dir();
    p.push("PAK0.PAK");
    p
}
