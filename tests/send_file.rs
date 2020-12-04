#![cfg(feature = "ui-test")]

use termchat::application::{Application, Event, Config};
use message_io::events::EventSender;

#[test]
fn send_file() {
    let termchat_dir = std::env::temp_dir().join("termchat");
    let test_path = termchat_dir.join("test");
    let _ = std::fs::remove_dir_all(&termchat_dir);
    std::fs::create_dir_all(&termchat_dir).unwrap();

    let data = vec![rand::random(); 10usize.pow(6)];
    std::fs::write(&test_path, &data).unwrap();

    // spawn users
    let config1: Config = Config {
        discovery_addr: "238.255.0.1:5877".parse().unwrap(),
        tcp_server_port: "0".parse().unwrap(),
        user_name: 1.to_string(),
    };
    let config2: Config = Config {
        discovery_addr: "238.255.0.1:5877".parse().unwrap(),
        tcp_server_port: "0".parse().unwrap(),
        user_name: 2.to_string(),
    };
    let (mut s1, t1) = test_user(config1);
    // wait a bit or termchat will creates two-communication channels at the same time
    std::thread::sleep(std::time::Duration::from_millis(100));
    let (s2, t2) = test_user(config2);

    // wait for users to connect
    std::thread::sleep(std::time::Duration::from_millis(100));
    // send file
    input(&mut s1, &format!("?send {}", test_path.display()));
    // wait for the file to finish sending
    std::thread::sleep(std::time::Duration::from_secs(2));

    // finish
    s1.send(Event::Close(None));
    s2.send(Event::Close(None));
    t1.join().unwrap();
    t2.join().unwrap();

    // assert eq
    let send_data =
        std::fs::read(std::env::temp_dir().join("termchat").join("1").join("test")).unwrap();
    assert_eq!(data.len(), send_data.len());
    assert_eq!(data, send_data);
}

fn test_user(config: Config) -> (EventSender<Event>, std::thread::JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let t = std::thread::spawn(move || {
        let mut app = Application::new(&config).unwrap();
        let sender = app.sender();
        tx.send(sender).unwrap();
        app.run(std::io::sink()).unwrap();
    });
    (rx.recv().unwrap(), t)
}

fn input(sender: &mut EventSender<Event>, s: &str) {
    for c in s.chars() {
        sender.send(Event::Terminal(crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char(c),
            modifiers: crossterm::event::KeyModifiers::NONE,
        })));
    }
    sender.send(Event::Terminal(crossterm::event::Event::Key(crossterm::event::KeyEvent {
        code: crossterm::event::KeyCode::Enter,
        modifiers: crossterm::event::KeyModifiers::NONE,
    })));
}
