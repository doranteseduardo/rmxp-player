use crate::types::RubyValue;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarshalWriteError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Marshal writer not implemented yet")]
    Unsupported,
}

pub struct MarshalWriter<W: Write> {
    writer: W,
}

impl<W: Write> MarshalWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn dump(self, _value: &RubyValue) -> Result<(), MarshalWriteError> {
        let _ = self.writer;
        Err(MarshalWriteError::Unsupported)
    }
}

pub fn dump<W: Write>(writer: W, value: &RubyValue) -> Result<(), MarshalWriteError> {
    MarshalWriter::new(writer).dump(value)
}

pub fn dump_file(path: impl AsRef<Path>, value: &RubyValue) -> Result<(), MarshalWriteError> {
    let file = File::create(path)?;
    dump(file, value)
}
