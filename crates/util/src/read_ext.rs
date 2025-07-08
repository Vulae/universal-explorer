use std::io::Read;

pub trait ReadExt {
    fn read_const<const N: usize>(&mut self) -> std::io::Result<[u8; N]>;
    fn read_var(&mut self, size: usize) -> std::io::Result<Box<[u8]>>;
    fn read_null_terminated_string(&mut self) -> std::io::Result<String>;
}

impl<T: Read> ReadExt for T {
    fn read_const<const N: usize>(&mut self) -> std::io::Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_var(&mut self, size: usize) -> std::io::Result<Box<[u8]>> {
        let mut buf = vec![0u8; size].into_boxed_slice();
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_null_terminated_string(&mut self) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        loop {
            let [byte] = self.read_const::<1>()?;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        String::from_utf8(bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }
}
