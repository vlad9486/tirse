#![cfg(not(feature = "std"))]

use byteorder::LittleEndian;
use serde::ser;
use serde::Serialize;
use serde_derive::Serialize;

use tirse::BinarySerializer;
use tirse::BinarySerializerError;
use tirse::DefaultBinarySerializerDelegate;
use tirse::Write;

use core::fmt;

const SMALL_BUFFER_SIZE: usize = 8;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct SmallBuffer {
    raw: [u8; SMALL_BUFFER_SIZE],
    position: usize,
}

#[derive(Debug, Clone)]
pub enum Error {
    SizeLimit,
    Serialization,
    RequiredAlloc,
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            &SizeLimit => write!(fmt, "size limit reached"),
            &Serialization => write!(fmt, "some serialization error"),
            &RequiredAlloc => write!(fmt, "required alloc"),
        }
    }
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(_desc: T) -> Self {
        Error::Serialization
    }
}

impl Write for SmallBuffer {
    type Error = Error;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let position = self.position + bytes.len();
        if position > SMALL_BUFFER_SIZE {
            Err(Error::SizeLimit)
        } else {
            self.raw.copy_from_slice(bytes);
            self.position = position;
            Ok(())
        }
    }
}

impl BinarySerializerError<SmallBuffer> for Error {
    fn writing(e: <SmallBuffer as Write>::Error) -> Self {
        e
    }

    fn required_alloc() -> Self {
        Error::RequiredAlloc
    }
}

type SerializerIntoSmallBuffer = BinarySerializer<SmallBuffer, LittleEndian, DefaultBinarySerializerDelegate, Error>;

#[test]
fn test() {
    #[derive(Serialize)]
    pub struct Point3d {
        x: f32,
        y: f32,
        z: f32,
    }

    let _p = Point3d {
        x: 0.4,
        y: 7.5,
        z: 0.0,
    };
    let w = SmallBuffer::default();
    let serializer = SerializerIntoSmallBuffer::new(w);
    let buffer = Serialize::serialize(&5u64, serializer)
        .map(SerializerIntoSmallBuffer::consume)
        .unwrap();

    assert_eq!(
        buffer,
        SmallBuffer {
            raw: [5, 0, 0, 0, 0, 0, 0, 0],
            position: 8,
        }
    );
    println!("{:?}", buffer)
}
