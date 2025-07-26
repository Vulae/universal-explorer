use std::io::{Read, Seek};

// TODO: Make better, every like 10MiB of read data, we store a checkpoint that contains the
// internal state of the deflate decoder, so if we want to seek somewhere we go to the nearest
// checkpoint then decode until we hit where we want to seek to.

// For now, every time we seek to a previous part of the stream, we reset the decoder.

#[derive(Debug)]
pub struct SeekableDeflateDecoder<R: Read + Seek> {
    decoder: Option<flate2::read::DeflateDecoder<R>>,
    position: u64,
    uncompressed_size: u64,
}

impl<R: Read + Seek> SeekableDeflateDecoder<R> {
    pub fn new(inner: R, uncompressed_size: u64) -> Self {
        Self {
            decoder: Some(flate2::read::DeflateDecoder::new(inner)),
            position: 0,
            uncompressed_size,
        }
    }

    pub fn into_inner(self) -> R {
        self.decoder.unwrap().into_inner()
    }

    pub fn get_ref(&self) -> &R {
        self.decoder.as_ref().unwrap().get_ref()
    }

    pub fn get_mut(&mut self) -> &mut R {
        self.decoder.as_mut().unwrap().get_mut()
    }

    fn decoder(&mut self) -> &mut flate2::read::DeflateDecoder<R> {
        self.decoder.as_mut().unwrap()
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.decoder.as_mut().unwrap().get_mut().rewind()?;
        let decoder = self.decoder.take().unwrap();
        self.decoder = Some(flate2::read::DeflateDecoder::new(decoder.into_inner()));
        self.position = 0;
        Ok(())
    }
}

impl<R: Read + Seek> Read for SeekableDeflateDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amount = self.decoder().read(buf)?;
        self.position += amount as u64;
        Ok(amount)
    }
}

impl<R: Read + Seek> Seek for SeekableDeflateDecoder<R> {
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

        let mut buf = [0u8; BUF_SIZE];
        while self.position < seek_pos {
            self.read_exact(
                &mut buf[0..((seek_pos as usize) - (self.position as usize)).min(BUF_SIZE)],
            )?;
        }

        Ok(self.position)
    }
}
