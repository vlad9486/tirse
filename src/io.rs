use core::slice;
use core::fmt;
use core::ops::Range;
use byteorder::ByteOrder;

use serde::ser;

pub trait Read<'de> {
    fn read(&mut self, length: usize) -> Result<&'de [u8], Range<usize>>;

    fn read_variant<E>(&mut self) -> Result<u32, Range<usize>>
    where
        E: ByteOrder;

    fn read_length<E>(&mut self) -> Result<usize, Range<usize>>
    where
        E: ByteOrder;

    fn read_char<E>(&mut self) -> Result<Option<char>, Range<usize>>
    where
        E: ByteOrder;
}

impl<'a, 'de, R> Read<'de> for &'a mut R
where
    R: Read<'de>,
{
    fn read(&mut self, length: usize) -> Result<&'de [u8], Range<usize>> {
        (&mut **self).read(length)
    }

    fn read_variant<E>(&mut self) -> Result<u32, Range<usize>>
    where
        E: ByteOrder,
    {
        (&mut **self).read_variant::<E>()
    }

    fn read_length<E>(&mut self) -> Result<usize, Range<usize>>
    where
        E: ByteOrder,
    {
        (&mut **self).read_length::<E>()
    }

    fn read_char<E>(&mut self) -> Result<Option<char>, Range<usize>>
    where
        E: ByteOrder,
    {
        (&mut **self).read_char::<E>()
    }
}

pub trait Crop<'de>
where
    Self: Sized + Read<'de>,
{
    fn crop(&self, length: usize) -> Result<Self, Range<usize>>;
}

impl<'de> Read<'de> for slice::Iter<'de, u8> {
    fn read(&mut self, length: usize) -> Result<&'de [u8], Range<usize>> {
        let limit = self.as_slice().len();
        if limit < length {
            Err(limit..length)
        } else {
            let s = &self.as_slice()[0..length];
            self.nth(length - 1);
            Ok(s)
        }
    }

    fn read_variant<E>(&mut self) -> Result<u32, Range<usize>>
    where
        E: ByteOrder,
    {
        use core::mem;
        self.read(mem::size_of::<u32>()).map(E::read_u32)
    }

    fn read_length<E>(&mut self) -> Result<usize, Range<usize>>
    where
        E: ByteOrder,
    {
        use core::mem;
        match mem::size_of::<usize>() {
            l @ 8 => self.read(l).map(E::read_u64).map(|a| a as _),
            l @ 4 => self.read(l).map(E::read_u32).map(|a| a as _),
            l @ _ => self.read(l).map(E::read_u16).map(|a| a as _),
        }
    }

    fn read_char<E>(&mut self) -> Result<Option<char>, Range<usize>>
    where
        E: ByteOrder,
    {
        use core::char;
        use core::mem;
        self.read(mem::size_of::<u32>())
            .map(E::read_u32)
            .map(char::from_u32)
    }
}

impl<'de> Crop<'de> for slice::Iter<'de, u8> {
    fn crop(&self, length: usize) -> Result<Self, Range<usize>> {
        let slice = self.as_slice();
        if slice.len() < length {
            Err(slice.len()..length)
        } else {
            Ok(slice[0..length].iter())
        }
    }
}

pub trait Write {
    type Error: Sized + fmt::Debug + fmt::Display;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

pub trait BinarySerializerDelegate {
    type Variant: ser::Serialize;
    type Length: ser::Serialize;
    type Char: ser::Serialize;

    fn transform_variant(v: u32) -> Self::Variant;
    fn transform_length(v: usize) -> Self::Length;
    fn transform_char(v: char) -> Self::Char;
}

pub struct DefaultBinarySerializerDelegate;

impl BinarySerializerDelegate for DefaultBinarySerializerDelegate {
    type Variant = u32;
    type Length = usize;
    type Char = u32;

    fn transform_variant(v: u32) -> Self::Variant {
        v
    }

    fn transform_length(v: usize) -> Self::Length {
        v
    }

    fn transform_char(v: char) -> Self::Char {
        v as _
    }
}

#[cfg(feature = "std")]
pub mod with_std {
    use super::Write;

    use std::error::Error;
    use std::fmt;
    use std::io;

    pub struct WriteWrapper<T>
    where
        T: io::Write,
    {
        raw: T,
        written: usize,
    }

    impl<T> From<T> for WriteWrapper<T>
    where
        T: io::Write,
    {
        fn from(v: T) -> Self {
            WriteWrapper { raw: v, written: 0 }
        }
    }

    impl<T> WriteWrapper<T>
    where
        T: io::Write,
    {
        pub fn consume(self) -> T {
            self.raw
        }
    }

    impl<T> Write for WriteWrapper<T>
    where
        T: io::Write,
    {
        type Error = WriteError;

        fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            io::Write::write(&mut self.raw, bytes)
                .map_err(WriteError::Io)
                .and_then(|length| {
                    self.written += length;
                    if length < bytes.len() {
                        Err(WriteError::RunOutOfData(self.written))
                    } else {
                        Ok(())
                    }
                })
        }
    }

    #[derive(Debug)]
    pub enum WriteError {
        RunOutOfData(usize),
        Io(io::Error),
    }

    impl fmt::Display for WriteError {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            write!(fmt, "{}", self)
        }
    }

    impl Error for WriteError {
        fn description(&self) -> &str {
            use self::WriteError::*;
            match self {
                &RunOutOfData(_) => "run out of data",
                &Io(ref io_error) => io_error.description(),
            }
        }

        fn cause(&self) -> Option<&Error> {
            use self::WriteError::*;
            match self {
                &Io(ref io_error) => Some(io_error),
                _ => None,
            }
        }
    }
}
