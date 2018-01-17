use std;


// The host system specifies the base of the directory tree and the
// command line parms passed to the program.
pub struct QuakeParms<'a> {
    pub basedir: std::path::PathBuf,
    pub cachedir: Option<std::path::PathBuf>,
    pub argv: &'a Vec<&'a str>
}
