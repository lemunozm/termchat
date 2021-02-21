use crate::action::{Action, Processing};
use crate::commands::{Command};
use crate::state::{State};
use crate::message::{NetMessage};
use crate::util::{Result, Reportable};

use message_io::network::{Network};
use v4l::prelude::*;
use v4l::FourCC;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::video::traits::Capture;

use std::time::{Duration};

// Send Stream logic

pub struct SendStreamCommand;
impl Command for SendStreamCommand {
    fn name(&self) -> &'static str {
        "startstream"
    }

    fn parse_params(&self, _params: Vec<String>) -> Result<Box<dyn Action>> {
        match SendStream::new() {
            Ok(action) => Ok(Box::new(action)),
            Err(e) => Err(e),
        }
    }
}
pub struct SendStream {
    stream: MmapStream<'static>,
    width: usize,
    height: usize,
}

impl SendStream {
    pub fn new() -> Result<SendStream> {
        let dev = Device::new(0).expect("Failed to open device");
        let mut fmt = dev.format()?;
        fmt.fourcc = FourCC::new(b"YUYV");
        let width = fmt.width as usize;
        let height = fmt.height as usize;
        dev.set_format(&fmt)?;

        let stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4)?;

        Ok(SendStream { stream, width, height })
    }
}

impl Action for SendStream {
    fn process(&mut self, mut state: &mut State, network: &mut Network) -> Processing {
        if state.stop_stream {
            // stop stream and restore stop_stream to false for the next stream usage
            state.stop_stream = false;
            network.send_all(state.all_user_endpoints(), NetMessage::Stream(None));
            return Processing::Completed
        }
        let (data, _metadata) = match self.stream.next() {
            Ok(d) => d,
            Err(e) => {
                e.to_string().report_err(&mut state);
                network.send_all(state.all_user_endpoints(), NetMessage::Stream(None));
                return Processing::Completed
            }
        };
        #[allow(non_snake_case)]
        let data: Vec<u8> = data.chunks_exact(4).fold(vec![], |mut acc, v| {
            // convert form YUYV to RGB
            let [Y, U, _, V]: [u8; 4] = std::convert::TryFrom::try_from(v).unwrap();
            let Y = Y as f32;
            let U = U as f32;
            let V = V as f32;

            let b = 1.164 * (Y - 16.) + 2.018 * (U - 128.);

            let g = 1.164 * (Y - 16.) - 0.813 * (V - 128.) - 0.391 * (U - 128.);

            let r = 1.164 * (Y - 16.) + 1.596 * (V - 128.);
            let r = r as u8;
            let g = g as u8;
            let b = b as u8;
            acc.push(r);
            acc.push(g);
            acc.push(b);
            acc
        });

        let message = NetMessage::Stream(Some((data, self.width, self.height)));
        network.send_all(state.all_user_endpoints(), message);

        Processing::Partial(Duration::from_millis(16)) //~60fps - delay of computation
    }
}

// Stop stream logic

pub struct StopStreamCommand;

impl Command for StopStreamCommand {
    fn name(&self) -> &'static str {
        "stopstream"
    }

    fn parse_params(&self, _params: Vec<String>) -> Result<Box<dyn Action>> {
        Ok(Box::new(StopStream {}))
    }
}
struct StopStream {}
impl Action for StopStream {
    fn process(&mut self, state: &mut State, _network: &mut Network) -> Processing {
        state.stop_stream = true;
        Processing::Completed
    }
}
