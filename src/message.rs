use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Chunk {
    Data(Vec<u8>),
    Error,
    End,
}

#[derive(Serialize, Deserialize)]
pub enum NetMessage {
    HelloLan(String, u16),                   // user_name, server_port
    HelloUser(String),                       // user_name
    UserMessage(String),                     // content
    UserData(String, Chunk),                 // file_name, chunk
    Stream(Option<(Vec<u8>, usize, usize)>), // Option of (stream_data width, height ) None means stream has ended
}
