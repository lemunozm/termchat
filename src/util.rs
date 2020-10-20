pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub trait SplitEach {
    fn split_each(&self, n: usize) -> Vec<String>;
}

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

impl SplitEach for str {
    fn split_each(&self, n: usize) -> Vec<String> {
        let mut ll = Vec::with_capacity(self.width() / n);
        let mut l = String::new();

        let mut i = 0;

        for c in self.chars() {
            if i != 0 && i >= n {
                ll.push(l.drain(..).collect());
                i = 0;
            }
            l.push(c);
            i += c.width().unwrap_or(0);
        }
        if !l.is_empty() {
            ll.push(l.drain(..).collect());
        }
        ll
    }
}
