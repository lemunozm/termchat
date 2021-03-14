use serde::{Serialize, Deserialize};

pub struct Encoder {
    output_buffer: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Encoder {
        Self { output_buffer: Vec::new() }
    }

    pub fn encode<M: Serialize>(&mut self, message: M) -> &[u8] {
        self.output_buffer.clear();
        bincode::serialize_into(&mut self.output_buffer, &message).unwrap();
        &self.output_buffer
    }
}

pub fn decode<'a, M: Deserialize<'a>>(data_message: &'a [u8]) -> Option<M> {
    bincode::deserialize::<M>(data_message).ok()
}
