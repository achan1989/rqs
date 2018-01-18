mod game;
mod parms;
mod quakedef;
mod time;
mod cmd;


fn main() {
    // WinQuake used the WinMain entry point, which doesn't include the program
    // name in it's equivalent of argv.
    let argv_owned: Vec<String> = std::env::args().skip(1).collect();
    // let argv_ref: Vec<&str> =
    //     argv_owned.iter().map(std::ops::Deref::deref).collect();

    let cwd = std::env::current_dir();
    let cwd = String::from(cwd.unwrap().to_str().unwrap());

    let mut game = game::new(argv_owned, cwd);
    game.run();
}
