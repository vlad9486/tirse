#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "use_std"), no_std)]

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

#[cfg(feature = "use_std")]
pub use self::io::{WriteWrapper, ReadWrapper};

pub use self::io::Write;
pub use self::io::Read;
pub use self::io::IoError;

pub use self::io::BinarySerializerDelegate;
pub use self::io::DefaultBinarySerializerDelegate;
pub use self::io::BinaryDeserializerDelegate;
pub use self::io::DefaultBinaryDeserializerDelegate;

pub use self::err::DisplayCollector;
pub use self::err::ErrorAdapter;

use byteorder::NativeEndian;

pub type DefaultBinarySerializer<W, D> =
    BinarySerializer<W, NativeEndian, DefaultBinarySerializerDelegate, D>;

pub type DefaultBinaryDeserializer<'de, R, D> =
    BinaryDeserializer<'de, R, NativeEndian, DefaultBinaryDeserializerDelegate, D>;
