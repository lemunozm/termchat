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
        use std::convert::TryInto;

        let try_read = || -> Result<(std::fs::File, usize)> {
            let file_size = std::fs::metadata(&path)?.len().try_into()?;
            let file = std::fs::File::open(path)?;
            Ok((file, file_size))
        };

        let (file, file_size) = match try_read() {
            Ok((file, file_size)) => (file, file_size),
            Err(e) => {
                return Err(e);
            }
        };

        let send_id = self.id;
        (self.callback)(file, file_name, file_size, send_id);
        self.id += 1;

        Ok(send_id)
    }
}

use super::Event;
pub fn read_file(
    sender: message_io::events::EventSender<Event>,
    mut file: std::fs::File,
    file_name: String,
    file_size: usize,
    id: usize,
) {
    use std::io::Read;

    const BLOCK: usize = 65536;
    let mut data = [0; BLOCK];

    let res = match file.read(&mut data) {
        Ok(bytes_read) => {
            let chunk = Chunk {
                file,
                id,
                file_name,
                data: data[..bytes_read].to_vec(),
                bytes_read,
                file_size,
            };
            Event::ReadFile(Ok(chunk))
        }
        Err(e) => Event::ReadFile(Err(e.into())),
    };
    sender.send(res);
}
