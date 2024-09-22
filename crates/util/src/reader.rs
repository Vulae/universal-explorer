use anyhow::{anyhow, Result};
use std::{
    convert::TryInto,
    io::{Read, Seek, SeekFrom},
};

pub trait Primitive: Sized {
    fn read_from_le<R: Read>(reader: &mut Reader<R>) -> Result<Self>;
    fn read_from_be<R: Read>(reader: &mut Reader<R>) -> Result<Self>;
}

macro_rules! impl_primitive_number {
    ($type:ty) => {
        impl Primitive for $type {
            fn read_from_le<R: Read>(reader: &mut Reader<R>) -> Result<Self> {
                let mut buf = [0; std::mem::size_of::<Self>()];
                reader.data.read_exact(&mut buf)?;
                Ok(Self::from_le_bytes(buf))
            }

            fn read_from_be<R: Read>(reader: &mut Reader<R>) -> Result<Self> {
                let mut buf = [0; std::mem::size_of::<Self>()];
                reader.data.read_exact(&mut buf)?;
                Ok(Self::from_be_bytes(buf))
            }
        }

        impl<const N: usize> Primitive for [$type; N] {
            fn read_from_le<R: Read>(reader: &mut Reader<R>) -> Result<Self> {
                let mut vals: [$type; N] = [<$type>::default(); N];
                for i in 0..vals.len() {
                    vals[i] = reader.read_le::<$type>()?;
                }
                Ok(vals)
            }

            fn read_from_be<R: Read>(reader: &mut Reader<R>) -> Result<Self> {
                let mut vals: [$type; N] = [<$type>::default(); N];
                for i in 0..vals.len() {
                    vals[i] = reader.read_be::<$type>()?;
                }
                Ok(vals)
            }
        }
    };
}

impl_primitive_number!(u8);
impl_primitive_number!(u16);
impl_primitive_number!(u32);
impl_primitive_number!(u64);
impl_primitive_number!(u128);
impl_primitive_number!(i8);
impl_primitive_number!(i16);
impl_primitive_number!(i32);
impl_primitive_number!(i64);
impl_primitive_number!(i128);
impl_primitive_number!(f32);
impl_primitive_number!(f64);

pub enum Endianness {
    LittleEndian,
    BigEndian,
}

pub struct Reader<T: Read> {
    pub data: T,
    pub endianness: Endianness,
}

impl<T: Read> Reader<T> {
    pub fn new(data: T, endianness: Endianness) -> Reader<T> {
        Reader { data, endianness }
    }

    pub fn new_le(data: T) -> Reader<T> {
        Reader::new(data, Endianness::LittleEndian)
    }

    pub fn new_be(data: T) -> Reader<T> {
        Reader::new(data, Endianness::BigEndian)
    }

    pub fn read_le<P: Primitive>(&mut self) -> Result<P> {
        P::read_from_le(self)
    }

    pub fn read_be<P: Primitive>(&mut self) -> Result<P> {
        P::read_from_be(self)
    }

    pub fn read<P: Primitive>(&mut self) -> Result<P> {
        match self.endianness {
            Endianness::LittleEndian => self.read_le(),
            Endianness::BigEndian => self.read_be(),
        }
    }

    pub fn read_vec_le<P: Primitive>(&mut self, length: usize) -> Result<Vec<P>> {
        let mut vec: Vec<P> = Vec::new();
        for _ in 0..length {
            vec.push(self.read_le::<P>()?);
        }
        Ok(vec)
    }

    pub fn read_vec_be<P: Primitive>(&mut self, length: usize) -> Result<Vec<P>> {
        let mut vec: Vec<P> = Vec::new();
        for _ in 0..length {
            vec.push(self.read_be::<P>()?);
        }
        Ok(vec)
    }

    pub fn read_vec<P: Primitive>(&mut self, length: usize) -> Result<Vec<P>> {
        match self.endianness {
            Endianness::LittleEndian => self.read_vec_le(length),
            Endianness::BigEndian => self.read_vec_be(length),
        }
    }

    pub fn read_buf(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];
        self.data.read(&mut buf)?;
        Ok(buf)
    }

    /// If length is not provided, will keep reading until null character "\0"
    pub fn read_string(&mut self, length: Option<usize>) -> Result<String> {
        let mut vec = Vec::new();
        match length {
            Some(length) => vec.append(&mut self.read_vec_le::<u8>(length)?),
            None => loop {
                let byte = self.read_le::<u8>()?;
                if byte == 0x00 {
                    break;
                }
                vec.push(byte);
            },
        }
        Ok(String::from_utf8(vec)?)
    }

    pub fn read_length_string<P: Primitive + TryInto<usize>>(&mut self) -> Result<String> {
        let length: usize = self
            .read::<P>()?
            .try_into()
            .map_err(|_| anyhow!("Could not convert primitive to usize"))?;
        Ok(self.read_string(Some(length))?)
    }

    // TODO: Don't do it this way.
    pub fn skip(&mut self, length: u64) -> Result<()> {
        for _ in 0..length {
            self.read_le::<u8>()?;
        }
        Ok(())
    }
}

impl<T: Read + Seek> Reader<T> {
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        Ok(self.data.seek(pos)?)
    }

    pub fn rewind(&mut self) -> Result<()> {
        Ok(self.data.rewind()?)
    }

    pub fn position(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    pub fn size(&mut self) -> Result<u64> {
        let position = self.position()?;
        let size = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(position))?;
        Ok(size)
    }

    pub fn bytes_remaining(&mut self) -> Result<u64> {
        let position = self.position()?;
        let size = self.size()?;
        Ok(size - position)
    }
}
