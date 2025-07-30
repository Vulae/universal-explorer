use std::{collections::HashMap, io::Read};

use de::PickleValueDeserializer;
use reader::Opcode;
use thiserror::Error;

mod de;
mod reader;

#[derive(Debug, Error)]
pub enum PickleError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(u8),
    #[error("Invalid protocol: {0}")]
    InvalidProtocol(u8),
    #[error("Opcode definining pickle protocol must either be not present or the first opcode")]
    OpcodeProtocolInvalidPosition,
    #[error("Stack is malformed: {0}")]
    MalformedStack(&'static str),
    #[error("Failed to get memo item")]
    NoMemoItem,
    #[error("Opcode {0:?} error: {1}")]
    #[allow(private_interfaces)]
    OpcodeError(Opcode, &'static str),
    #[error("Couldn't convert bigint: {0:?}")]
    CouldntConvertBigInt(Box<[u8]>),
    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Serde deserialize error: {0}")]
    SerdeDeserializeError(String),
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
    pub args: Value,
    pub state: Value,
    pub data: HashMap<String, Value>,
}

impl Class {
    pub fn new(module: Module, args: Value) -> Self {
        Self {
            module,
            args,
            state: Value::None,
            data: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(i128),
    Float(f64),
    String(String),
    Binary(Box<[u8]>),
    List(Vec<Value>),
    Dict(HashMap<String, Value>),
    Tuple(Box<[Value]>),
    Module(Module),
    Class(Box<Class>),
}

impl Value {
    pub fn from_binary<R: Read>(data: R, compressed: bool) -> Result<Self, PickleError> {
        if !compressed {
            reader::read_pickle(data)
        } else {
            reader::read_pickle(flate2::read::ZlibDecoder::new(data))
        }
    }
}

pub fn from_pickle<T>(value: Value) -> Result<T, PickleError>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(PickleValueDeserializer(value))
}
