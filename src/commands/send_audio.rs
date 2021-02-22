#[cfg(target_os = "linux")]
pub mod linux {
    use crate::action::{Action, Processing};
    use crate::commands::{Command};
    use crate::state::{State};
    use crate::message::{NetMessage};
    use crate::util::{Result};
    use std::sync::mpsc;
    use std::time::Duration;

    use message_io::network::{Network};
    use fon::{stereo::Stereo32, Audio};
    use wavy::{Microphone};

    const SAMPLE_RATE: f32 = 48_000.;

    pub struct SendAudioCommand;
    impl Command for SendAudioCommand {
        fn name(&self) -> &'static str {
            "sendaudio"
        }

        fn parse_params(&self, _params: Vec<String>) -> Result<Box<dyn Action>> {
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
                futures::executor::block_on(async move {
                    loop {
                        let microphone = microphone.record().await;
                        let data =
                            Audio::with_frames(SAMPLE_RATE, microphone.collect::<Vec<Stereo32>>())
                                .as_f32_slice()
                                .iter()
                                .map(|f| f.to_le_bytes().to_vec())
                                .flatten()
                                .collect::<Vec<u8>>();
                        if tx.send(data).is_err() {
                            break
                        }
                    }
                });
            });
            Ok(Self { rx })
        }
    }

    impl Action for SendAudio {
        fn process(&mut self, state: &mut State, network: &mut Network) -> Processing {
            // if stop audio is true stop the audio stream
            // SendAudio struct will be dropped with its receiver, so the sender will receive an error
            // that will cause it to break out of the loop
            if state.stop_audio {
                // reset the flag
                state.stop_audio = false;
                // send None so the reciever nows that the stream has ended
                let message = NetMessage::StreamAudio(None);
                network.send_all(state.all_user_endpoints(), message);
                return Processing::Completed
            }

            let audio: Vec<u8> = self.rx.try_iter().flatten().collect();
            let message = NetMessage::StreamAudio(Some(audio));
            network.send_all(state.all_user_endpoints(), message);

            Processing::Partial(Duration::from_millis(10))
        }
    }

    // Stop stream logic

    pub struct StopAudioCommand;

    impl Command for StopAudioCommand {
        fn name(&self) -> &'static str {
            "stopaudio"
        }

        fn parse_params(&self, _params: Vec<String>) -> Result<Box<dyn Action>> {
            Ok(Box::new(StopAudioStream {}))
        }
    }
    struct StopAudioStream {}
    impl Action for StopAudioStream {
        fn process(&mut self, state: &mut State, _network: &mut Network) -> Processing {
            state.stop_audio = true;
            Processing::Completed
        }
    }
}
#[cfg(target_os = "linux")]
pub use linux::*;

crate::generate_unsupported!(SendAudioCommand StopAudioCommand);
