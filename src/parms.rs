use std;

const MAX_NUM_ARGVS: usize = 50;
const CMDLINE_LENGTH: usize = 256;
const SAFE_ARGVS: [&'static str; 7] =
    ["-stdvid", "-nolan", "-nosound", "-nocdaudio", "-nojoy", "-nomouse",
     "-dibonly"];


pub struct Parms {
    // C: com_argv
    pub argv: Vec<String>,
    pub cwd: String,
    pub cachedir: Option<String>,
    pub cmdline: String,
    pub is_dedicated: bool,
    pub standard_quake: bool,
    pub rogue: bool,
    pub hipnotic: bool
}

impl Parms {
    pub fn new(argv: Vec<String>, cwd: String) -> Self {
        let mut parms = Parms {
            argv,
            cwd,
            cachedir: None,  // Don't intend to support this.
            cmdline: String::new(),
            is_dedicated: false,
            standard_quake: true,
            rogue: false,
            hipnotic: false
        };
        parms.init();
        parms
    }

    // C: COM_InitArgv
    fn init(&mut self) {
        let mut safe = false;

        // Reconstitute the command line for the cmdline externally visible
        // cvar.
        self.argv.truncate(MAX_NUM_ARGVS);
        for arg in &self.argv {
            self.cmdline.push_str(arg.as_str());
            self.cmdline.push_str(" ");
            if arg == "-safe" {
                safe = true;
            }
        }
        if self.cmdline.len() > 0 {
            self.cmdline.pop();
        }
        self.cmdline.truncate(CMDLINE_LENGTH);

        if safe {
            // Force all the safe-mode switches.
            for &arg in &SAFE_ARGVS {
                self.argv.push(String::from(arg));
            }
        }

        if self.has_parm("-rogue") {
            self.rogue = true;
            self.standard_quake = false;
        }
        if self.has_parm("-hipnotic") {
            self.hipnotic = true;
            self.standard_quake = false;
        }
        if self.has_parm("-dedicated") {
            self.is_dedicated = true;
        }
    }

    // C: COM_CheckParm
    pub fn check_parm(&self, parm: &str) -> Option<usize> {
        self.argv.iter().position(|ref p| p.as_str() == parm)
    }

    pub fn has_parm(&self, parm: &str) -> bool {
        match self.check_parm(parm) {
            Some(_i) => true,
            None => false
        }
    }

    pub fn get_parm_value<F: std::str::FromStr>(&self, parm: &str) -> Option<F> {
        match self.check_parm(parm) {
            None => None,
            Some(p_idx) => {
                let v_idx = p_idx + 1;
                match self.argv.get(v_idx) {
                    None => None,
                    Some(val) => {
                        val.parse().ok()
                    }
                }
            }
        }
    }

    pub fn get_raw_parm_values(&self, parm: &str) -> Option<Vec<String>> {
        match self.check_parm(parm) {
            None => None,
            Some(p_idx) => {
                let mut raw: Vec<String> = Vec::new();
                for val in self.argv.iter().skip(p_idx + 1) {
                    match val.chars().nth(0) {
                        None => break,
                        Some('+') => break,
                        Some('-') => break,
                        Some(_) => raw.push(val.clone())
                    }
                }
                Some(raw)
            }
        }
    }
}
