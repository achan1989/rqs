use std;


pub struct Cmd {
    cmd_text: String,
    wait: bool
}

#[derive(Debug)]
pub enum CmdSource {
    // C: src_client
    // C: src_command
    Client,
    CmdBuf
}

impl Cmd {
    pub fn new() -> Self {
        Cmd {
            cmd_text: String::with_capacity(8000),
            wait: false
        }
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
    }
}


mod test {
    use super:: Cmd;

    #[test]
    fn buf_basic() {
        let mut cmd = Cmd::new();

        cmd.buf_add_text("First cmd;");
        cmd.buf_add_text("Second cmd;");
        cmd.buf_insert_text("Third cmd;");

        assert_eq!(cmd.cmd_text, "Third cmd;First cmd;Second cmd;");
    }

    #[test]
    fn execute_basic() {
        let mut cmd = Cmd::new();

        cmd.buf_add_text("First cmd;");
        cmd.buf_add_text("Second cmd\n");
        cmd.buf_add_text("third");

        cmd.buf_execute();
        assert_eq!(cmd.cmd_text, "");
    }

    #[test]
    fn execute_wait() {
        let mut cmd = Cmd::new();

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
        let mut cmd = Cmd::new();

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
