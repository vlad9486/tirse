use core::{slice, fmt, ops};
use byteorder::ByteOrder;
use serde::ser;

pub trait Read<'de> {
    type Error: fmt::Display + fmt::Debug;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error>;

    fn read_variant<E>(&mut self) -> Result<u32, Self::Error>
    where
        E: ByteOrder;

    fn read_length<E>(&mut self) -> Result<usize, Self::Error>
    where
        E: ByteOrder;

    fn read_char<E>(&mut self) -> Result<Result<char, u32>, Self::Error>
    where
        E: ByteOrder;
}

impl<'a, 'de, R> Read<'de> for &'a mut R
where
    R: Read<'de>,
{
    type Error = R::Error;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error> {
        (&mut **self).read(length)
    }

    fn read_variant<E>(&mut self) -> Result<u32, Self::Error>
    where
        E: ByteOrder,
    {
        (&mut **self).read_variant::<E>()
    }

    fn read_length<E>(&mut self) -> Result<usize, Self::Error>
    where
        E: ByteOrder,
    {
        (&mut **self).read_length::<E>()
    }

    fn read_char<E>(&mut self) -> Result<Result<char, u32>, Self::Error>
    where
        E: ByteOrder,
    {
        (&mut **self).read_char::<E>()
    }
}

#[derive(Debug)]
pub struct IterReadError {
    missing: ops::Range<usize>,
}

impl fmt::Display for IterReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to read {}..{}", self.missing.start, self.missing.end)
    }
}

impl<'de> Read<'de> for slice::Iter<'de, u8> {
    type Error = IterReadError;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error> {
        let limit = self.as_slice().len();
        if limit < length {
            Err(IterReadError { missing: limit..length })
        } else {
            let s = &self.as_slice()[0..length];
            self.nth(length - 1);
            Ok(s)
        }
    }

    fn read_variant<E>(&mut self) -> Result<u32, Self::Error>
    where
        E: ByteOrder,
    {
        use core::mem;
        self.read(mem::size_of::<u32>()).map(E::read_u32)
    }

    fn read_length<E>(&mut self) -> Result<usize, Self::Error>
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

    fn read_char<E>(&mut self) -> Result<Result<char, u32>, Self::Error>
    where
        E: ByteOrder,
    {
        use core::{mem, char};
        self.read(mem::size_of::<u32>())
            .map(E::read_u32)
            .map(|code| char::from_u32(code).ok_or(code))
    }
}

pub trait Write {
    type Error: fmt::Display + fmt::Debug;

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
    use std::io;

    pub struct WriteWrapper<T>
    where
        T: io::Write,
    {
        raw: T,
    }

    impl<T> From<T> for WriteWrapper<T>
    where
        T: io::Write,
    {
        fn from(v: T) -> Self {
            WriteWrapper { raw: v }
        }
    }

    impl<T> WriteWrapper<T>
    where
        T: io::Write,
    {
        pub fn into_inner(self) -> T {
            self.raw
        }
    }

    impl<T> Write for WriteWrapper<T>
    where
        T: io::Write,
    {
        type Error = io::Error;

        fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            io::Write::write_all(&mut self.raw, bytes)
        }
    }
}
