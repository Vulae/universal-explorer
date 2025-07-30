#![allow(unused)]
//! https://github.com/python/cpython/blob/main/Lib/pickle.py

use std::{cell::RefCell, collections::HashMap, io::Read, rc::Rc};
use util_general::ReadExt as _;

use super::{Class, Module, PickleError, Value};

/// Should be equivalent to `int.from_bytes(data, byteorder='little', signed=True)` in python
/// https://github.com/python/cpython/blob/800d37feca2e0ea3343995b3b817b653db2f9034/Lib/pickle.py#L356
/// https://github.com/python/cpython/blob/800d37feca2e0ea3343995b3b817b653db2f9034/Doc/library/stdtypes.rst?plain=1#L511
pub(crate) fn int_from_bytes(bytes: &[u8]) -> Result<i128, PickleError> {
    if bytes.len() > 16 {
        return Err(PickleError::CouldntConvertBigInt(
            bytes.to_vec().into_boxed_slice(),
        ));
    }

    if bytes.is_empty() {
        return Ok(0);
    }

    let mut padded_bytes = [0u8; 16];
    padded_bytes[0..bytes.len()].copy_from_slice(bytes);

    let is_negative = (bytes.last().unwrap() & 0x80) != 0;
    if is_negative {
        padded_bytes
            .iter_mut()
            .skip(bytes.len())
            .for_each(|byte| *byte = 0xFF);
    }

    Ok(i128::from_le_bytes(padded_bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub(super) enum Opcode {
    MARK,            // push special markobject on stack
    STOP,            // every pickle ends with STOP
    POP,             // discard topmost stack item
    POP_MARK,        // discard stack top through topmost markobject
    DUP,             // duplicate top stack item
    FLOAT,           // push float object; decimal string argument
    INT,             // push integer or bool; decimal string argument
    BININT,          // push four-byte signed int
    BININT1,         // push 1-byte unsigned int
    LONG,            // push long; decimal string argument
    BININT2,         // push 2-byte unsigned int
    NONE,            // push None
    PERSID,          // push persistent object; id is taken from string arg
    BINPERSID,       //"       "         "  ;  "  "   "     "  stack
    REDUCE,          // apply callable to argtuple, both on stack
    STRING,          // push string; NL-terminated string argument
    BINSTRING,       // push string; counted binary string argument
    SHORT_BINSTRING, //"     "   ;    "      "       "      " < 256 bytes
    UNICODE,         // push Unicode string; raw-unicode-escaped'd argument
    BINUNICODE,      // "     "       "  ; counted UTF-8 string argument
    APPEND,          // append stack top to list below it
    BUILD,           // call __setstate__ or __dict__.update()
    GLOBAL,          // push self.find_class(modname, name); 2 string args
    DICT,            // build a dict from stack items
    EMPTY_DICT,      // push empty dict
    APPENDS,         // extend list on stack by topmost stack slice
    GET,             // push item from memo on stack; index is string arg
    BINGET,          // "    "    "    "   "   "  ;   "    " 1-byte arg
    INST,            // build & push class instance
    LONG_BINGET,     // push item from memo on stack; index is 4-byte arg
    LIST,            // build list from topmost stack items
    EMPTY_LIST,      // push empty list
    OBJ,             // build & push class instance
    PUT,             // store stack top in memo; index is string arg
    BINPUT,          // "     "    "   "   " ;   "    " 1-byte arg
    LONG_BINPUT,     // "     "    "   "   " ;   "    " 4-byte arg
    SETITEM,         // add key+value pair to dict
    TUPLE,           // build tuple from topmost stack items
    EMPTY_TUPLE,     // push empty tuple
    SETITEMS,        // modify dict by adding topmost key+value pairs
    BINFLOAT,        // push float; arg is 8-byte float encoding
    //  Protocol 2
    PROTO,    // identify pickle protocol
    NEWOBJ,   // build object by applying cls.__new__ to argtuple
    EXT1,     // push object from extension registry; 1-byte index
    EXT2,     // ditto, but 2-byte index
    EXT4,     // ditto, but 4-byte index
    TUPLE1,   // build 1-tuple from stack top
    TUPLE2,   // build 2-tuple from two topmost stack items
    TUPLE3,   // build 3-tuple from three topmost stack items
    NEWTRUE,  // push True
    NEWFALSE, // push False
    LONG1,    // push long from < 256 bytes
    LONG4,    // push really big long
    //  Protocol 3 (Python 3.x)
    BINBYTES,       // push bytes; counted binary string argument
    SHORT_BINBYTES, //"     "   ;    "      "       "      " < 256 bytes
    //  Protocol 4
    SHORT_BINUNICODE, // push short string; UTF-8 length < 256 bytes
    BINUNICODE8,      // push very long string
    BINBYTES8,        // push very long bytes string
    EMPTY_SET,        // push empty set on the stack
    ADDITEMS,         // modify set by adding topmost stack items
    FROZENSET,        // build frozenset from topmost stack items
    NEWOBJ_EX,        // like NEWOBJ but work with keyword only arguments
    STACK_GLOBAL,     // same as GLOBAL but using names on the stacks
    MEMOIZE,          // store top of the stack in memo
    FRAME,            // indicate the beginning of a new frame
    //  Protocol 5
    BYTEARRAY8,      // push bytearray
    NEXT_BUFFER,     // push next out-of-band buffer
    READONLY_BUFFER, // make top of stack readonly
}

impl Opcode {
    fn from_byte(byte: u8) -> Result<Self, PickleError> {
        Ok(match byte {
            0x28 => Self::MARK,
            0x2E => Self::STOP,
            0x30 => Self::POP,
            0x31 => Self::POP_MARK,
            0x32 => Self::DUP,
            0x46 => Self::FLOAT,
            0x49 => Self::INT,
            0x4A => Self::BININT,
            0x4B => Self::BININT1,
            0x4C => Self::LONG,
            0x4D => Self::BININT2,
            0x4E => Self::NONE,
            0x50 => Self::PERSID,
            0x51 => Self::BINPERSID,
            0x52 => Self::REDUCE,
            0x53 => Self::STRING,
            0x54 => Self::BINSTRING,
            0x55 => Self::SHORT_BINSTRING,
            0x56 => Self::UNICODE,
            0x58 => Self::BINUNICODE,
            0x61 => Self::APPEND,
            0x62 => Self::BUILD,
            0x63 => Self::GLOBAL,
            0x64 => Self::DICT,
            0x7D => Self::EMPTY_DICT,
            0x65 => Self::APPENDS,
            0x67 => Self::GET,
            0x68 => Self::BINGET,
            0x69 => Self::INST,
            0x6A => Self::LONG_BINGET,
            0x6C => Self::LIST,
            0x5D => Self::EMPTY_LIST,
            0x6F => Self::OBJ,
            0x70 => Self::PUT,
            0x71 => Self::BINPUT,
            0x72 => Self::LONG_BINPUT,
            0x73 => Self::SETITEM,
            0x74 => Self::TUPLE,
            0x29 => Self::EMPTY_TUPLE,
            0x75 => Self::SETITEMS,
            0x47 => Self::BINFLOAT,
            0x80 => Self::PROTO,
            0x81 => Self::NEWOBJ,
            0x82 => Self::EXT1,
            0x83 => Self::EXT2,
            0x84 => Self::EXT4,
            0x85 => Self::TUPLE1,
            0x86 => Self::TUPLE2,
            0x87 => Self::TUPLE3,
            0x88 => Self::NEWTRUE,
            0x89 => Self::NEWFALSE,
            0x8a => Self::LONG1,
            0x8b => Self::LONG4,
            0x42 => Self::BINBYTES,
            0x43 => Self::SHORT_BINBYTES,
            0x8c => Self::SHORT_BINUNICODE,
            0x8d => Self::BINUNICODE8,
            0x8e => Self::BINBYTES8,
            0x8f => Self::EMPTY_SET,
            0x90 => Self::ADDITEMS,
            0x91 => Self::FROZENSET,
            0x92 => Self::NEWOBJ_EX,
            0x93 => Self::STACK_GLOBAL,
            0x94 => Self::MEMOIZE,
            0x95 => Self::FRAME,
            0x96 => Self::BYTEARRAY8,
            0x97 => Self::NEXT_BUFFER,
            0x98 => Self::READONLY_BUFFER,
            _ => return Err(PickleError::InvalidOpcode(byte)),
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum Protocol {
    #[default]
    Protocol1,
    Protocol2,
    Protocol3,
    Protocol4,
    Protocol5,
}

impl Protocol {
    fn from_byte(byte: u8) -> Result<Self, PickleError> {
        Ok(match byte {
            1 => Self::Protocol1,
            2 => Self::Protocol2,
            3 => Self::Protocol3,
            4 => Self::Protocol4,
            5 => Self::Protocol5,
            _ => return Err(PickleError::InvalidProtocol(byte)),
        })
    }
}

/// Same as pickle::Value but inner contents are SharedParseValue
#[derive(Debug, Clone)]
enum ParseValue {
    None,
    Bool(bool),
    Int(i128),
    Float(f64),
    String(String),
    Binary(Box<[u8]>),
    List(Vec<SharedParseValue>),
    Dict(HashMap<String, SharedParseValue>),
    Tuple(Box<[SharedParseValue]>),
    Module(ParseModule),
    Class(ParseClass),
}

impl ParseValue {
    fn to_cloned_dedupe(&self) -> Value {
        match self {
            ParseValue::None => Value::None,
            ParseValue::Bool(bool) => Value::Bool(*bool),
            ParseValue::Int(int) => Value::Int(*int),
            ParseValue::Float(float) => Value::Float(*float),
            ParseValue::String(string) => Value::String(string.clone()),
            ParseValue::Binary(binary) => Value::Binary(binary.clone()),
            ParseValue::List(list) => Value::List(
                list.iter()
                    .map(|item| item.borrow().to_cloned_dedupe())
                    .collect(),
            ),
            ParseValue::Dict(dict) => Value::Dict(
                dict.iter()
                    .map(|(key, value)| (key.clone(), value.borrow().to_cloned_dedupe()))
                    .collect(),
            ),
            ParseValue::Tuple(tuple) => Value::Tuple(
                tuple
                    .iter()
                    .map(|item| item.borrow().to_cloned_dedupe())
                    .collect(),
            ),
            ParseValue::Module(module) => Value::Module(Module {
                module: module.module.clone(),
                name: module.name.clone(),
            }),
            ParseValue::Class(class) => Value::Class(Box::new(Class {
                module: Module {
                    module: class.module.module.clone(),
                    name: class.module.name.clone(),
                },
                args: class.args.borrow().to_cloned_dedupe(),
                state: class.state.borrow().to_cloned_dedupe(),
                data: class
                    .data
                    .iter()
                    .map(|(key, value)| (key.clone(), value.borrow().to_cloned_dedupe()))
                    .collect(),
            })),
        }
    }
}

type SharedParseValue = Rc<RefCell<ParseValue>>;

#[derive(Debug, Clone)]
struct ParseModule {
    module: String,
    name: String,
}

impl ParseModule {
    fn new(module: String, name: String) -> Self {
        Self { module, name }
    }

    fn into_class<T: Into<SharedParseValue>>(self, args: T) -> ParseClass {
        ParseClass::new(self, args)
    }
}

#[derive(Debug, Clone)]
struct ParseClass {
    module: ParseModule,
    args: SharedParseValue,
    state: SharedParseValue,
    data: HashMap<String, SharedParseValue>,
}

impl ParseClass {
    fn new<T: Into<SharedParseValue>>(module: ParseModule, args: T) -> Self {
        Self {
            module,
            args: args.into(),
            state: ParseValue::None.into(),
            data: HashMap::new(),
        }
    }
}

impl From<ParseValue> for SharedParseValue {
    fn from(value: ParseValue) -> Self {
        Rc::new(RefCell::new(value))
    }
}

#[derive(Debug)]
struct Stack {
    segments: Vec<Vec<SharedParseValue>>,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            segments: vec![Vec::new()],
        }
    }
}

impl Stack {
    fn push<T: Into<SharedParseValue>>(&mut self, value: T) -> Result<(), PickleError> {
        self.segments
            .last_mut()
            .ok_or(PickleError::MalformedStack("No stack segments"))?
            .push(value.into());
        Ok(())
    }

    fn pop(&mut self) -> Result<SharedParseValue, PickleError> {
        self.segments
            .last_mut()
            .ok_or(PickleError::MalformedStack("No stack segments"))?
            .pop()
            .ok_or(PickleError::MalformedStack(
                "Cannot pop from empty stack segment",
            ))
    }

    fn last(&self) -> Result<&SharedParseValue, PickleError> {
        self.segments
            .last()
            .ok_or(PickleError::MalformedStack("No stack segments"))?
            .last()
            .ok_or(PickleError::MalformedStack(
                "Cannot get last item from empty stack segment",
            ))
    }

    fn push_mark(&mut self) {
        self.segments.push(Vec::new());
    }

    fn pop_mark(&mut self) -> Result<Vec<SharedParseValue>, PickleError> {
        self.segments
            .pop()
            .ok_or(PickleError::MalformedStack("No stack segments"))
    }
}

#[derive(Debug, Default)]
struct Memo {
    items: HashMap<u64, SharedParseValue>,
    max_index: u64,
}

impl Memo {
    fn get(&self, index: u64) -> Result<&SharedParseValue, PickleError> {
        self.items.get(&index).ok_or(PickleError::NoMemoItem)
    }

    fn set<T: Into<SharedParseValue>>(&mut self, index: u64, value: T) {
        if self.items.contains_key(&index) {
            log::warn!("Pickle memo overwritting item that already exists");
        }
        self.items.insert(index, value.into());
        self.max_index = self.max_index.max(index);
    }

    pub fn push<T: Into<SharedParseValue>>(&mut self, value: T) {
        self.max_index += 1;
        self.items.insert(self.max_index, value.into());
    }
}

#[derive(Debug, Default)]
struct State {
    protocol: Protocol,
    stack: Stack,
    memo: Memo,
}

impl State {
    fn read<R: Read>(&mut self, opcode: Opcode, mut reader: R) -> Result<(), PickleError> {
        match opcode {
            Opcode::MARK => {
                self.stack.push_mark();
            }
            Opcode::STOP => unreachable!(),
            // Opcode::POP => todo!(),
            // Opcode::POP_MARK => todo!(),
            // Opcode::DUP => todo!(),
            // Opcode::FLOAT => todo!(),
            // Opcode::INT => todo!(),
            Opcode::BININT => {
                let int = i32::from_le_bytes(reader.read_const()?);
                self.stack.push(ParseValue::Int(int as i128))?;
            }
            // Opcode::BININT1 => todo!(),
            // Opcode::LONG => todo!(),
            // Opcode::BININT2 => todo!(),
            // Opcode::NONE => todo!(),
            // Opcode::PERSID => todo!(),
            // Opcode::BINPERSID => todo!(),
            // Opcode::REDUCE => todo!(),
            // Opcode::STRING => todo!(),
            // Opcode::BINSTRING => todo!(),
            // Opcode::SHORT_BINSTRING => todo!(),
            // Opcode::UNICODE => todo!(),
            // Opcode::BINUNICODE => todo!(),
            Opcode::APPEND => {
                let item = self.stack.pop()?;
                if let ParseValue::List(list) = &mut *self.stack.last()?.borrow_mut() {
                    list.push(item);
                } else {
                    return Err(PickleError::OpcodeError(
                        opcode,
                        "Last stack item must be a list",
                    ));
                }
            }
            // Opcode::BUILD => todo!(),
            // Opcode::GLOBAL => todo!(),
            // Opcode::DICT => todo!(),
            Opcode::EMPTY_DICT => {
                self.stack.push(ParseValue::Dict(HashMap::new()))?;
            }
            // Opcode::APPENDS => todo!(),
            // Opcode::GET => todo!(),
            Opcode::BINGET => {
                let index = u8::from_le_bytes(reader.read_const()?);
                let item = Rc::clone(self.memo.get(index as u64)?);
                self.stack.push(item)?;
            }
            // Opcode::INST => todo!(),
            // Opcode::LONG_BINGET => todo!(),
            // Opcode::LIST => todo!(),
            Opcode::EMPTY_LIST => {
                self.stack.push(ParseValue::List(Vec::new()))?;
            }
            // Opcode::OBJ => todo!(),
            // Opcode::PUT => todo!(),
            // Opcode::BINPUT => todo!(),
            // Opcode::LONG_BINPUT => todo!(),
            // Opcode::SETITEM => todo!(),
            // Opcode::TUPLE => todo!(),
            // Opcode::EMPTY_TUPLE => todo!(),
            Opcode::SETITEMS => {
                let items = self.stack.pop_mark()?;
                let mut binding = self.stack.last()?.borrow_mut();
                let dict = match &mut *binding {
                    ParseValue::Dict(dict) => dict,
                    ParseValue::Class(class) => &mut class.data,
                    _ => {
                        return Err(PickleError::OpcodeError(
                            opcode,
                            "Last stack item must be a dict or class",
                        ));
                    }
                };
                let (chunks, extra) = items.as_chunks::<2>();
                if !extra.is_empty() {
                    return Err(PickleError::OpcodeError(opcode, "Invalid items"));
                }
                for [key, value] in chunks {
                    let key = if let ParseValue::String(string) = &*key.borrow() {
                        string.to_owned()
                    } else {
                        return Err(PickleError::OpcodeError(
                            opcode,
                            "Expected key for dict to be a string",
                        ));
                    };
                    dict.insert(key, Rc::clone(value));
                }
            }
            // Opcode::BINFLOAT => todo!(),
            Opcode::PROTO => unreachable!(),
            // Opcode::NEWOBJ => todo!(),
            // Opcode::EXT1 => todo!(),
            // Opcode::EXT2 => todo!(),
            // Opcode::EXT4 => todo!(),
            // Opcode::TUPLE1 => todo!(),
            // Opcode::TUPLE2 => todo!(),
            Opcode::TUPLE3 => {
                let mut items = vec![self.stack.pop()?, self.stack.pop()?, self.stack.pop()?];
                items.reverse();
                self.stack
                    .push(ParseValue::Tuple(items.into_boxed_slice()))?;
            }
            // Opcode::NEWTRUE => todo!(),
            // Opcode::NEWFALSE => todo!(),
            Opcode::LONG1 => {
                let length = u8::from_le_bytes(reader.read_const()?);
                let bytes = reader.read_var(length as usize)?;
                self.stack.push(ParseValue::Int(int_from_bytes(&bytes)?))?;
            }
            // Opcode::LONG4 => todo!(),
            // Opcode::BINBYTES => todo!(),
            Opcode::SHORT_BINBYTES => {
                let len = u8::from_le_bytes(reader.read_const()?);
                let bytes = reader.read_var(len as usize)?;
                self.stack.push(ParseValue::Binary(bytes))?;
            }
            Opcode::SHORT_BINUNICODE => {
                let len = u8::from_le_bytes(reader.read_const()?);
                let str = String::from_utf8(reader.read_var(len as usize)?.into_vec())?;
                self.stack.push(ParseValue::String(str))?;
            }
            // Opcode::BINUNICODE8 => todo!(),
            // Opcode::BINBYTES8 => todo!(),
            // Opcode::EMPTY_SET => todo!(),
            // Opcode::ADDITEMS => todo!(),
            // Opcode::FROZENSET => todo!(),
            // Opcode::NEWOBJ_EX => todo!(),
            // Opcode::STACK_GLOBAL => todo!(),
            Opcode::MEMOIZE => {
                self.memo.push(Rc::clone(self.stack.last()?));
            }
            Opcode::FRAME => {
                // TODO: Preload amount of bytes requested.
                let _preload_bytes = u64::from_le_bytes(reader.read_const()?);
            }
            // Opcode::BYTEARRAY8 => todo!(),
            // Opcode::NEXT_BUFFER => todo!(),
            // Opcode::READONLY_BUFFER => todo!(),
            _ => todo!("Pickle unimplemented opcode: {opcode:?}"),
        }
        Ok(())
    }
}

pub fn read_pickle<R: Read>(mut reader: R) -> Result<Value, PickleError> {
    let mut state = State::default();

    let mut first = true;
    loop {
        let opcode = Opcode::from_byte(u8::from_le_bytes(reader.read_const()?))?;

        match opcode {
            Opcode::PROTO => {
                if !first {
                    return Err(PickleError::OpcodeProtocolInvalidPosition);
                }
                state.protocol = Protocol::from_byte(u8::from_le_bytes(reader.read_const()?))?;
            }
            Opcode::STOP => return Ok(state.stack.pop()?.borrow().to_cloned_dedupe()),
            _ => state.read(opcode, &mut reader)?,
        }

        first = true;
    }
}

#[cfg(test)]
mod test {
    use crate::pickle::{reader::int_from_bytes, PickleError};

    #[test]
    fn bigint_convert_test() -> Result<(), PickleError> {
        assert_eq!(0i128, int_from_bytes(&[])?);
        assert_eq!(255i128, int_from_bytes(&[0xFF, 0x00])?);
        assert_eq!(32767i128, int_from_bytes(&[0xFF, 0x7F])?);
        assert_eq!(-256i128, int_from_bytes(&[0x00, 0xFF])?);
        assert_eq!(-32768i128, int_from_bytes(&[0x00, 0x80])?);
        assert_eq!(-128i128, int_from_bytes(&[0x80])?);
        assert_eq!(127i128, int_from_bytes(&[0x7F])?);
        Ok(())
    }
}
