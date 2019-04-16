#![cfg(feature = "use_std")]

use serde::Serialize;
use serde::Deserialize;
use serde_derive::Serialize;
use serde_derive::Deserialize;

use std::slice::Iter;

use tirse::WriteWrapper;
use tirse::DefaultBinarySerializer;
use tirse::DefaultBinaryDeserializer;

type SerializerIntoVec = DefaultBinarySerializer<WriteWrapper<Vec<u8>>, String>;
type DeserializeFromSlice<'a> = DefaultBinaryDeserializer<'a, Iter<'a, u8>, String>;

#[test]
fn test_str() {

    let v = vec![];
    let serializer = SerializerIntoVec::new(v);
    let v = "here".serialize(serializer).unwrap().consume().into_inner();

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
    let v = p.serialize(serializer).unwrap().consume().into_inner();

    assert_eq!(v, vec![17, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]);
    println!("{:?}", v);

    let q = Point3d::deserialize(DeserializeFromSlice::new(v.as_slice().iter())).unwrap();
    assert_eq!(p, q);
}
