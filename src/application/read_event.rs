use crate::util::Result;

type CallBack = Box<dyn Fn(std::fs::File, String, usize, usize) + Send + Sync>;

pub struct ReadFile {
    callback: CallBack,
    id: usize,
}

pub struct Chunk {
    pub file: std::fs::File,
    pub id: usize,
    pub file_name: String,
    pub data: Vec<u8>,
    pub bytes_read: usize,
    pub file_size: usize,
}

impl ReadFile {
    pub fn new(callback: CallBack) -> Self {
        Self { callback, id: 0 }
    }

    pub fn send(&mut self, file_name: String, path: std::path::PathBuf) -> Result<usize> {
    }
}

pub fn read_file(
    mut file: std::fs::File,
    file_name: String,
    file_size: usize,
    id: usize,
) -> Result<Chunk>
{
    use std::io::Read;

    const BLOCK: usize = 65536;
    let mut data = [0; BLOCK];

    match file.read(&mut data) {
        Ok(bytes_read) => {
            let chunk = Chunk {
                file,
                id,
                file_name,
                data: data[..bytes_read].to_vec(),
                bytes_read,
                file_size,
            };
            Ok(chunk)
        }
        Err(e) => Err(e.into()),
    }
}
