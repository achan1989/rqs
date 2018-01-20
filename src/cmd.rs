use game;


pub struct Cmd {
    game: Box<Fn() -> &'static game::Game>,
    cmd_text: String,
    cmd_functions: Vec<CmdFunction>,
    wait: bool
}

pub struct CmdFunction {
    name: &'static str,
    function: XCommand
}

#[derive(Debug)]
pub enum CmdSource {
    // C: src_client
    // C: src_command
    Client,
    CmdBuf
}

type XCommand = Box< Fn() -> ()>;

impl Cmd {
    pub fn new(game: Box<Fn() -> &'static game::Game>) -> Self {
        let cmd = Cmd {
            game,
            cmd_text: String::with_capacity(8000),
            cmd_functions: Vec::with_capacity(10),
            wait: false
        };
        // cmd.add_basic_commands();
        cmd
    }

    fn _add_basic_commands(&mut self) {
        unimplemented!("add basic commands");
        // self.add_command("stuffcmds", stuffcmds_fn);
        // self.add_command("exec", exec_fn);
        // self.add_command("echo", echo_fn);
        // self.add_command("alias", alias_fn);
        // self.add_command("cmd", forward_to_server_fn);
        // self.add_command("wait", wait_fn);
    }

    // Add text to the end of the buffer.
    pub fn buf_add_text<S>(&mut self, text: S) where S: Into<String> {
        self.cmd_text.push_str(&text.into());
    }

    // Insert text at the front of the buffer.
    pub fn buf_insert_text<S>(&mut self, text: S) where S: Into<String> {
        self.cmd_text.insert_str(0, &text.into());
    }

    pub fn buf_execute(&mut self) {
        while !self.cmd_text.is_empty() {
            let mut line = String::with_capacity(200);
            let mut quotes = 0;
            // Find a '\n' or ';' line break, unless it's a ';' inside a quote.
            for c in self.cmd_text.chars() {
                if c == '"' {
                    quotes += 1;
                } else if (quotes % 2 == 0) && c == ';' {
                    break;
                } else if c == '\n' {
                    break;
                }

                // Don't include the terminating char.
                line.push(c);
            }

            let len = line.len();
            if len == self.cmd_text.len() {
                // Hit the end of the text, no terminating char.
                self.cmd_text.clear();
            } else {
                // Remove the fragment plus the terminating char.
                self.cmd_text.drain(0..len+1);
            }

            self.execute_string(line, CmdSource::CmdBuf);

            if self.wait {
                // Process the rest of the buffer in the next frame.
                self.wait = false;
                break;
            }
        }
    }

    pub fn execute_string(&self, s: String, src: CmdSource) {
        println!("Execute from {:?}: {:?}", src, s);
        //unimplemented!();
    }

    pub fn add_command(&mut self, name: &'static str, function: XCommand) {
        let game = (self.game)();

        if is_host_initialized() {
            game.error("Cmd::add_command after host initialized");
        }

        if is_variable_name(&name) {
            unimplemented!("console print already defined as a var");
            // return;
        }

        if self.command_exists(&name) {
            unimplemented!("console print command already defined");
            // return;
        }

        self.cmd_functions.push( CmdFunction { name, function } );
    }

    pub fn command_exists(&self, name: &'static str) -> bool {
        for cmd in &self.cmd_functions {
            if cmd.name == name {
                return true;
            }
        }
        false
    }

    pub fn complete_command(&self, partial: &String) -> Option<String> {
        for cmd in &self.cmd_functions {
            if cmd.name.starts_with(partial) {
                return Some(String::from(cmd.name));
            }
        }
        None
    }
}


fn is_host_initialized() -> bool {
    unimplemented!();
}

fn is_variable_name(_name: &str) -> bool {
    unimplemented!();
}


mod test {
    use super:: Cmd;
    use game;

    #[cfg(test)]
    fn dummy_game_ref() -> Box<Fn() -> &'static game::Game> {
        Box::new(|| panic!("dummy, not supposed to be used") )
    }

    #[test]
    fn buf_basic() {
        let mut cmd = Cmd::new(dummy_game_ref());

        cmd.buf_add_text("First cmd;");
        cmd.buf_add_text("Second cmd;");
        cmd.buf_insert_text("Third cmd;");

        assert_eq!(cmd.cmd_text, "Third cmd;First cmd;Second cmd;");
    }

    #[test]
    fn execute_basic() {
        let mut cmd = Cmd::new(dummy_game_ref());

        cmd.buf_add_text("First cmd;");
        cmd.buf_add_text("Second cmd\n");
        cmd.buf_add_text("third");

        cmd.buf_execute();
        assert_eq!(cmd.cmd_text, "");
    }

    #[test]
    fn execute_wait() {
        let mut cmd = Cmd::new(dummy_game_ref());

        cmd.buf_add_text("First cmd;");
        cmd.buf_add_text("Second cmd\n");
        cmd.buf_add_text("remainder");

        cmd.wait = true;
        cmd.buf_execute();
        assert_eq!(cmd.cmd_text, "Second cmd\nremainder");

        cmd.wait = true;
        cmd.buf_execute();
        assert_eq!(cmd.cmd_text, "remainder");
    }

    #[test]
    fn execute_escape() {
        let mut cmd = Cmd::new(dummy_game_ref());

        cmd.buf_add_text(r#"First "cmd";"#);
        cmd.buf_add_text(r#"Second "cmd;" thing;"#);
        cmd.buf_add_text("Third \"cmd; consumed\n");
        cmd.buf_add_text("remainder");

        cmd.wait = true;
        cmd.buf_execute();
        assert_eq!(
            cmd.cmd_text,
            "Second \"cmd;\" thing;Third \"cmd; consumed\nremainder");

        cmd.wait = true;
        cmd.buf_execute();
        assert_eq!(cmd.cmd_text, "Third \"cmd; consumed\nremainder");

        cmd.wait = true;
        cmd.buf_execute();
        assert_eq!(cmd.cmd_text, "remainder");
    }
}
