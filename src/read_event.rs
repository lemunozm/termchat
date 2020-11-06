use crate::util::Result;
use std::sync::Arc;

type CallBack = Box<dyn Fn(std::fs::File, String, usize, usize) + Send + Sync>;

pub struct ReadFile {
    callback: CallBack,
    id: usize,
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
            callback: callback,
            id: 0,
        }
    }

    pub fn send(&mut self, file_name: String, path: std::path::PathBuf) -> Result<usize> {
        //let callback = self.callback.clone();

        //    std::thread::spawn(move || {
        use std::convert::TryInto;
        use std::io::Read;

        let try_read = || -> Result<(std::fs::File, usize)> {
            let file_size = std::fs::metadata(&path)?.len().try_into()?;
            let file = std::fs::File::open(path)?;
            Ok((file, file_size))
        };

        let (file, file_size) = match try_read() {
            Ok((file, file_size)) => (file, file_size),
            Err(e) => {
                //self.callback(Err(e));
                return Err(e);
            }
        };

        let send_id = self.id;
        (self.callback)(file, file_name, file_size, send_id);
        self.id += 1;

        Ok(send_id)
        /*
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
        }*/
        //    })
    }
}
