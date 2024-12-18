#[cfg(target_family = "windows")]
pub mod windows {
    use crate::{Error, SoundValue, Target, DEVICE_DEFAULT};
    use lazy_static::lazy_static;
    use std::collections::{HashMap, VecDeque};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::sleep;
    use std::time::Duration;
    use windows::core::HSTRING;
    use windows::Devices::Enumeration::{DeviceClass, DeviceInformation};
    use windows::Foundation::TypedEventHandler;
    use windows::Media::Core::MediaSource;
    use windows::Media::Playback::MediaPlayer;
    use windows::Media::SpeechSynthesis::SpeechSynthesizer;

    lazy_static! {
        static ref DEVICE_CACHE_INFO:Arc<HashMap<String, DeviceInformation>> = {
            let mut device_map = HashMap::new();
            if let Ok(op) =  DeviceInformation::FindAllAsyncDeviceClass(DeviceClass::AudioRender){
               if let Ok(async_operation) = op.get(){
                  for x in async_operation {
                     if let Ok(name) = x.Name(){
                         let name = name.to_string();
                            if let Ok(default) = x.IsDefault(){
                                if default {
                                    if let Ok(mut default) = DEVICE_DEFAULT.clone().try_write() {
                                        default.clear();
                                        default.push_str(&name);
                                    }
                                }
                            }
                         device_map.insert(name, x);
                     }
                  }
               }
            }
            Arc::new(device_map)
        };
    }

    #[derive(Debug)]
    pub(crate) struct WindowsTTs {
        tag: Arc<AtomicBool>,
        end: Arc<Mutex<bool>>,
        device: String,
        queue: Arc<Mutex<VecDeque<SoundValue>>>,
        state: Arc<Mutex<u8>>,
        play_count: Arc<AtomicU64>,
    }
    impl WindowsTTs {
        fn play(&self) -> Result<(), Error> {
           //sleep(Duration::from_millis(10));
            let next_text = {
                if let Ok(mut queue) = self.queue.clone().try_lock() {
                    queue.pop_front()
                } else {
                    None
                }
            };

            let stop = || {
                if let Ok(mut state) = self.state.clone().lock() {
                    *state = 0;
                }
            };

            let refresh = || {
                self.play_count.clone().store(0, Ordering::Relaxed);
            };

            let player = |next_text: SoundValue| -> Result<(), Error> {
                self.tag.clone().store(true, Ordering::Relaxed);
                let play_count = next_text.play_count;
                let loop_interval = next_text.play_interval;
                let str: String = next_text.into();
                if str.is_empty() {
                    return Ok(());
                }
                let player = MediaPlayer::new()?;
                let binding = DEVICE_CACHE_INFO.clone();
                let device = binding.get(&self.device);

                if let Some(device) = device{
                    let _ = player.SetAudioDevice(device);
                }

                let str = HSTRING::from(&str);
                let synthesizer = SpeechSynthesizer::new()?;
                let stream = synthesizer.SynthesizeTextToStreamAsync(&str)?.get()?;
                let media_source = MediaSource::CreateFromStream(&stream, &stream.ContentType()?)?;
                player.SetSource(&media_source)?;

                let end_clone = self.end.clone();
                player.MediaEnded(&TypedEventHandler::new(move |_sender: &Option<MediaPlayer>, _args| {
                    if let Ok(mut end) = end_clone.try_lock() {
                        *end = true;
                    }
                    Ok(())
                }))?;
                // 播放方法
                let play = || -> Result<(), Error> {
                    if play_count > 0 {
                        self.play_count.clone().fetch_add(1, Ordering::SeqCst);
                    }
                    if let Ok(mut end) = self.end.clone().try_lock() {
                        *end = false;
                    }
                    Ok(player.Play()?)
                };
                play()?;
                while self.tag.clone().load(Ordering::Relaxed) {
                    sleep(Duration::from_millis(10));
                    let end_x = if let Ok(end) = self.end.clone().try_lock() {
                        *end
                    } else {
                        false
                    };
                    if end_x {
                        let continue_playing = {
                            if play_count == 0 {
                                true
                            } else {
                                let count = self.play_count.clone().load(Ordering::Relaxed);
                                count < play_count
                            }
                        };
                        if continue_playing {
                            sleep(Duration::from_secs(loop_interval));
                            play()?
                        } else {
                            self.tag.clone().store(false, Ordering::Relaxed);
                        }
                    }
                }
                player.Pause()?;
                player.Close()?;
                media_source.Close()?;
                stream.Close()?;
                drop(player);
                drop(media_source);
                drop(stream);
                refresh();
                Ok(self.play()?)
            };

            if let Some(next_text) = next_text {
                player(next_text)?
            }
            stop();
            Ok(())
        }
    }
    impl Target for WindowsTTs {
        fn new(device: &str) -> Result<WindowsTTs, Error> {
            Ok(Self {
                tag: Arc::new(AtomicBool::new(false)),
                end: Arc::new(Mutex::new(false)),
                device: device.to_string(),
                queue: Arc::new(Mutex::new(VecDeque::<SoundValue>::new())),
                state: Arc::new(Mutex::new(0)),
                play_count: Arc::new(AtomicU64::new(0)),
            })
        }

        fn devices() -> Vec<String> {
            DEVICE_CACHE_INFO.clone().keys().map(|k| k.to_string()).collect::<Vec<String>>()
        }

        fn default_device() -> Option<String> {
            if let Ok(default) = DEVICE_DEFAULT.clone().try_read() {
                Some(default.to_string())
            } else {
                None
            }
        }

        fn speak(&self, context: SoundValue, interrupt: bool) -> Result<(), Error> {
            if interrupt {
                self.stop()?;
            }
            if let Ok(mut guard) =  self.queue.clone().try_lock() {
                guard.push_back(context);
                if let Ok(mut state) = self.state.clone().try_lock() {
                    if *state == 1u8 {
                        return Ok(());
                    } else {
                        *state = 1;
                    }
                }
            }
            self.play()?;
            Ok(())
        }
        fn is_playing(&self) -> Result<bool, Error> {
            if let Ok(state) = self.state.clone().try_lock() {
                return Ok(*state > 0);
            }
            Ok(false)
        }


        fn stop(&self) -> Result<(), Error> {
            let is_playing = self.is_playing()?;
            if is_playing {
                if let Ok(mut queue) = self.queue.clone().try_lock() {
                    queue.clear();
                }

                self.tag.clone().store(false, Ordering::Relaxed);
            }
            Ok(())
        }
    }


    impl From<windows_result::Error> for Error {
        fn from(value: windows_result::Error) -> Self {
            Error::Windows(value)
        }
    }
}