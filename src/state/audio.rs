#[cfg(target_os = "linux")]
pub mod linux {
    use fon::{stereo::Stereo32, Audio, Sink};
    use wavy::{Speakers, SpeakersSink};
    use std::sync::mpsc;
    use super::super::State;

    pub struct AudioStream {
        tx: mpsc::Sender<Vec<u8>>,
        tx_stop: mpsc::Sender<()>,
    }

    impl AudioStream {
        fn update(&self, audio: Vec<u8>) {
            self.tx.send(audio).unwrap();
        }

        fn stop(&self) {
            self.tx_stop.send(()).unwrap();
        }
    }

    impl State {
        pub fn pulse_audio(&mut self, audio: Vec<u8>) {
            fn start_audio() -> AudioStream {
                const SAMPLE_RATE: f32 = 48_000.;
                let (tx, rx) = mpsc::channel();
                let (tx_stop, rx_stop) = mpsc::channel();
                std::thread::spawn(move || {
                    let mut speakers = Speakers::default();
                    let mut buffer: Audio<Stereo32> = Audio::with_silence(SAMPLE_RATE, 0);
                    futures::executor::block_on(async move {
                        loop {
                            let mut speakers: SpeakersSink<'_, Stereo32> = speakers.play().await;
                            speakers.stream(buffer.drain());
                            let data: Vec<u8> = rx.try_iter().flatten().collect();
                            let data: Vec<f32> = data
                                .chunks_exact(4)
                                .map(|array| std::convert::TryFrom::try_from(array).unwrap())
                                .map(f32::from_le_bytes)
                                .collect();
                            let mut audio: Audio<Stereo32> =
                                Audio::with_f32_buffer(SAMPLE_RATE, data);
                            buffer.extend(audio.drain());
                            if rx_stop.try_recv().is_ok() {
                                break
                            }
                        }
                    });
                });
                AudioStream { tx, tx_stop }
            }
            if self.audio.is_none() {
                self.audio = Some(start_audio());
            }
            // safe unwrap
            self.audio.as_ref().unwrap().update(audio);
        }

        pub fn stop_audio(&mut self) {
            if let Some(audio) = self.audio.take() {
                audio.stop();
            }
        }
    }
}
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
pub mod other {
    pub struct AudioStream;
    impl super::super::State {
        pub fn pulse_audio(&mut self, _audio: Vec<u8>) {}

        pub fn stop_audio(&mut self) {}
    }
}
#[cfg(not(target_os = "linux"))]
pub use other::*;
