use core::{slice, fmt, ops};
use byteorder::ByteOrder;
use serde::ser;

pub trait Read<'de> {
    type Error: fmt::Display + fmt::Debug;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error>;

    fn is(&self) -> Option<()>;
}

impl<'a, 'de, R> Read<'de> for &'a mut R
where
    R: Read<'de>,
{
    type Error = R::Error;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error> {
        (&mut **self).read(length)
    }

    fn is(&self) -> Option<()> {
        (&**self).is()
    }
}

#[derive(Debug)]
pub struct IoError {
    missing: ops::Range<usize>,
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to read {}..{}", self.missing.start, self.missing.end)
    }
}

impl<'de> Read<'de> for slice::Iter<'de, u8> {
    type Error = IoError;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error> {
        let limit = self.as_slice().len();
        if limit < length {
            Err(IoError { missing: limit..length })
        } else {
            let s = &self.as_slice()[0..length];
            self.nth(length - 1);
            Ok(s)
        }
    }

    fn is(&self) -> Option<()> {
        if self.as_slice().len() != 0 {
            Some(())
        } else {
            None
        }
    }
}

pub trait BinaryDeserializerDelegate {
    fn variant_size() -> usize;
    fn length_size() -> usize;
    fn sequence_length_size() -> usize;
    fn char_size() -> usize;

    fn decode_variant<E>(bytes: &[u8]) -> u32 where E: ByteOrder;
    fn decode_length<E>(bytes: &[u8]) -> usize where E: ByteOrder;
    fn decode_sequence_length<E>(bytes: &[u8]) -> Option<usize> where E: ByteOrder;
    fn decode_char<E>(bytes: &[u8]) -> Result<char, u32> where E: ByteOrder;
}

pub struct DefaultBinaryDeserializerDelegate;

impl BinaryDeserializerDelegate for DefaultBinaryDeserializerDelegate {
    fn variant_size() -> usize {
        core::mem::size_of::<u32>()
    }

    fn length_size() -> usize {
        core::mem::size_of::<usize>()
    }

    fn sequence_length_size() -> usize {
        0
    }

    fn char_size() -> usize {
        core::mem::size_of::<u32>()
    }

    fn decode_variant<E>(bytes: &[u8]) -> u32 where E: ByteOrder {
        E::read_u32(bytes)
    }

    fn decode_length<E>(bytes: &[u8]) -> usize where E: ByteOrder {
        match Self::length_size() {
            8 => E::read_u64(bytes) as usize,
            4 => E::read_u32(bytes) as usize,
            _ => E::read_u16(bytes) as usize,
        }
    }

    fn decode_sequence_length<E>(bytes: &[u8]) -> Option<usize> where E: ByteOrder {
        let _ = bytes;
        None
    }

    fn decode_char<E>(bytes: &[u8]) -> Result<char, u32> where E: ByteOrder {
        let code = E::read_u32(bytes);
        core::char::from_u32(code).ok_or(code)
    }
}

pub trait Write {
    type Error: fmt::Display + fmt::Debug;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

impl<'de> Write for slice::IterMut<'de, u8> {
    type Error = IoError;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        use core::mem;

        let limit = self.size_hint().0;
        let length = bytes.len();
        if limit < length {
            Err(IoError { missing: limit..length })
        } else {
            let mut temp = (&mut []).iter_mut();
            mem::swap(&mut temp, self);
            let slice = temp.into_slice();
            slice.copy_from_slice(bytes);
            *self = slice[length..].iter_mut();
            Ok(())
        }
    }
}

pub trait BinarySerializerDelegate {
    type Variant: ser::Serialize;
    type Length: ser::Serialize;
    type SequenceLength: ser::Serialize;
    type Char: ser::Serialize;

    fn encode_variant(v: u32) -> Self::Variant;
    fn encode_length(v: usize) -> Self::Length;
    fn encode_sequence_length(v: usize) -> Self::SequenceLength;
    fn encode_char(v: char) -> Self::Char;
}

pub struct DefaultBinarySerializerDelegate;

impl BinarySerializerDelegate for DefaultBinarySerializerDelegate {
    type Variant = u32;
    type Length = usize;
    type SequenceLength = usize;
    type Char = u32;

    fn encode_variant(v: u32) -> Self::Variant {
        v
    }

    fn encode_length(v: usize) -> Self::Length {
        v
    }

    fn encode_sequence_length(v: usize) -> Self::SequenceLength {
        v
    }

    fn encode_char(v: char) -> Self::Char {
        v as _
    }
}

#[cfg(feature = "use_std")]
pub use self::with_std::WriteWrapper;

#[cfg(feature = "use_std")]
mod with_std {
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
