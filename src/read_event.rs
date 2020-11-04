use crate::util::Result;
use std::sync::Arc;

type CallBack = Box<dyn Fn(Result<Chunk>) + Send + Sync>;

pub struct ReadFile {
    callback: Arc<CallBack>,
}

pub struct Chunk {
    pub id: usize,
    pub file_name: String,
    pub data: Vec<u8>,
    pub bytes_read: usize,
    pub file_size: usize,
}

impl ReadFile {
    pub fn new(callback: CallBack) -> Self {
        Self {
            callback: Arc::new(callback),
        }
    }

    pub fn send(
        &mut self,
        id: usize,
        file_name: String,
        path: std::path::PathBuf,
    ) -> std::thread::JoinHandle<()> {
        let callback = self.callback.clone();

        std::thread::spawn(move || {
            use std::convert::TryInto;
            use std::io::Read;

            let try_read = || -> Result<(std::fs::File, usize)> {
                let file_size = std::fs::metadata(&path)?.len().try_into()?;
                let file = std::fs::File::open(path)?;
                Ok((file, file_size))
            };

            let (mut file, file_size) = match try_read() {
                Ok((file, file_size)) => (file, file_size),
                Err(e) => {
                    callback(Err(e));
                    return;
                }
            };

            const BLOCK: usize = 65536;
            let mut data = [0; BLOCK];

            loop {
                match file.read(&mut data) {
                    Ok(bytes_read) => {
                        let chunk = Chunk {
                            id,
                            file_name: file_name.clone(),
                            data: data[..bytes_read].to_vec(),
                            bytes_read,
                            file_size,
                        };
                        callback(Ok(chunk));
                        if bytes_read == 0 {
                            break;
                        }
                    }
                    Err(e) => {
                        callback(Err(e.into()));
                        break;
                    }
                }
                std::thread::park();
            }
        })
    }
}
