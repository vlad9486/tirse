#![cfg(feature = "std")]

#[macro_use]
extern crate serde_derive;

extern crate serde;

extern crate byteorder;
extern crate tirse;

use byteorder::LittleEndian;
use serde::Serialize;
use serde::Deserialize;
use serde::ser::Error as SerError;
use serde::de::Error as DeError;

use std::slice::Iter;
use std::fmt;
use std::error;
use std::str;
use std::string::FromUtf8Error;

use tirse::WriteWrapper;
use tirse::Write;
use tirse::Read;
use tirse::BinarySerializer;
use tirse::BinarySerializerError;
use tirse::DefaultBinarySerializerDelegate;
use tirse::BinaryDeserializer;
use tirse::BinaryDeserializerError;

#[derive(Debug)]
pub struct Error;

impl SerError for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        let _ = msg;
        Error
    }
}

impl DeError for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        let _ = msg;
        Error
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error")
    }
}

impl error::Error for Error {}

impl BinarySerializerError<WriteWrapper<Vec<u8>>> for Error {
    fn writing(e: <WriteWrapper<Vec<u8>> as Write>::Error) -> Self {
        let _ = e;
        Error
    }

    fn required_alloc() -> Self {
        Error
    }
}

impl<'a> BinaryDeserializerError<'a, Iter<'a, u8>> for Error {
    fn reading(e: <Iter<'a, u8> as Read<'a>>::Error) -> Self {
        let _ = e;
        Error
    }

    fn required_alloc() -> Self {
        Error
    }

    fn wrong_char() -> Self {
        Error
    }

    fn utf8_error(e: str::Utf8Error) -> Self {
        let _ = e;
        Error
    }

    fn from_utf8_error(e: FromUtf8Error) -> Self {
        let _ = e;
        Error
    }

    fn unexpected_variant(variant: u32) -> Self {
        let _ = variant;
        Error
    }

    fn not_supported() -> Self {
        Error
    }
}

type SerializerIntoVec = BinarySerializer<WriteWrapper<Vec<u8>>, LittleEndian, DefaultBinarySerializerDelegate, Error>;
type DeserializeFromSlice<'a> = BinaryDeserializer<'a, Iter<'a, u8>, LittleEndian, Error>;

#[test]
fn test_str() {

    let v = vec![];
    let serializer = SerializerIntoVec::new(v);
    let v = "here".serialize(serializer).unwrap().consume().consume();

    assert_eq!(
        v,
        vec![
            4, 0, 0, 0, 0, 0, 0, 0, 'h' as _, 'e' as _, 'r' as _, 'e' as _,
        ]
    );

    let s: &str = Deserialize::deserialize(DeserializeFromSlice::new(v.as_slice().iter())).unwrap();
    assert_eq!(s, "here");

    println!("{:?}", v)
}

#[test]
fn test_struct() {
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    pub struct Point3d {
        x: u32,
        y: u32,
        z: u32,
    }

    let p = Point3d { x: 17, y: 7, z: 0 };
    let v = Vec::new();
    let serializer = SerializerIntoVec::new(v);
    let v = p.serialize(serializer).unwrap().consume().consume();

    assert_eq!(v, vec![17, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]);
    println!("{:?}", v);

    let q = Point3d::deserialize(DeserializeFromSlice::new(v.as_slice().iter())).unwrap();
    assert_eq!(p, q);
}
