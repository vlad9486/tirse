use core::slice;
use byteorder::ByteOrder;

use serde::de;
use serde::ser;

pub trait Read<'de> {
    type Error: de::Error;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error>;
}

impl<'de> Read<'de> for slice::Iter<'de, u8> {
    type Error = Error;

    fn read(&mut self, length: usize) -> Result<&'de [u8], Self::Error> {
        if self.as_slice().len() < length {
            Err(Error::RunOutOfData)
        } else {
            let s = &self.as_slice()[0..length];
            self.nth(length - 1);
            Ok(s)
        }
    }
}

pub trait BinaryDeserializerDelegate {
    fn read_variant<'de, R, E>(r: &mut R) -> Result<u32, R::Error>
    where
        R: Read<'de>,
        E: ByteOrder;

    fn read_length<'de, R, E>(r: &mut R) -> Result<usize, R::Error>
    where
        R: Read<'de>,
        E: ByteOrder;

    fn read_char<'de, R, E>(r: &mut R) -> Result<Option<char>, R::Error>
    where
        R: Read<'de>,
        E: ByteOrder;
}

pub struct DefaultBinaryDeserializerDelegate;

impl BinaryDeserializerDelegate for DefaultBinaryDeserializerDelegate {
    fn read_variant<'de, R, E>(r: &mut R) -> Result<u32, R::Error>
    where
        R: Read<'de>,
        E: ByteOrder,
    {
        use core::mem;
        r.read(mem::size_of::<u32>()).map(E::read_u32)
    }

    fn read_length<'de, R, E>(r: &mut R) -> Result<usize, R::Error>
    where
        R: Read<'de>,
        E: ByteOrder,
    {
        use core::mem;
        match mem::size_of::<usize>() {
            l @ 8 => r.read(l).map(E::read_u64).map(|a| a as _),
            l @ 4 => r.read(l).map(E::read_u32).map(|a| a as _),
            l @ _ => r.read(l).map(E::read_u16).map(|a| a as _),
        }
    }

    fn read_char<'de, R, E>(r: &mut R) -> Result<Option<char>, R::Error>
    where
        R: Read<'de>,
        E: ByteOrder,
    {
        use core::char;
        use core::mem;
        r.read(mem::size_of::<u32>())
            .map(E::read_u32)
            .map(char::from_u32)
    }
}

pub trait Write {
    type Error: ser::Error;

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

#[cfg(not(feature = "std"))]
pub use self::without_std::Error;

#[cfg(not(feature = "std"))]
mod without_std {
    use serde::de;
    use serde::ser;
    use core::fmt;

    #[derive(Debug)]
    pub enum Error {
        RunOutOfData,
        Serialization,
        Deserialization,
    }

    impl fmt::Display for Error {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            use self::Error::*;
            match self {
                &RunOutOfData => write!(fmt, "run out of data"),
                &Serialization => write!(fmt, "some serialization error"),
                &Deserialization => write!(fmt, "some deserialization error"),
            }
        }
    }

    impl<'a> ser::Error for Error {
        fn custom<T: fmt::Display>(desc: T) -> Self {
            let _ = desc;
            Error::Serialization
        }
    }

    impl<'a> de::Error for Error {
        fn custom<T: fmt::Display>(desc: T) -> Self {
            let _ = desc;
            Error::Deserialization
        }
    }
}

#[cfg(feature = "std")]
pub use self::with_std::Error;

#[cfg(feature = "std")]
pub use self::with_std::WriteWrapper;

#[cfg(feature = "std")]
mod with_std {
    use super::Write;

    use serde::de;
    use serde::ser;
    use std::error;
    use std::fmt;
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
        pub fn consume(self) -> T {
            self.raw
        }
    }

    impl<T> Write for WriteWrapper<T>
    where
        T: io::Write,
    {
        type Error = Error;

        fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            io::Write::write(&mut self.raw, bytes)
                .map_err(Error::Io)
                .and_then(|length| {
                    if length < bytes.len() {
                        Err(Error::RunOutOfData)
                    } else {
                        Ok(())
                    }
                })
        }
    }

    #[derive(Debug)]
    pub enum Error {
        RunOutOfData,
        Io(io::Error),
        Serialization(String),
        Deserialization(String),
    }

    impl fmt::Display for Error {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            write!(fmt, "{}", self)
        }
    }

    impl ser::Error for Error {
        fn custom<T: fmt::Display>(desc: T) -> Self {
            Error::Serialization(format!("{}", desc))
        }
    }

    impl de::Error for Error {
        fn custom<T: fmt::Display>(desc: T) -> Self {
            Error::Deserialization(format!("{}", desc))
        }
    }

    impl error::Error for Error {
        fn description(&self) -> &str {
            use self::Error::*;
            use self::error::Error;
            match self {
                &RunOutOfData => "run out of data",
                &Io(ref io_error) => io_error.description(),
                &Serialization(ref msg) => msg,
                &Deserialization(ref msg) => msg,
            }
        }

        fn cause(&self) -> Option<&error::Error> {
            use self::Error::*;
            match self {
                &Io(ref io_error) => Some(io_error),
                _ => None,
            }
        }
    }
}
