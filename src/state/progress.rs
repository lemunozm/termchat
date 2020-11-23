#[derive(PartialEq)]
pub enum ProgressState {
    Started(usize),               // id
    Working(usize, u64, u64),       // id, file_size, current_bytes
    Stopped(u64),               // file_size
}

pub struct ProgressBar {
    state: ProgressState,
}

