use crate::util::Result;
use std::sync::Arc;

type CallBack = Box<dyn Fn(Result<Chunk>) + Send + Sync>;

pub struct ReadFile {
    f: Arc<CallBack>,
    // tx: mpsc::Sender<CallBack>,
    // rx: mpsc::Receiver<CallBack>,
}

pub struct Chunk {
    pub file_name: String,
    pub data: Vec<u8>,
    pub bytes_read: usize,
    pub file_size: usize,
}

impl ReadFile {
    pub fn new(f: CallBack) -> Self {
        Self { f: Arc::new(f) }
    }

    pub fn send(&mut self, file_name: String, path: std::path::PathBuf) {
        let f = self.f.clone();

        std::thread::spawn(move || {
            use std::io::Read;

            use std::convert::TryInto;
            let file_size = std::fs::metadata(&path).unwrap().len().try_into().unwrap();

            let mut file = std::fs::File::open(path).unwrap();

            const BLOCK: usize = 65536;
            let mut data = [0; BLOCK];

            loop {
                match file.read(&mut data) {
                    Ok(bytes_read) => {
                        let chunk = Chunk {
                            file_name: file_name.clone(),
                            data: data[..bytes_read].to_vec(),
                            bytes_read,
                            file_size,
                        };
                        f(Ok(chunk));
                        if bytes_read == 0 {
                            break;
                        }
                    }
                    Err(e) => {
                        f(Err(e.into()));
                        break;
                    }
                }
            }
        });
    }
}
