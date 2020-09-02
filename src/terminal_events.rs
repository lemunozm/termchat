use crossterm::event::{Event};

use std::thread::{self, JoinHandle};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration};

const EVENT_SAMPLING_TIMEOUT: u64 = 50; //ms

pub struct TerminalEventCollector {
    collector_thread_running: Arc<AtomicBool>,
    collector_thread_handle: Option<JoinHandle<()>>,
}

impl TerminalEventCollector {
    pub fn new<C>(event_callback: C) -> TerminalEventCollector
    where C: Fn(Event) + Send + 'static {
        let collector_thread_running = Arc::new(AtomicBool::new(true));
        let collector_thread_handle = {
            let running = collector_thread_running.clone();
            let timeout = Duration::from_millis(EVENT_SAMPLING_TIMEOUT);
            thread::Builder::new().name("termchat: terminal event collector".into()).spawn(move || {
                while running.load(Ordering::Relaxed) {
                    if crossterm::event::poll(timeout).unwrap() {
                        let event = crossterm::event::read().unwrap();
                        event_callback(event);
                    }
                }
            })
        }.unwrap();

        TerminalEventCollector {
            collector_thread_running,
            collector_thread_handle: Some(collector_thread_handle),
        }
    }
}

impl Drop for TerminalEventCollector {
    fn drop(&mut self) {
        self.collector_thread_running.store(false, Ordering::Relaxed);
        self.collector_thread_handle.take().unwrap().join().unwrap();
    }
}
