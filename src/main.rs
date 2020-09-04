mod application;
mod state;
mod ui;
mod terminal_events;
mod util;

use application::{Application};

fn main() {
    let addr = "239.255.0.1:3010".parse().unwrap();

    let args: Vec<String> = std::env::args().collect();
    let name = match args.get(1) {
        Some(name) => name.into(),
        None => whoami::username(),
    };
    println!("{}", name);
    if let Ok(mut app) = Application::new(addr, &name) {
        app.run()
    }
}
