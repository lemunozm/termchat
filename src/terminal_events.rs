use crate::util::Result;
use crossterm::event::Event;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const EVENT_SAMPLING_TIMEOUT: u64 = 50; //ms

pub struct TerminalEventCollector {
    collector_thread_running: Arc<AtomicBool>,
    collector_thread_handle: Option<JoinHandle<()>>,
}

impl TerminalEventCollector {
    pub fn new<C>(event_callback: C) -> Result<TerminalEventCollector>
    where C: Fn(Result<Event>) + Send + 'static {
        let collector_thread_running = Arc::new(AtomicBool::new(true));
        let collector_thread_handle = {
            let running = collector_thread_running.clone();
            let timeout = Duration::from_millis(EVENT_SAMPLING_TIMEOUT);
            thread::Builder::new().name("termchat: terminal event collector".into()).spawn(
                move || {
                    let try_read = || -> Result<()> {
                        if crossterm::event::poll(timeout)? {
                            let event = crossterm::event::read()?;
                            event_callback(Ok(event));
                        }
                        Ok(())
                    };
                    while running.load(Ordering::Relaxed) {
                        if let Err(e) = try_read() {
                            event_callback(Err(e));
                        }
                    }
                },
            )
        }?;

        Ok(TerminalEventCollector {
            collector_thread_running,
            collector_thread_handle: Some(collector_thread_handle),
        })
    }
}

impl Drop for TerminalEventCollector {
    fn drop(&mut self) {
        self.collector_thread_running.store(false, Ordering::Relaxed);
        // the first unwrap is safe, beacuse we now the handle is some and this is the only time we take it
        self.collector_thread_handle
            .take()
            .unwrap()
            .join()
            .expect("Error while joining collector thread handle");
    }
}
