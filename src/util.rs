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
