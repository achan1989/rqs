use std;

use parms;
use time;


pub struct Game {
    parms: parms::Parms,
    time: time::Time
}

pub fn new(argv: Vec<String>, cwd: String) -> Game {
    let parms = parms::Parms::new(argv, cwd);
    let time = time::Time::new(&parms);
    Game {
        parms,
        time
    }
}

impl Game {
    pub fn run(&mut self) {
        println!("cmdline is {:?}", self.parms.cmdline);
        println!("com_argv is {:?}", self.parms.argv);

        if self.parms.is_dedicated {
            unimplemented!("dedicated server alloc console");
            // AllocConsole(), InitConProc().
        }

        self.printf(format!("host init\n"));
        unimplemented!("host init");
    }

    pub fn error(&self, s: &'static str) {
        // WARN: this is supposed to do a lot more.
        panic!(s);
    }

    pub fn printf(&self, s: String) {
        if self.parms.is_dedicated {
            unimplemented!("dedicated server printf");
        }
    }

    fn make_self_ref(&self) -> Box<Fn() -> &'static Self> {
        let p_game: *const Game = self;
        Box::new(move || {
            unsafe {
                &*p_game
            }
        })
    }
}
