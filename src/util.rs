pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

lazy_static! {
    static ref TERMCHAT_TEMP: std::path::PathBuf = std::env::temp_dir().join("termchat");
    pub static ref PANIC_LOG_PATH: std::path::PathBuf = TERMCHAT_TEMP.join("panic_log");
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let _ = std::fs::create_dir_all(&*TERMCHAT_TEMP);
        let _ = std::fs::write(&*PANIC_LOG_PATH, panic_info.to_string());
    }));
}

pub trait SplitEach {
    fn split_each(&self, n: usize) -> Vec<&Self>;
}

impl SplitEach for str {
    fn split_each(&self, n: usize) -> Vec<&str> {
        let mut splitted =
            Vec::with_capacity(self.len() / n + if self.len() % n > 0 { 1 } else { 0 });
        let mut last = self;
        while !last.is_empty() {
            let (chunk, rest) = last.split_at(std::cmp::min(n, last.len()));
            splitted.push(chunk);
            last = rest;
        }
        splitted
    }
}
