pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub trait SplitEach {
    fn split_each(&self, n: usize) -> Vec<String>;
}

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

impl SplitEach for str {
    fn split_each(&self, width: usize) -> Vec<String> {
        let mut splitted = Vec::with_capacity(self.width() / width);
        let mut row = String::new();

        let mut index = 0;

        for current_char in self.chars() {
            if (index != 0 && index == width) || index + current_char.width().unwrap_or(0) > width {
                splitted.push(row.drain(..).collect());
                index = 0;
            }

            row.push(current_char);
            index += current_char.width().unwrap_or(0);
        }
        // leftover
        if !row.is_empty() {
            splitted.push(row.drain(..).collect());
        }
        splitted
    }
}
