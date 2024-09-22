#![allow(unused)]

use super::pickle::Value;
use anyhow::{anyhow, Error, Result};
use std::{collections::HashMap, convert::TryFrom, io::Read};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
#[allow(non_camel_case_types)]
enum Opcode {
    MARK = 0x28,            // push special markobject on stack
    STOP = 0x2E,            // every pickle ends with STOP
    POP = 0x30,             // discard topmost stack item
    POP_MARK = 0x31,        // discard stack top through topmost markobject
    DUP = 0x32,             // duplicate top stack item
    FLOAT = 0x46,           // push float object; decimal string argument
    INT = 0x49,             // push integer or bool; decimal string argument
    BININT = 0x4A,          // push four-byte signed int
    BININT1 = 0x4B,         // push 1-byte unsigned int
    LONG = 0x4C,            // push long; decimal string argument
    BININT2 = 0x4D,         // push 2-byte unsigned int
    NONE = 0x4E,            // push None
    PERSID = 0x50,          // push persistent object; id is taken from string arg
    BINPERSID = 0x51,       //"       "         "  ;  "  "   "     "  stack
    REDUCE = 0x52,          // apply callable to argtuple, both on stack
    STRING = 0x53,          // push string; NL-terminated string argument
    BINSTRING = 0x54,       // push string; counted binary string argument
    SHORT_BINSTRING = 0x55, //"     "   ;    "      "       "      " < 256 bytes
    UNICODE = 0x56,         // push Unicode string; raw-unicode-escaped'd argument
    BINUNICODE = 0x58,      // "     "       "  ; counted UTF-8 string argument
    APPEND = 0x61,          // append stack top to list below it
    BUILD = 0x62,           // call __setstate__ or __dict__.update()
    GLOBAL = 0x63,          // push self.find_class(modname, name); 2 string args
    DICT = 0x64,            // build a dict from stack items
    EMPTY_DICT = 0x7D,      // push empty dict
    APPENDS = 0x65,         // extend list on stack by topmost stack slice
    GET = 0x67,             // push item from memo on stack; index is string arg
    BINGET = 0x68,          // "    "    "    "   "   "  ;   "    " 1-byte arg
    INST = 0x69,            // build & push class instance
    LONG_BINGET = 0x6A,     // push item from memo on stack; index is 4-byte arg
    LIST = 0x6C,            // build list from topmost stack items
    EMPTY_LIST = 0x5D,      // push empty list
    OBJ = 0x6F,             // build & push class instance
    PUT = 0x70,             // store stack top in memo; index is string arg
    BINPUT = 0x71,          // "     "    "   "   " ;   "    " 1-byte arg
    LONG_BINPUT = 0x72,     // "     "    "   "   " ;   "    " 4-byte arg
    SETITEM = 0x73,         // add key+value pair to dict
    TUPLE = 0x74,           // build tuple from topmost stack items
    EMPTY_TUPLE = 0x29,     // push empty tuple
    SETITEMS = 0x75,        // modify dict by adding topmost key+value pairs
    BINFLOAT = 0x47,        // push float; arg is 8-byte float encoding
    //  Protocol 2
    PROTO = 0x80,    // identify pickle protocol
    NEWOBJ = 0x81,   // build object by applying cls.__new__ to argtuple
    EXT1 = 0x82,     // push object from extension registry; 1-byte index
    EXT2 = 0x83,     // ditto, but 2-byte index
    EXT4 = 0x84,     // ditto, but 4-byte index
    TUPLE1 = 0x85,   // build 1-tuple from stack top
    TUPLE2 = 0x86,   // build 2-tuple from two topmost stack items
    TUPLE3 = 0x87,   // build 3-tuple from three topmost stack items
    NEWTRUE = 0x88,  // push True
    NEWFALSE = 0x89, // push False
    LONG1 = 0x8a,    // push long from < 256 bytes
    LONG4 = 0x8b,    // push really big long
    //  Protocol 3 (Python 3.x)
    BINBYTES = 0x42,       // push bytes; counted binary string argument
    SHORT_BINBYTES = 0x43, //"     "   ;    "      "       "      " < 256 bytes
    //  Protocol 4
    SHORT_BINUNICODE = 0x8c, // push short string; UTF-8 length < 256 bytes
    BINUNICODE8 = 0x8d,      // push very long string
    BINBYTES8 = 0x8e,        // push very long bytes string
    EMPTY_SET = 0x8f,        // push empty set on the stack
    ADDITEMS = 0x90,         // modify set by adding topmost stack items
    FROZENSET = 0x91,        // build frozenset from topmost stack items
    NEWOBJ_EX = 0x92,        // like NEWOBJ but work with keyword only arguments
    STACK_GLOBAL = 0x93,     // same as GLOBAL but using names on the stacks
    MEMOIZE = 0x94,          // store top of the stack in memo
    FRAME = 0x95,            // indicate the beginning of a new frame
    //  Protocol 5
    BYTEARRAY8 = 0x96,      // push bytearray
    NEXT_BUFFER = 0x97,     // push next out-of-band buffer
    READONLY_BUFFER = 0x98, // make top of stack readonly
}

impl Opcode {
    fn try_from(value: u8) -> Result<Opcode> {
        // TODO: Make safe.
        Ok(unsafe { std::mem::transmute(value) })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Protocol {
    Protocol1,
    Protocol2,
    Protocol3,
    Protocol4,
    Protocol5,
}

impl TryFrom<u8> for Protocol {
    type Error = Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(Protocol::Protocol1),
            2 => Ok(Protocol::Protocol2),
            3 => Ok(Protocol::Protocol3),
            4 => Ok(Protocol::Protocol4),
            5 => Ok(Protocol::Protocol5),
            _ => Err(anyhow!("Invalid protocol")),
        }
    }
}

#[derive(Debug, Clone)]
enum StackItem {
    Value(Value),
    Mark,
}

#[derive(Debug)]
struct Stack {
    stack: Vec<StackItem>,
}

impl Stack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, value: Value) {
        self.stack.push(StackItem::Value(value));
    }

    pub fn pop(&mut self) -> Result<Value> {
        match self.stack.pop() {
            Some(StackItem::Value(value)) => Ok(value),
            Some(StackItem::Mark) => Err(anyhow!("Cannot pop StackItem::Mark off the stack")),
            None => Err(anyhow!("Cannot pop from an empty stack")),
        }
    }

    pub fn last(&self) -> Result<&Value> {
        match self.stack.last() {
            Some(StackItem::Value(value)) => Ok(value),
            Some(StackItem::Mark) => {
                Err(anyhow!("Cannot get StackItem::Mark as last item in stack"))
            }
            None => Err(anyhow!("Cannot get last item in empty stack")),
        }
    }

    pub fn last_mut(&mut self) -> Result<&mut Value> {
        match self.stack.last_mut() {
            Some(StackItem::Value(value)) => Ok(value),
            Some(StackItem::Mark) => {
                Err(anyhow!("Cannot get StackItem::Mark as last item in stack"))
            }
            None => Err(anyhow!("Cannot get last item in empty stack")),
        }
    }

    pub fn set_last(&mut self, value: Value) -> Result<Value> {
        let removed = self.pop()?;
        self.push(value);
        Ok(removed)
    }

    pub fn push_mark(&mut self) {
        self.stack.push(StackItem::Mark);
    }

    pub fn pop_mark(&mut self) -> Result<Vec<Value>> {
        let mut values = Vec::new();

        loop {
            match self.stack.pop() {
                Some(StackItem::Value(value)) => values.push(value),
                Some(StackItem::Mark) => break,
                None => return Err(anyhow!("Pop mark emptied stack without finding mark")),
            }
        }

        Ok(values)
    }
}

#[derive(Debug, Clone)]
enum MemoItem {
    Value(Value),
    Empty,
}

#[derive(Debug)]
struct Memo {
    items: Vec<MemoItem>,
}

impl Memo {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn get(&self, index: usize) -> Result<&Value> {
        match self.items.get(index) {
            Some(MemoItem::Value(value)) => Ok(value),
            Some(MemoItem::Empty) => Err(anyhow!("Cannot get empty memo value")),
            None => Err(anyhow!("Cannot get memo out of bounds index")),
        }
    }

    pub fn set(&mut self, index: usize, value: Value) {
        while index >= self.items.len() {
            self.items.push(MemoItem::Empty);
        }

        self.items[index] = MemoItem::Value(value);
    }

    pub fn push(&mut self, value: Value) {
        self.items.push(MemoItem::Value(value))
    }

    pub fn last(&self) -> Result<&Value> {
        match self.items.last() {
            Some(MemoItem::Value(value)) => Ok(value),
            Some(MemoItem::Empty) => Err(anyhow!("Cannot get empty memo value")),
            None => Err(anyhow!("Cannot get last item on empty memo")),
        }
    }
}

#[derive(Debug)]
pub struct Parser {
    protocol: Option<Protocol>,
    stack: Stack,
    memo: Memo,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            protocol: None,
            stack: Stack::new(),
            memo: Memo::new(),
        }
    }

    pub fn is_protocol_supported(&self) -> bool {
        match self.protocol {
            Some(Protocol::Protocol1) => false,
            Some(Protocol::Protocol2) => true,
            Some(Protocol::Protocol3) => true,
            Some(Protocol::Protocol4) => true,
            Some(Protocol::Protocol5) => true,
            None => false,
        }
    }

    fn read_operation(&mut self, data: &mut impl Read) -> Result<Opcode> {
        let mut reader = crate::reader::Reader::new_le(data);

        let opcode = Opcode::try_from(reader.read::<u8>()?)?;

        if self.protocol.is_none() {
            if opcode == Opcode::PROTO {
                self.protocol = Some(Protocol::try_from(reader.read::<u8>()?)?);
                if !self.is_protocol_supported() {
                    return Err(anyhow!("Unsupported protocol"));
                }
                return Ok(opcode);
            } else {
                return Err(anyhow!("First opcode MUST be Opcode::PROTO"));
            }
        }

        // FIXME: Refactor so each Value is a Rc<RefCell<Value>> or something like that.
        // Because each time we clone a value, it is supposed to be a reference to that value.

        match opcode {
            Opcode::PROTO => return Err(anyhow!("Invalid Opcode::PROTO operation")),
            Opcode::STOP => {}
            Opcode::FRAME => {
                reader.read::<u64>()?;
            } // A hint for how many bytes to read in the pickle object.
            Opcode::EMPTY_DICT => self.stack.push(Value::Dict(HashMap::new())),
            Opcode::BINPUT => self
                .memo
                .set(reader.read::<u8>()? as usize, self.stack.last()?.clone()),
            Opcode::MARK => self.stack.push_mark(),
            Opcode::BINUNICODE => self
                .stack
                .push(Value::String(reader.read_length_string::<u32>()?)),
            Opcode::EMPTY_LIST => self.stack.push(Value::List(Vec::new())),
            Opcode::LONG1 => {
                let length = reader.read::<u8>()?;
                let bytes = reader.read_buf(length as usize)?;
                self.stack.push(Value::BigInt(bytes));
            }
            Opcode::BININT => self.stack.push(Value::Int(reader.read::<i32>()? as i64)),
            Opcode::SHORT_BINSTRING => self
                .stack
                .push(Value::String(reader.read_length_string::<u8>()?)),
            Opcode::TUPLE3 => {
                let mut items = vec![self.stack.pop()?, self.stack.pop()?, self.stack.pop()?];
                items.reverse();
                self.stack.push(Value::Tuple(items));
            }
            Opcode::APPEND => {
                let item = self.stack.pop()?;
                match self.stack.last_mut()? {
                    Value::List(list) => list.push(item),
                    _ => {
                        return Err(anyhow!(
                            "Cannot Opcode::APPEND onto value that is not a list"
                        ))
                    }
                }
            }
            Opcode::BINGET => {
                let index = reader.read::<u8>()?;
                let item = self.memo.get(index as usize)?;
                self.stack.push(item.clone());
            }
            Opcode::LONG_BINPUT => self
                .memo
                .set(reader.read::<u32>()? as usize, self.stack.last()?.clone()),
            Opcode::SETITEMS => {
                let mut items = self.stack.pop_mark()?;
                let dict = match self.stack.last_mut()? {
                    Value::Dict(dict) => dict,
                    Value::Class(class) => &mut class.data,
                    _ => {
                        return Err(anyhow!(
                            "Cannot Opcode::SETITEMS onto value that is not dict or class"
                        ))
                    }
                };
                while items.len() > 0 {
                    let key = match items.pop() {
                        Some(Value::String(string)) => string,
                        Some(_) => {
                            return Err(anyhow!(
                                "Opcode::SETITEMS key item is expected to be Value::String"
                            ))
                        }
                        None => return Err(anyhow!("Opcode::SETITEMS failed to get key")),
                    };
                    let value = items
                        .pop()
                        .ok_or(anyhow!("Opcode::SETITEMS failed to get value"))?;
                    dict.insert(key, value);
                }
            }
            Opcode::MEMOIZE => self.memo.push(self.stack.last()?.clone()),
            Opcode::SHORT_BINUNICODE => self
                .stack
                .push(Value::String(reader.read_length_string::<u8>()?)),
            Opcode::SHORT_BINBYTES => {
                let length = reader.read::<u8>()?;
                self.stack
                    .push(Value::Binary(reader.read_buf(length as usize)?));
            }
            _ => return Err(anyhow!("Unimplemented opcode {:?}", opcode)),
        }

        Ok(opcode)
    }

    pub fn parse(data: &mut impl Read) -> Result<Value> {
        let mut parser = Parser::new();
        loop {
            match parser.read_operation(data) {
                Ok(opcode) => {
                    if opcode == Opcode::STOP {
                        break;
                    }
                }
                Err(err) => {
                    // println!("{:?}", &parser);
                    return Err(err);
                }
            }
        }
        Ok(parser.stack.pop()?)
    }
}
