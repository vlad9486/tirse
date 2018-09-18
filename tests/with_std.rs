#![cfg(feature = "std")]

#[macro_use]
extern crate serde_derive;

extern crate serde;

extern crate byteorder;
extern crate tirse;

use byteorder::LittleEndian;
use serde::Serialize;
use serde::Deserialize;

use std::slice::Iter;

use tirse::WriteWrapper;
use tirse::BinarySerializer;
use tirse::BinarySerializerDelegate;
use tirse::BinaryDeserializerDelegate;
use tirse::BinaryDeserializer;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Point3d {
    x: u32,
    y: u32,
    z: u32,
}

struct Helper;

impl BinarySerializerDelegate for Helper {
    type Variant = u32;
    type Length = usize;

    fn transform_variant(v: u32) -> Self::Variant {
        v
    }

    fn transform_length(v: usize) -> Self::Length {
        v
    }
}

impl BinaryDeserializerDelegate for Helper {
}

type SerializerIntoVec = BinarySerializer<WriteWrapper<Vec<u8>>, LittleEndian, Helper>;
type DeserializeFromSlice<'a> = BinaryDeserializer<'a, Iter<'a, u8>, LittleEndian, Helper>;

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

    let mut r = v.as_slice().iter();
    let s: &str = Deserialize::deserialize(&mut DeserializeFromSlice::new(&mut r)).unwrap();
    assert_eq!(s, "here");

    println!("{:?}", v)
}

#[test]
fn test_struct() {
    let p = Point3d { x: 17, y: 7, z: 0 };
    let v = vec![];
    let serializer = SerializerIntoVec::new(v);
    let v = p.serialize(serializer).unwrap().consume().consume();

    assert_eq!(v, vec![17, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]);
    println!("{:?}", v);

    let mut r = v.as_slice().iter();
    let q = Point3d::deserialize(&mut DeserializeFromSlice::new(&mut r)).unwrap();
    assert_eq!(p, q);
}
