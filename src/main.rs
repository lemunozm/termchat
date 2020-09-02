mod application;
mod terminal_events;

use application::{Application};

fn main() {
    let addr = "239.255.0.1:5089".parse().unwrap();
    let name = whoami::username();
    if let Ok(mut app) = Application::new(addr, &name) {
        app.run()
    }
}
