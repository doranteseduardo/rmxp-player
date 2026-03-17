use crate::types::*;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarshalError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid marshal magic: expected 4.8, got {0}.{1}")]
    InvalidMagic(u8, u8),
    #[error("Unknown type indicator: 0x{0:02x} at position {1}")]
    UnknownType(u8, usize),
    #[error("Invalid symbol reference: {0}")]
    InvalidSymlink(usize),
    #[error("Invalid object reference: {0}")]
    InvalidObjectLink(usize),
    #[error("Unexpected end of data")]
    UnexpectedEof,
    #[error("Invalid UTF-8 symbol: {0}")]
    InvalidSymbol(String),
}

pub fn load<R: Read>(reader: R) -> Result<RubyValue, MarshalError> {
    let mut reader = MarshalReader::new(reader);
    reader.read()
}

pub fn load_file(path: impl AsRef<Path>) -> Result<RubyValue, MarshalError> {
    let file = File::open(path)?;
    load(file)
}

pub struct MarshalReader<R: Read> {
    reader: R,
    symbols: Vec<String>,
    objects: Vec<RubyValue>,
    position: usize,
}

impl<R: Read> MarshalReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            symbols: Vec::new(),
            objects: Vec::new(),
            position: 0,
        }
    }

    pub fn read(&mut self) -> Result<RubyValue, MarshalError> {
        let major = self.read_byte()?;
        let minor = self.read_byte()?;
        if major != 4 || minor != 8 {
            return Err(MarshalError::InvalidMagic(major, minor));
        }
        self.read_value()
    }

    fn read_byte(&mut self) -> Result<u8, MarshalError> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        self.position += 1;
        Ok(buf[0])
    }

    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>, MarshalError> {
        let mut buf = vec![0u8; len];
        self.reader.read_exact(&mut buf)?;
        self.position += len;
        Ok(buf)
    }

    fn read_long(&mut self) -> Result<i64, MarshalError> {
        let c = self.read_byte()? as i8;
        if c == 0 {
            return Ok(0);
        }
        if (1..=4).contains(&c) {
            let mut val = 0i64;
            for i in 0..(c as usize) {
                val |= (self.read_byte()? as i64) << (i * 8);
            }
            return Ok(val);
        }
        if (-4..=-1).contains(&c) {
            let mut val = -1i64;
            let count = (-c) as usize;
            for i in 0..count {
                let byte = self.read_byte()? as i64;
                val &= !(0xFF << (i * 8));
                val |= byte << (i * 8);
            }
            return Ok(val);
        }
        if c > 4 {
            Ok((c as i64) - 5)
        } else {
            Ok((c as i64) + 5)
        }
    }

    fn read_value(&mut self) -> Result<RubyValue, MarshalError> {
        let type_byte = self.read_byte()?;
        match type_byte {
            b'0' => Ok(RubyValue::Nil),
            b'T' => Ok(RubyValue::True),
            b'F' => Ok(RubyValue::False),
            b'i' => Ok(RubyValue::Integer(self.read_long()?)),
            b'f' => self.read_float(),
            b'"' => self.read_string(),
            b':' => self.read_symbol(),
            b';' => self.read_symbol_link(),
            b'@' => self.read_object_link(),
            b'[' => self.read_array(),
            b'{' => self.read_hash(),
            b'I' => self.read_with_ivars(),
            b'o' => self.read_object(),
            b'u' => self.read_user_defined(),
            b'U' => self.read_user_marshal(),
            b'S' => self.read_struct(),
            b'e' => self.read_extended(),
            b'c' => self.read_class(),
            b'm' => self.read_module(),
            b'/' => self.read_regexp(),
            _ => Err(MarshalError::UnknownType(type_byte, self.position)),
        }
    }

    fn read_float(&mut self) -> Result<RubyValue, MarshalError> {
        let len = self.read_long()? as usize;
        let bytes = self.read_bytes(len)?;
        let s = String::from_utf8_lossy(&bytes);
        let value = if s == "nan" {
            f64::NAN
        } else if s == "inf" {
            f64::INFINITY
        } else if s == "-inf" {
            f64::NEG_INFINITY
        } else {
            s.parse::<f64>().unwrap_or(0.0)
        };
        Ok(RubyValue::Float(value))
    }

    fn read_string(&mut self) -> Result<RubyValue, MarshalError> {
        let len = self.read_long()? as usize;
        let bytes = self.read_bytes(len)?;
        let string = RubyString::new(bytes);
        let idx = self.objects.len();
        self.objects.push(RubyValue::Nil);
        let value = RubyValue::String(string);
        self.objects[idx] = value.clone();
        Ok(value)
    }

    fn read_symbol(&mut self) -> Result<RubyValue, MarshalError> {
        let len = self.read_long()? as usize;
        let bytes = self.read_bytes(len)?;
        let sym =
            String::from_utf8(bytes).map_err(|err| MarshalError::InvalidSymbol(err.to_string()))?;
        self.symbols.push(sym.clone());
        Ok(RubyValue::Symbol(sym))
    }

    fn read_symbol_link(&mut self) -> Result<RubyValue, MarshalError> {
        let idx = self.read_long()? as usize;
        match self.symbols.get(idx) {
            Some(sym) => Ok(RubyValue::Symbol(sym.clone())),
            None => Err(MarshalError::InvalidSymlink(idx)),
        }
    }

    fn read_object_link(&mut self) -> Result<RubyValue, MarshalError> {
        let idx = self.read_long()? as usize;
        match self.objects.get(idx) {
            Some(value) => Ok(value.clone()),
            None => Err(MarshalError::InvalidObjectLink(idx)),
        }
    }

    fn read_array(&mut self) -> Result<RubyValue, MarshalError> {
        let idx = self.objects.len();
        self.objects.push(RubyValue::Nil);
        let len = self.read_long()? as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.read_value()?);
        }
        let value = RubyValue::Array(values);
        self.objects[idx] = value.clone();
        Ok(value)
    }

    fn read_hash(&mut self) -> Result<RubyValue, MarshalError> {
        let idx = self.objects.len();
        self.objects.push(RubyValue::Nil);
        let len = self.read_long()? as usize;
        let mut pairs = Vec::with_capacity(len);
        for _ in 0..len {
            let key = self.read_value()?;
            let val = self.read_value()?;
            pairs.push((key, val));
        }
        let value = RubyValue::Hash(pairs);
        self.objects[idx] = value.clone();
        Ok(value)
    }

    fn read_with_ivars(&mut self) -> Result<RubyValue, MarshalError> {
        let mut value = self.read_value()?;
        let count = self.read_long()? as usize;
        let mut attrs = Vec::with_capacity(count);
        for _ in 0..count {
            let key = self.read_symbol()?;
            let val = self.read_value()?;
            attrs.push((key, val));
        }
        match value {
            RubyValue::String(ref mut s) => {
                for (key, val) in attrs {
                    if let RubyValue::Symbol(sym) = key {
                        match sym.as_str() {
                            "E" => {
                                if matches!(val, RubyValue::True) {
                                    s.encoding = Some("UTF-8".into());
                                }
                            }
                            "encoding" => {
                                if let RubyValue::Symbol(enc) = val {
                                    s.encoding = Some(enc);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            RubyValue::Object(ref mut obj) => {
                for (key, val) in attrs {
                    if let RubyValue::Symbol(sym) = key {
                        obj.instance_vars.push((format!("@{}", sym), val));
                    }
                }
            }
            _ => {}
        }
        Ok(value)
    }

    fn read_object(&mut self) -> Result<RubyValue, MarshalError> {
        let class_name = match self.read_value()? {
            RubyValue::Symbol(sym) => sym,
            RubyValue::String(s) => s.to_string_lossy(),
            other => format!("{:?}", other),
        };
        let idx = self.objects.len();
        self.objects.push(RubyValue::Nil);
        let len = self.read_long()? as usize;
        let mut instance_vars = Vec::with_capacity(len);
        for _ in 0..len {
            let key = self.read_symbol()?;
            let val = self.read_value()?;
            if let RubyValue::Symbol(sym) = key {
                instance_vars.push((format!("@{}", sym), val));
            }
        }
        let value = RubyValue::Object(RubyObject {
            class_name,
            instance_vars,
        });
        self.objects[idx] = value.clone();
        Ok(value)
    }

    fn read_user_defined(&mut self) -> Result<RubyValue, MarshalError> {
        let class_name = match self.read_value()? {
            RubyValue::Symbol(sym) => sym,
            RubyValue::String(s) => s.to_string_lossy(),
            other => format!("{:?}", other),
        };
        let len = self.read_long()? as usize;
        let data = self.read_bytes(len)?;
        Ok(RubyValue::UserDefined { class_name, data })
    }

    fn read_user_marshal(&mut self) -> Result<RubyValue, MarshalError> {
        let class_name = match self.read_value()? {
            RubyValue::Symbol(sym) => sym,
            RubyValue::String(s) => s.to_string_lossy(),
            other => format!("{:?}", other),
        };
        let data = self.read_value()?;
        Ok(RubyValue::UserMarshal {
            class_name,
            data: Box::new(data),
        })
    }

    fn read_struct(&mut self) -> Result<RubyValue, MarshalError> {
        let name = match self.read_value()? {
            RubyValue::Symbol(sym) => sym,
            RubyValue::String(s) => s.to_string_lossy(),
            other => format!("{:?}", other),
        };
        let len = self.read_long()? as usize;
        let mut members = Vec::with_capacity(len);
        for _ in 0..len {
            let key = self.read_symbol()?;
            let val = self.read_value()?;
            if let RubyValue::Symbol(sym) = key {
                members.push((sym, val));
            }
        }
        Ok(RubyValue::Struct { name, members })
    }

    fn read_extended(&mut self) -> Result<RubyValue, MarshalError> {
        let module_name = match self.read_value()? {
            RubyValue::Symbol(sym) => sym,
            RubyValue::String(s) => s.to_string_lossy(),
            other => format!("{:?}", other),
        };
        let object = self.read_value()?;
        Ok(RubyValue::Extended {
            module_name,
            object: Box::new(object),
        })
    }

    fn read_class(&mut self) -> Result<RubyValue, MarshalError> {
        let len = self.read_long()? as usize;
        let bytes = self.read_bytes(len)?;
        let name =
            String::from_utf8(bytes).map_err(|err| MarshalError::InvalidSymbol(err.to_string()))?;
        Ok(RubyValue::Class(name))
    }

    fn read_module(&mut self) -> Result<RubyValue, MarshalError> {
        let len = self.read_long()? as usize;
        let bytes = self.read_bytes(len)?;
        let name =
            String::from_utf8(bytes).map_err(|err| MarshalError::InvalidSymbol(err.to_string()))?;
        Ok(RubyValue::Module(name))
    }

    fn read_regexp(&mut self) -> Result<RubyValue, MarshalError> {
        let len = self.read_long()? as usize;
        let pattern = self.read_bytes(len)?;
        let flags = self.read_byte()?;
        Ok(RubyValue::Regexp { pattern, flags })
    }
}
