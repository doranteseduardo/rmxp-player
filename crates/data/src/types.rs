use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum RubyValue {
    Nil,
    True,
    False,
    Integer(i64),
    Float(f64),
    String(RubyString),
    Symbol(String),
    Array(Vec<RubyValue>),
    Hash(Vec<(RubyValue, RubyValue)>),
    Object(RubyObject),
    UserDefined {
        class_name: String,
        data: Vec<u8>,
    },
    UserMarshal {
        class_name: String,
        data: Box<RubyValue>,
    },
    Regexp {
        pattern: Vec<u8>,
        flags: u8,
    },
    Struct {
        name: String,
        members: Vec<(String, RubyValue)>,
    },
    Extended {
        module_name: String,
        object: Box<RubyValue>,
    },
    Class(String),
    Module(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubyString {
    pub bytes: Vec<u8>,
    pub encoding: Option<String>,
}

impl RubyString {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            encoding: None,
        }
    }

    pub fn with_encoding(bytes: Vec<u8>, encoding: String) -> Self {
        Self {
            bytes,
            encoding: Some(encoding),
        }
    }

    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubyObject {
    pub class_name: String,
    pub instance_vars: Vec<(String, RubyValue)>,
}

impl RubyObject {
    pub fn new(class_name: String) -> Self {
        Self {
            class_name,
            instance_vars: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&RubyValue> {
        let ivar = if name.starts_with('@') {
            name.to_string()
        } else {
            format!("@{}", name)
        };
        self.instance_vars
            .iter()
            .find(|(k, _)| k == &ivar)
            .map(|(_, v)| v)
    }

    pub fn get_int(&self, name: &str) -> Option<i64> {
        match self.get(name)? {
            RubyValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.get(name)? {
            RubyValue::True => Some(true),
            RubyValue::False => Some(false),
            _ => None,
        }
    }

    pub fn get_string(&self, name: &str) -> Option<String> {
        match self.get(name)? {
            RubyValue::String(s) => Some(s.to_string_lossy()),
            RubyValue::Symbol(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl RubyValue {
    pub fn to_json_value(&self) -> Value {
        match self {
            RubyValue::Nil => Value::Null,
            RubyValue::True => Value::Bool(true),
            RubyValue::False => Value::Bool(false),
            RubyValue::Integer(v) => Value::Number((*v).into()),
            RubyValue::Float(v) => serde_json::Number::from_f64(*v)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            RubyValue::String(s) => Value::String(s.to_string_lossy()),
            RubyValue::Symbol(s) => Value::String(format!(":{}", s)),
            RubyValue::Array(items) => {
                Value::Array(items.iter().map(|v| v.to_json_value()).collect())
            }
            RubyValue::Hash(pairs) => {
                let mut map = serde_json::Map::new();
                for (k, v) in pairs {
                    let key = match k {
                        RubyValue::String(s) => s.to_string_lossy(),
                        RubyValue::Symbol(sym) => sym.clone(),
                        RubyValue::Integer(i) => i.to_string(),
                        _ => format!("{:?}", k),
                    };
                    map.insert(key, v.to_json_value());
                }
                Value::Object(map)
            }
            RubyValue::Object(obj) => {
                let mut map = serde_json::Map::new();
                map.insert("__class".into(), Value::String(obj.class_name.clone()));
                for (name, val) in &obj.instance_vars {
                    map.insert(
                        name.trim_start_matches('@').to_string(),
                        val.to_json_value(),
                    );
                }
                Value::Object(map)
            }
            RubyValue::UserDefined { class_name, data } => decode_user_defined(class_name, data),
            RubyValue::UserMarshal { class_name, data } => {
                let mut map = serde_json::Map::new();
                map.insert("__class".into(), Value::String(class_name.clone()));
                map.insert("value".into(), data.to_json_value());
                Value::Object(map)
            }
            RubyValue::Regexp { pattern, flags } => {
                let mut map = serde_json::Map::new();
                map.insert(
                    "pattern".into(),
                    Value::String(String::from_utf8_lossy(pattern).into_owned()),
                );
                map.insert("flags".into(), Value::Number((*flags).into()));
                Value::Object(map)
            }
            RubyValue::Struct { name, members } => {
                let mut map = serde_json::Map::new();
                map.insert("__struct".into(), Value::String(name.clone()));
                for (k, v) in members {
                    map.insert(k.clone(), v.to_json_value());
                }
                Value::Object(map)
            }
            RubyValue::Extended {
                module_name,
                object,
            } => {
                let mut map = serde_json::Map::new();
                map.insert("__extended".into(), Value::String(module_name.clone()));
                map.insert("object".into(), object.to_json_value());
                Value::Object(map)
            }
            RubyValue::Class(name) => Value::String(format!("Class({})", name)),
            RubyValue::Module(name) => Value::String(format!("Module({})", name)),
        }
    }
}

fn decode_user_defined(class_name: &str, data: &[u8]) -> Value {
    match class_name {
        "Color" | "Tone" if data.len() >= 32 => {
            let r = f64::from_le_bytes(data[0..8].try_into().unwrap());
            let g = f64::from_le_bytes(data[8..16].try_into().unwrap());
            let b = f64::from_le_bytes(data[16..24].try_into().unwrap());
            let a = f64::from_le_bytes(data[24..32].try_into().unwrap());
            let mut map = serde_json::Map::new();
            map.insert("__class".into(), Value::String(class_name.into()));
            map.insert(
                "red".into(),
                serde_json::Number::from_f64(r)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            map.insert(
                "green".into(),
                serde_json::Number::from_f64(g)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            map.insert(
                "blue".into(),
                serde_json::Number::from_f64(b)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            let label = if class_name == "Color" {
                "alpha"
            } else {
                "gray"
            };
            map.insert(
                label.into(),
                serde_json::Number::from_f64(a)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            Value::Object(map)
        }
        "Table" if data.len() >= 20 => {
            let dims = i32::from_le_bytes(data[0..4].try_into().unwrap());
            let xsize = i32::from_le_bytes(data[4..8].try_into().unwrap());
            let ysize = i32::from_le_bytes(data[8..12].try_into().unwrap());
            let zsize = i32::from_le_bytes(data[12..16].try_into().unwrap());
            let len = i32::from_le_bytes(data[16..20].try_into().unwrap()).max(0) as usize;
            let mut values = Vec::with_capacity(len);
            let mut offset = 20;
            while offset + 2 <= data.len() && values.len() < len {
                let v = i16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
                values.push(Value::Number(v.into()));
                offset += 2;
            }
            let mut map = serde_json::Map::new();
            map.insert("__class".into(), Value::String("Table".into()));
            map.insert("dims".into(), Value::Number(dims.into()));
            map.insert("xsize".into(), Value::Number(xsize.into()));
            map.insert("ysize".into(), Value::Number(ysize.into()));
            map.insert("zsize".into(), Value::Number(zsize.into()));
            map.insert("data".into(), Value::Array(values));
            Value::Object(map)
        }
        _ => {
            let mut map = serde_json::Map::new();
            map.insert("__class".into(), Value::String(class_name.to_string()));
            let bytes: Vec<Value> = data.iter().map(|b| Value::Number((*b).into())).collect();
            map.insert("raw".into(), Value::Array(bytes));
            Value::Object(map)
        }
    }
}

impl fmt::Display for RubyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RubyValue::Nil => write!(f, "nil"),
            RubyValue::True => write!(f, "true"),
            RubyValue::False => write!(f, "false"),
            RubyValue::Integer(v) => write!(f, "{}", v),
            RubyValue::Float(v) => write!(f, "{}", v),
            RubyValue::String(s) => write!(f, "\"{}\"", s.to_string_lossy()),
            RubyValue::Symbol(sym) => write!(f, ":{}", sym),
            _ => write!(f, "{:?}", self),
        }
    }
}
