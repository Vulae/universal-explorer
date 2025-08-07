use std::io::{Read, Seek};

use crate::{LzmaDecodeOptions, LzmaDecoder, LzmaState};

// TODO: Make better, every like 50MiB of read data, we store a checkpoint that contains the
// internal state of the LZMA decoder, so if we want to seek somewhere we go to the nearest
// checkpoint then decode until we hit where we want to seek to.

// For now, every time we seek to a previous part of the stream, we reset the decoder.

#[derive(Debug)]
pub struct SeekableLZMADecoder<R: Read + Seek> {
    inner: R,
    options: LzmaDecodeOptions,
    state: LzmaState,
    position: u64,
    uncompressed_size: u64,
}

impl<R: Read + Seek> SeekableLZMADecoder<R> {
    pub fn new(inner: R, uncompressed_size: u64, options: LzmaDecodeOptions) -> Self {
        Self {
            inner,
            options,
            state: LzmaState::new(options),
            position: 0,
            uncompressed_size,
        }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.inner.rewind()?;
        self.state = LzmaState::new(self.options);
        self.position = 0;
        Ok(())
    }
}

impl<R: Read + Seek> Read for SeekableLZMADecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amount = LzmaDecoder::new(&mut self.inner, &mut self.state).read(buf)?;
        self.position += amount as u64;
        Ok(amount)
    }
}

impl<R: Read + Seek> Seek for SeekableLZMADecoder<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let seek_pos = match pos {
            std::io::SeekFrom::Start(start_offset) => start_offset,
            std::io::SeekFrom::End(end_offset) => self
                .uncompressed_size
                .checked_add_signed(end_offset)
                .ok_or(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Cannot seek behind the start of the file",
                ))?,
            std::io::SeekFrom::Current(offset) => {
                self.position
                    .checked_add_signed(offset)
                    .ok_or(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Cannot seek behind the start of the file",
                    ))?
            }
        };

        if seek_pos < self.position {
            self.reset()?;
        }

        const BUF_SIZE: usize = 1024;
        while self.position < seek_pos {
            let mut buf = [0u8; BUF_SIZE];
            let target_amount = ((seek_pos as usize) - (self.position as usize)).min(BUF_SIZE);
            let read_amount = self.read(&mut buf[0..target_amount])?;
            if read_amount == 0 {
                break;
            }
        }

        Ok(self.position)
    }
}
