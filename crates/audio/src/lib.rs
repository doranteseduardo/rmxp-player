use rodio::{
    decoder::{Decoder, DecoderError},
    source::Source,
    OutputStream, OutputStreamHandle, PlayError, Sink,
};
use std::{
    collections::VecDeque,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use thiserror::Error;
use tracing::{info, warn};

type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to initialize audio output: {0}")]
    Init(#[from] rodio::StreamError),
    #[error("Failed to open audio file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to decode audio file {path}: {source}")]
    Decode { path: PathBuf, source: DecoderError },
    #[error("Failed to create audio sink: {0}")]
    Sink(#[from] PlayError),
}

pub struct AudioSystem {
    _stream: OutputStream,
    handle: AudioHandle,
}

#[derive(Clone)]
pub struct AudioHandle {
    mixer: Arc<AudioMixer>,
}

/// Last-played BGM parameters, saved so ME auto-resume can restore it.
#[derive(Clone)]
struct BgmState {
    path: PathBuf,
    volume: i32,
    position_ms: u32,
}

struct AudioMixer {
    handle: OutputStreamHandle,
    bgm: Mutex<Option<Arc<Sink>>>,
    /// Saved BGM info — updated every time play_bgm is called.
    bgm_state: Mutex<Option<BgmState>>,
    bgs: Mutex<Option<Arc<Sink>>>,
    me: Mutex<Option<Arc<Sink>>>,
    se_sinks: Mutex<VecDeque<Arc<Sink>>>,
    se_limit: usize,
}

impl AudioSystem {
    pub fn new() -> Result<Self> {
        let (stream, handle) = OutputStream::try_default().map_err(AudioError::from)?;
        info!(target: "audio", "Initialized default audio output");
        let mixer = Arc::new(AudioMixer::new(handle));
        Ok(Self {
            _stream: stream,
            handle: AudioHandle { mixer },
        })
    }

    pub fn handle(&self) -> AudioHandle {
        self.handle.clone()
    }
}

impl AudioHandle {
    pub fn play_bgm(&self, path: &Path, volume: i32, pitch: i32, position_ms: u32) -> Result<()> {
        self.mixer.play_bgm(path, volume, pitch, position_ms)
    }

    pub fn stop_bgm(&self) {
        self.mixer.stop_bgm();
    }

    pub fn fade_bgm(&self, frames: u32) {
        self.mixer.fade_bgm(frames);
    }

    pub fn play_bgs(&self, path: &Path, volume: i32, pitch: i32, position_ms: u32) -> Result<()> {
        self.mixer.play_bgs(path, volume, pitch, position_ms)
    }

    pub fn stop_bgs(&self) {
        self.mixer.stop_bgs();
    }

    pub fn fade_bgs(&self, frames: u32) {
        self.mixer.fade_bgs(frames);
    }

    /// Play a Music Effect. The current BGM is paused and automatically
    /// restored once the ME finishes playing.
    pub fn play_me(&self, path: &Path, volume: i32, pitch: i32) -> Result<()> {
        // Snapshot BGM state *before* starting the ME.
        let bgm_snapshot = self.mixer.bgm_state.lock().ok().and_then(|g| g.clone());

        // Stop BGM so the ME plays cleanly.
        self.mixer.stop_bgm();

        let me_sink = self.mixer.play_me_inner(path, volume, pitch)?;

        // Spawn a monitor thread that restores BGM once the ME sink is empty.
        if let Some(state) = bgm_snapshot {
            let mixer = Arc::clone(&self.mixer);
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_millis(100));
                    if me_sink.empty() {
                        break;
                    }
                }
                if let Err(err) = mixer.play_bgm(&state.path, state.volume, 100, state.position_ms)
                {
                    warn!(target: "audio", error = %err, "ME auto-resume: failed to restore BGM");
                }
            });
        }

        Ok(())
    }

    pub fn stop_me(&self) {
        self.mixer.stop_me();
    }

    pub fn fade_me(&self, frames: u32) {
        self.mixer.fade_me(frames);
    }

    pub fn play_se(&self, path: &Path, volume: i32, pitch: i32) -> Result<()> {
        self.mixer.play_se(path, volume, pitch)
    }

    pub fn stop_all_se(&self) {
        self.mixer.stop_all_se();
    }

    pub fn fade_all_se(&self, frames: u32) {
        self.mixer.fade_all_se(frames);
    }
}

impl AudioMixer {
    fn new(handle: OutputStreamHandle) -> Self {
        Self {
            handle,
            bgm: Mutex::new(None),
            bgm_state: Mutex::new(None),
            bgs: Mutex::new(None),
            me: Mutex::new(None),
            se_sinks: Mutex::new(VecDeque::new()),
            se_limit: 24,
        }
    }

    pub fn play_bgm(&self, path: &Path, volume: i32, pitch: i32, position_ms: u32) -> Result<()> {
        self.warn_if_pitch_changed("bgm_play", pitch);
        let sink = self.build_sink(path, position_ms, true)?;
        sink.set_volume(volume_to_gain(volume));
        // Save state so ME auto-resume can restore it.
        if let Ok(mut state) = self.bgm_state.lock() {
            *state = Some(BgmState {
                path: path.to_path_buf(),
                volume,
                position_ms,
            });
        }
        self.swap_sink(&self.bgm, sink);
        Ok(())
    }

    pub fn stop_bgm(&self) {
        self.stop_channel(&self.bgm);
    }

    pub fn fade_bgm(&self, frames: u32) {
        self.fade_channel(&self.bgm, frames);
    }

    pub fn play_bgs(&self, path: &Path, volume: i32, pitch: i32, position_ms: u32) -> Result<()> {
        self.warn_if_pitch_changed("bgs_play", pitch);
        let sink = self.build_sink(path, position_ms, true)?;
        sink.set_volume(volume_to_gain(volume));
        self.swap_sink(&self.bgs, sink);
        Ok(())
    }

    pub fn stop_bgs(&self) {
        self.stop_channel(&self.bgs);
    }

    pub fn fade_bgs(&self, frames: u32) {
        self.fade_channel(&self.bgs, frames);
    }

    /// Internal: create the ME sink, store it, and return a clone for monitoring.
    fn play_me_inner(&self, path: &Path, volume: i32, pitch: i32) -> Result<Arc<Sink>> {
        self.warn_if_pitch_changed("me_play", pitch);
        let sink = self.build_sink(path, 0, false)?;
        sink.set_volume(volume_to_gain(volume));
        self.swap_sink(&self.me, Arc::clone(&sink));
        Ok(sink)
    }

    pub fn stop_me(&self) {
        self.stop_channel(&self.me);
    }

    pub fn fade_me(&self, frames: u32) {
        self.fade_channel(&self.me, frames);
    }

    pub fn play_se(&self, path: &Path, volume: i32, pitch: i32) -> Result<()> {
        self.warn_if_pitch_changed("se_play", pitch);
        let sink = self.build_sink(path, 0, false)?;
        sink.set_volume(volume_to_gain(volume));
        let mut queue = self.se_sinks.lock().expect("SE mutex poisoned");
        queue.retain(|s| !s.empty());
        if queue.len() >= self.se_limit {
            if let Some(old) = queue.pop_front() {
                old.stop();
            }
        }
        queue.push_back(sink);
        Ok(())
    }

    pub fn stop_all_se(&self) {
        let mut queue = self.se_sinks.lock().expect("SE mutex poisoned");
        for sink in queue.drain(..) {
            sink.stop();
        }
    }

    pub fn fade_all_se(&self, frames: u32) {
        let mut queue = self.se_sinks.lock().expect("SE mutex poisoned");
        for sink in queue.iter() {
            fade_and_stop(sink.clone(), frames);
        }
        queue.retain(|s| !s.empty());
    }

    fn build_sink(&self, path: &Path, position_ms: u32, looping: bool) -> Result<Arc<Sink>> {
        let buffered = self.open_buffered(path)?;
        let skip_duration = Duration::from_millis(position_ms as u64);
        let skipped = buffered
            .skip_duration(skip_duration)
            .convert_samples::<f32>();
        let source: Box<dyn Source<Item = f32> + Send> = if looping {
            Box::new(skipped.repeat_infinite())
        } else {
            Box::new(skipped)
        };
        let sink = Sink::try_new(&self.handle).map_err(AudioError::from)?;
        sink.append(source);
        sink.play();
        Ok(Arc::new(sink))
    }

    fn open_buffered(
        &self,
        path: &Path,
    ) -> Result<rodio::source::Buffered<Decoder<BufReader<File>>>> {
        let file = File::open(path).map_err(|source| AudioError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let decoder = Decoder::new(BufReader::new(file)).map_err(|source| AudioError::Decode {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(decoder.buffered())
    }

    fn swap_sink(&self, slot: &Mutex<Option<Arc<Sink>>>, sink: Arc<Sink>) {
        let mut guard = slot.lock().expect("audio channel mutex poisoned");
        if let Some(old) = guard.replace(sink) {
            old.stop();
        }
    }

    fn stop_channel(&self, slot: &Mutex<Option<Arc<Sink>>>) {
        if let Some(sink) = slot.lock().expect("audio channel mutex poisoned").take() {
            sink.stop();
        }
    }

    fn fade_channel(&self, slot: &Mutex<Option<Arc<Sink>>>, frames: u32) {
        if let Some(sink) = slot
            .lock()
            .expect("audio channel mutex poisoned")
            .as_ref()
            .cloned()
        {
            fade_and_stop(sink, frames);
        }
    }

    fn warn_if_pitch_changed(&self, label: &str, pitch: i32) {
        if pitch != 100 {
            warn!(
                target: "audio",
                method = %label,
                pitch,
                "Pitch adjustment is not currently supported"
            );
        }
    }
}

fn volume_to_gain(volume: i32) -> f32 {
    (volume as f32 / 100.0).clamp(0.0, 1.0)
}

fn fade_and_stop(sink: Arc<Sink>, frames: u32) {
    if frames == 0 {
        sink.stop();
        return;
    }
    let steps = frames.max(1);
    let total_secs = frames as f64 / 60.0;
    let step_secs = if steps > 0 {
        total_secs / steps as f64
    } else {
        0.0
    };
    let start_volume = sink.volume();
    thread::spawn(move || {
        for remaining in (0..steps).rev() {
            let ratio = remaining as f32 / steps as f32;
            sink.set_volume((start_volume * ratio.max(0.0)).max(0.0));
            if step_secs > 0.0 {
                thread::sleep(Duration::from_secs_f64(step_secs));
            }
        }
        sink.stop();
    });
}
