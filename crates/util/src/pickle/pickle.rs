use anyhow::{anyhow, Result};
use std::{collections::HashMap, io::Read, iter::FromIterator};

use super::parser;

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float(f64),
    BigInt(Vec<u8>),
    String(String),
    Binary(Vec<u8>),
    List(Vec<Value>),
    Dict(HashMap<String, Value>),
    Tuple(Vec<Value>),
    Module(Module),
    Class(Class),
}

fn bigint_to_u64(bigint: &[u8]) -> Result<u64> {
    if bigint.len() > 8 {
        return Err(anyhow!("Cannot parse a bigint bigger than 8 bytes"));
    }
    let mut v: u64 = 0;
    for byte in bigint.iter().rev() {
        v <<= 8;
        v |= *byte as u64;
    }
    Ok(v)
}

impl Value {
    pub fn from_binary(mut data: impl Read, is_compressed: bool) -> Result<Value> {
        if !is_compressed {
            parser::Parser::parse(&mut data)
        } else {
            let mut decoded = Vec::new();
            flate2::read::ZlibDecoder::new(data).read_to_end(&mut decoded)?;
            parser::Parser::parse(&mut std::io::Cursor::new(decoded))
        }
    }

    // TODO: Actual serializer instead of just converting to JSON.
    pub fn to_json(self) -> serde_json::Value {
        match self {
            Value::None => serde_json::Value::Null,
            Value::Bool(bool) => serde_json::Value::Bool(bool),
            Value::Int(int) => serde_json::Value::Number(serde_json::Number::from(int)),
            Value::Uint(uint) => serde_json::Value::Number(serde_json::Number::from(uint)),
            Value::Float(float) => {
                serde_json::Value::Number(serde_json::Number::from_f64(float).unwrap())
            }
            Value::BigInt(bigint) => {
                serde_json::Value::Number(serde_json::Number::from(bigint_to_u64(&bigint).unwrap()))
            }
            Value::String(string) => serde_json::Value::String(string),
            Value::Binary(binary) => serde_json::Value::Array(
                binary
                    .into_iter()
                    .map(|v| serde_json::Value::Number(serde_json::Number::from(v)))
                    .collect::<Vec<_>>(),
            ),
            Value::List(list) => {
                serde_json::Value::Array(list.into_iter().map(|v| v.to_json()).collect::<Vec<_>>())
            }
            Value::Dict(dict) => serde_json::Value::Object(serde_json::Map::from_iter(
                dict.into_iter().map(|(k, v)| (k, v.to_json())),
            )),
            Value::Tuple(tuple) => {
                serde_json::Value::Array(tuple.into_iter().map(|v| v.to_json()).collect::<Vec<_>>())
            }
            Value::Module(_) => todo!(),
            Value::Class(_) => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub module: String,
    pub name: String,
}

impl Module {
    pub fn new(module: String, name: String) -> Self {
        Self { module, name }
    }

    pub fn to_class(self, args: Value) -> Class {
        Class::new(self, args)
    }
}

#[derive(Debug, Clone)]
pub struct Class {
    pub module: Module,
    pub args: Box<Value>,
    pub state: Option<Box<Value>>,
    pub data: HashMap<String, Value>,
}

impl Class {
    pub fn new(module: Module, args: Value) -> Self {
        Self {
            module,
            args: Box::new(args),
            state: None,
            data: HashMap::new(),
        }
    }
}
