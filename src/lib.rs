#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(non_shorthand_field_patterns)]

extern crate byteorder;
extern crate serde;

#[cfg(feature = "std")]
extern crate core;

mod ser;
mod de;
mod io;

pub use self::ser::BinarySerializeSeq;
pub use self::ser::BinarySerializeTuple;
pub use self::ser::BinarySerializeTupleStruct;
pub use self::ser::BinarySerializeTupleVariant;
pub use self::ser::BinarySerializeMap;
pub use self::ser::BinarySerializeStruct;
pub use self::ser::BinarySerializeStructVariant;

pub use self::ser::BinarySerializer;

pub use self::de::BinaryDeserializer;

#[cfg(feature = "std")]
pub use self::io::WriteWrapper;

pub use self::io::Write;
pub use self::io::Read;

pub use self::io::BinarySerializerDelegate;
pub use self::io::BinaryDeserializerDelegate;
pub use self::io::Error;
