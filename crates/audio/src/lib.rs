use anyhow::Result;
use rodio::{OutputStream, OutputStreamHandle};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to initialize audio output: {0}")]
    Init(#[from] rodio::StreamError),
}

/// Thin wrapper around a rodio output stream.
pub struct AudioSystem {
    #[allow(dead_code)]
    stream: OutputStream,
    handle: OutputStreamHandle,
}

impl AudioSystem {
    pub fn new() -> Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        info!(target: "audio", "Initialized default audio output");
        Ok(Self { stream, handle })
    }

    pub fn handle(&self) -> &OutputStreamHandle {
        &self.handle
    }
}
