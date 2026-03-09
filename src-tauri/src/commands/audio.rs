use rodio::{buffer::SamplesBuffer, Decoder, OutputStream, Sink, Source};
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

/// Messages sent to the dedicated audio thread.
enum AudioMsg {
    Play {
        path: PathBuf,
        volume: f32,
        result_tx: mpsc::Sender<Result<(), String>>,
    },
    PlayMidi {
        samples: Vec<f32>,
        sample_rate: u32,
        volume: f32,
        result_tx: mpsc::Sender<Result<(), String>>,
    },
    Stop,
    IsPlaying {
        result_tx: mpsc::Sender<bool>,
    },
    Shutdown,
}

/// Handle to communicate with the audio thread.
pub struct AudioHandle {
    tx: mpsc::Sender<AudioMsg>,
}

impl AudioHandle {
    /// Spawn a dedicated audio thread and return a handle to it.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<AudioMsg>();

        std::thread::spawn(move || {
            // Create the output stream on this thread — it stays here and is never sent.
            let stream_result = OutputStream::try_default();
            let (_stream, handle) = match stream_result {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("[audio] Failed to open audio device: {}", e);
                    // Drain messages so senders don't block
                    for msg in rx {
                        match msg {
                            AudioMsg::Play { result_tx, .. } => {
                                let _ = result_tx.send(Err(format!("No audio device: {}", e)));
                            }
                            AudioMsg::IsPlaying { result_tx, .. } => {
                                let _ = result_tx.send(false);
                            }
                            AudioMsg::Shutdown => break,
                            _ => {}
                        }
                    }
                    return;
                }
            };

            let mut current_sink: Option<Sink> = None;

            for msg in rx {
                match msg {
                    AudioMsg::Play { path, volume, result_tx } => {
                        // Stop previous
                        if let Some(old) = current_sink.take() {
                            old.stop();
                        }

                        let result = (|| -> Result<Sink, String> {
                            let file = File::open(&path)
                                .map_err(|e| format!("Failed to open audio file: {}", e))?;
                            let reader = BufReader::new(file);
                            let source = Decoder::new(reader)
                                .map_err(|e| format!("Failed to decode audio: {}", e))?;
                            let sink = Sink::try_new(&handle)
                                .map_err(|e| format!("Failed to create sink: {}", e))?;
                            // rodio sources are usually 44100 or 48000.
                            sink.set_volume(volume);
                            sink.append(source);
                            Ok(sink)
                        })();

                        if let Ok(sink) = result {
                            current_sink = Some(sink);
                            let _ = result_tx.send(Ok(()));
                        } else if let Err(e) = result {
                            let _ = result_tx.send(Err(e));
                        }
                    }
                    AudioMsg::PlayMidi {
                        samples,
                        sample_rate,
                        volume,
                        result_tx,
                    } => {
                        if let Some(old) = current_sink.take() {
                            old.stop();
                        }

                        let res = (|| -> Result<Sink, String> {
                            let sink = Sink::try_new(&handle)
                                .map_err(|e| format!("Failed to create sink: {}", e))?;
                            
                            let source = SamplesBuffer::new(2, sample_rate, samples);
                            sink.set_volume(volume);
                            sink.append(source);
                            Ok(sink)
                        })();

                        match res {
                            Ok(sink) => {
                                current_sink = Some(sink);
                                let _ = result_tx.send(Ok(()));
                            }
                            Err(e) => {
                                let _ = result_tx.send(Err(e));
                            }
                        }
                    }
                    AudioMsg::Stop => {
                        if let Some(old) = current_sink.take() {
                            old.stop();
                        }
                    }
                    AudioMsg::IsPlaying { result_tx } => {
                        let playing = current_sink
                            .as_ref()
                            .map(|s| !s.empty())
                            .unwrap_or(false);
                        let _ = result_tx.send(playing);
                    }
                    AudioMsg::Shutdown => {
                        if let Some(old) = current_sink.take() {
                            old.stop();
                        }
                        break;
                    }
                }
            }

            eprintln!("[audio] Audio thread exiting");
        });

        AudioHandle { tx }
    }
}

/// Resolve an audio file path from the project directory.
fn resolve_audio_path(
    project_path: &str,
    asset_type: &str,
    asset_name: &str,
) -> Result<PathBuf, String> {
    let project = PathBuf::from(project_path);
    let (base_dir, dir, extensions): (&str, &str, &[&str]) = match asset_type {
        "bgm" => ("Audio", "BGM", &["ogg", "mp3", "wav", "mid", "midi", "wma"]),
        "bgs" => ("Audio", "BGS", &["ogg", "mp3", "wav", "mid", "midi", "wma"]),
        "me" => ("Audio", "ME", &["ogg", "mp3", "wav", "mid", "midi", "wma"]),
        "se" => ("Audio", "SE", &["ogg", "mp3", "wav", "mid", "midi", "wma"]),
        _ => return Err(format!("Not an audio type: {}", asset_type)),
    };

    let base_path = project.join(base_dir).join(dir).join(asset_name);

    for ext in extensions {
        let path = base_path.with_extension(ext);
        if path.exists() {
            // Found a valid file
            return Ok(path);
        }
    }
    if base_path.exists() {
        return Ok(base_path);
    }

    Err(format!(
        "Audio file not found: {}/{}",
        asset_type, asset_name
    ))
}

fn synthesize_midi(path: &PathBuf, project_path: &str) -> Result<(Vec<f32>, u32), String> {
    eprintln!("[audio] Synthesizing MIDI: {:?}", path);
    // Try to find soundfont
    let project = PathBuf::from(project_path);
    // Check common locations for SoundFont
    let candidates = [
        project.join("soundfont.sf2"),
        project.join("Data/soundfont.sf2"),
        PathBuf::from("soundfont.sf2"), // Workspace root fallback
    ];
    
    let sf2_path = candidates.iter().find(|p| p.exists())
        .ok_or("soundfont.sf2 not found. Please place 'soundfont.sf2' in your game folder.")?
        .clone();
    
    let mut sf2_file = File::open(&sf2_path).map_err(|e| format!("SF2 error: {}", e))?;
    let sound_font = Arc::new(SoundFont::new(&mut sf2_file).map_err(|_| "Invalid SoundFont file")?);
    
    let sample_rate = 44100;
    let settings = SynthesizerSettings::new(sample_rate as i32);
    let synthesizer = Synthesizer::new(&sound_font, &settings).map_err(|_| "Synth init failed")?;
    
    let mut midi_file_handle = File::open(path).map_err(|e| format!("MIDI error: {}", e))?;
    let midi_file = Arc::new(MidiFile::new(&mut midi_file_handle).map_err(|_| "Invalid MIDI file")?);
    
    let mut sequencer = MidiFileSequencer::new(synthesizer);
    sequencer.play(&midi_file, true);
    
    // Render the MIDI (limit to ~3 minutes to prevent huge memory usage for now)
    let duration = midi_file.get_length(); 
    let max_duration = 180.0; 
    let render_duration = if duration > 0.0 { duration.min(max_duration) } else { max_duration };
    
    let sample_count = (render_duration * sample_rate as f64).ceil() as usize;
    let mut left = vec![0.0f32; sample_count];
    let mut right = vec![0.0f32; sample_count];
    
    sequencer.render(&mut left, &mut right);
    
    // Interleave
    let mut interleaved = Vec::with_capacity(sample_count * 2);
    for i in 0..sample_count {
        interleaved.push(left[i]);
        interleaved.push(right[i]);
    }
    
    Ok((interleaved, sample_rate))
}

/// Preview an audio file. Stops any currently playing preview first.
#[tauri::command]
pub async fn preview_audio(
    audio: tauri::State<'_, Mutex<AudioHandle>>,
    project_path: String,
    asset_type: String,
    asset_name: String,
    volume: f32,
) -> Result<(), String> {
    let path = resolve_audio_path(&project_path, &asset_type, &asset_name)?;

    eprintln!("[audio] Playing preview: {:?}", path);

    let (result_tx, result_rx) = mpsc::channel();
    
    // Check if it's MIDI
    let is_midi = path.extension().map_or(false, |ext| {
        let e = ext.to_string_lossy().to_lowercase();
        e == "mid" || e == "midi"
    });

    if is_midi {
        let (samples, sample_rate) = synthesize_midi(&path, &project_path)?;
        let handle = audio.lock().map_err(|e| format!("Lock error: {}", e))?;
        handle
            .tx
            .send(AudioMsg::PlayMidi {
                samples,
                sample_rate,
                volume,
                result_tx,
            })
            .map_err(|_| "Audio thread not running".to_string())?;
    } else {
        let handle = audio.lock().map_err(|e| format!("Lock error: {}", e))?;
        handle
            .tx
            .send(AudioMsg::Play {
                path,
                volume,
                result_tx,
            })
            .map_err(|_| "Audio thread not running".to_string())?;
    }

    result_rx
        .recv()
        .map_err(|_| "Audio thread did not respond".to_string())?
}

/// Stop the currently playing audio preview.
#[tauri::command]
pub async fn stop_audio(audio: tauri::State<'_, Mutex<AudioHandle>>) -> Result<(), String> {
    let handle = audio.lock().map_err(|e| format!("Lock error: {}", e))?;
    let _ = handle.tx.send(AudioMsg::Stop);
    eprintln!("[audio] Stopped preview");
    Ok(())
}

/// Check if audio is currently playing.
#[tauri::command]
pub async fn is_audio_playing(audio: tauri::State<'_, Mutex<AudioHandle>>) -> Result<bool, String> {
    let (result_tx, result_rx) = mpsc::channel();
    {
        let handle = audio.lock().map_err(|e| format!("Lock error: {}", e))?;
        handle
            .tx
            .send(AudioMsg::IsPlaying { result_tx })
            .map_err(|_| "Audio thread not running".to_string())?;
    }
    result_rx
        .recv()
        .map_err(|_| "Audio thread did not respond".to_string())
}
