#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

mod ser;
mod de;
mod io;
mod err;

pub use self::ser::BinarySerializeSeq;
pub use self::ser::BinarySerializeTuple;
pub use self::ser::BinarySerializeTupleStruct;
pub use self::ser::BinarySerializeTupleVariant;
pub use self::ser::BinarySerializeMap;
pub use self::ser::BinarySerializeStruct;
pub use self::ser::BinarySerializeStructVariant;

pub use self::ser::BinarySerializer;
pub use self::ser::BinarySerializerError;

pub use self::de::BinaryDeserializer;
pub use self::de::BinaryDeserializerError;

pub use self::err::DisplayCollector;

#[cfg(feature = "std")]
pub use self::io::with_std::*;

pub use self::io::Write;
pub use self::io::Read;

pub use self::io::BinarySerializerDelegate;
pub use self::io::DefaultBinarySerializerDelegate;

use byteorder::NativeEndian;

pub type DefaultBinarySerializer<W, D> =
    BinarySerializer<W, NativeEndian, DefaultBinarySerializerDelegate, D>;

pub type DefaultBinaryDeserializer<'de, R, D> = BinaryDeserializer<'de, R, NativeEndian, D>;
