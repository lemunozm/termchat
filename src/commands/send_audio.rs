use crate::action::{Action, Processing};
use crate::commands::{Command};
use crate::state::{State};
use crate::message::{NetMessage};
use crate::util::{Result};
use std::sync::mpsc;

use message_io::network::{Network};
use fon::{stereo::Stereo32, Audio};
use wavy::{Microphone};

const SAMPLE_RATE: f32 = 48_000.;

pub struct SendAudioCommand;
impl Command for SendAudioCommand {
    fn name(&self) -> &'static str {
        "sendaudio"
    }

    fn parse_params(&self, _params: &[&str]) -> Result<Box<dyn Action>> {
        match SendAudio::new() {
            Ok(action) => Ok(Box::new(action)),
            Err(e) => Err(e),
        }
    }
}
pub struct SendAudio {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl SendAudio {
    pub fn new() -> Result<SendAudio> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut microphone = Microphone::default();
            pasts::block_on(async move {
                loop {
                    let microphone = microphone.record().await;
                    let data =
                        Audio::with_frames(SAMPLE_RATE, microphone.collect::<Vec<Stereo32>>())
                            .as_f32_slice()
                            .iter()
                            .map(|f| f.to_le_bytes().to_vec())
                            .flatten()
                            .collect::<Vec<u8>>();
                    tx.send(data).unwrap();
                }
            });
        });
        Ok(Self { rx })
    }
}

impl Action for SendAudio {
    fn process(&mut self, state: &mut State, network: &mut Network) -> Processing {
        let audio: Vec<u8> = self.rx.try_iter().flatten().collect();
        let message = NetMessage::StreamAudio(audio);
        network.send_all(state.all_user_endpoints(), message);

        Processing::Partial
    }
}

// Stop stream logic

pub struct StopAudioCommand;

impl Command for StopAudioCommand {
    fn name(&self) -> &'static str {
        "stopaudio"
    }

    fn parse_params(&self, _params: &[&str]) -> Result<Box<dyn Action>> {
        Ok(Box::new(StopAudioStream {}))
    }
}
struct StopAudioStream {}
impl Action for StopAudioStream {
    fn process(&mut self, _state: &mut State, _network: &mut Network) -> Processing {
        Processing::Completed
    }
}
