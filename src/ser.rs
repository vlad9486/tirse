#[cfg(not(feature = "std"))]
use core::fmt;

use core::marker;

#[cfg(not(feature = "std"))]
use serde::ser::Error;

use serde::ser::SerializeSeq;
use serde::ser::SerializeTuple;
use serde::ser::SerializeTupleStruct;
use serde::ser::SerializeTupleVariant;
use serde::ser::SerializeMap;
use serde::ser::SerializeStruct;
use serde::ser::SerializeStructVariant;
use serde::Serialize;
use serde::Serializer;

use byteorder::ByteOrder;

use super::io::Write;
use super::io::BinarySerializerDelegate;

pub struct BinarySerializer<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    write: W,
    phantom_data: marker::PhantomData<(E, H)>,
}

impl<W, E, H> BinarySerializer<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    pub fn new<WW: Into<W>>(write: WW) -> Self {
        BinarySerializer {
            write: write.into(),
            phantom_data: marker::PhantomData,
        }
    }

    pub fn consume(self) -> W {
        self.write
    }
}

impl<W, E, H> Serializer for BinarySerializer<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = Self;
    type Error = W::Error;
    type SerializeSeq = BinarySerializeSeq<W, E, H>;
    type SerializeTuple = BinarySerializeTuple<W, E, H>;
    type SerializeTupleStruct = BinarySerializeTupleStruct<W, E, H>;
    type SerializeTupleVariant = BinarySerializeTupleVariant<W, E, H>;
    type SerializeMap = BinarySerializeMap<W, E, H>;
    type SerializeStruct = BinarySerializeStruct<W, E, H>;
    type SerializeStructVariant = BinarySerializeStructVariant<W, E, H>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_i8(if v { 1 } else { 0 })
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u8(v as _)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u16(v as _)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(v as _)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as _)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        let mut mut_self = self;
        mut_self
            .write
            .write(&[v])
            .map(|_| mut_self)
            .map_err(Into::into)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<u16>()];
        E::write_u16(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map(|_| mut_self)
            .map_err(Into::into)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<u32>()];
        E::write_u32(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map(|_| mut_self)
            .map_err(Into::into)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<u64>()];
        E::write_u64(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map(|_| mut_self)
            .map_err(Into::into)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<f32>()];
        E::write_f32(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map(|_| mut_self)
            .map_err(Into::into)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<f64>()];
        E::write_f64(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map(|_| mut_self)
            .map_err(Into::into)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let _ = v;
        unimplemented!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let seq = self.serialize_seq(Some(v.len()));
        (0..v.len())
            .fold(seq, |seq, index| {
                seq.and_then(|mut s| s.serialize_element(&v[index]).map(|_| s))
            }).and_then(|s| s.end())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_bool(false)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.serialize_bool(true).and_then(|s| value.serialize(s))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(self)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        let _ = name;
        Ok(self)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        let _ = name;
        let _ = variant;
        H::transform_variant(variant_index).serialize(self)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let _ = name;
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let _ = name;
        let _ = variant;
        H::transform_variant(variant_index)
            .serialize(self)
            .and_then(|s| value.serialize(s))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let maybe_self = match len {
            Some(len) => H::transform_length(len).serialize(self),
            None => Ok(self),
        };
        maybe_self.and_then(|x| Ok(BinarySerializeSeq { raw: Ok(x) }))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let _ = len;
        let sequence = BinarySerializeSeq { raw: Ok(self) };
        Ok(BinarySerializeTuple { sequence: sequence })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        let _ = name;
        let _ = len;
        let sequence = BinarySerializeSeq { raw: Ok(self) };
        Ok(BinarySerializeTupleStruct { sequence: sequence })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        let _ = name;
        let _ = variant;
        let _ = len;
        H::transform_variant(variant_index)
            .serialize(self)
            .and_then(|x| {
                let sequence = BinarySerializeSeq { raw: Ok(x) };
                Ok(BinarySerializeTupleVariant { sequence: sequence })
            })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let maybe_self = match len {
            Some(len) => H::transform_length(len).serialize(self),
            None => Ok(self),
        };
        maybe_self.and_then(|x| {
            let sequence = BinarySerializeSeq { raw: Ok(x) };
            Ok(BinarySerializeMap { sequence: sequence })
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let _ = name;
        let _ = len;
        let sequence = BinarySerializeSeq { raw: Ok(self) };
        Ok(BinarySerializeStruct { sequence: sequence })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        let _ = name;
        let _ = variant;
        let _ = len;
        H::transform_variant(variant_index)
            .serialize(self)
            .and_then(|x| {
                let sequence = BinarySerializeSeq { raw: Ok(x) };
                Ok(BinarySerializeStructVariant { sequence: sequence })
            })
    }

    #[cfg(not(feature = "std"))]
    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + fmt::Display,
    {
        let _ = value;
        Err(W::Error::custom("cannot collect string without std"))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct BinarySerializeSeq<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    raw: Result<BinarySerializer<W, E, H>, Option<W::Error>>,
}

impl<W, E, H> SerializeSeq for BinarySerializeSeq<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = BinarySerializer<W, E, H>;
    type Error = W::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        use core::mem;

        let mut temp = Err(None);
        mem::swap(&mut temp, &mut self.raw);
        let mut temp = temp.and_then(|s| value.serialize(s).map_err(|e| Some(e)));
        mem::swap(&mut temp, &mut self.raw);
        self.raw.as_mut().map(|_| ()).map_err(|e| {
            let mut temp = None;
            mem::swap(&mut temp, e);
            temp.unwrap()
        })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.raw.map_err(|e| e.unwrap())
    }
}

pub struct BinarySerializeTuple<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    sequence: BinarySerializeSeq<W, E, H>,
}

impl<W, E, H> SerializeTuple for BinarySerializeTuple<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = BinarySerializer<W, E, H>;
    type Error = W::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.sequence.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.sequence.end()
    }
}

pub struct BinarySerializeTupleStruct<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    sequence: BinarySerializeSeq<W, E, H>,
}

impl<W, E, H> SerializeTupleStruct for BinarySerializeTupleStruct<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = BinarySerializer<W, E, H>;
    type Error = W::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.sequence.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.sequence.end()
    }
}

pub struct BinarySerializeTupleVariant<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    sequence: BinarySerializeSeq<W, E, H>,
}

impl<W, E, H> SerializeTupleVariant for BinarySerializeTupleVariant<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = BinarySerializer<W, E, H>;
    type Error = W::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.sequence.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.sequence.end()
    }
}

pub struct BinarySerializeMap<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    sequence: BinarySerializeSeq<W, E, H>,
}

impl<W, E, H> SerializeMap for BinarySerializeMap<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = BinarySerializer<W, E, H>;
    type Error = W::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.sequence.serialize_element(key)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.sequence.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.sequence.end()
    }
}

pub struct BinarySerializeStruct<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    sequence: BinarySerializeSeq<W, E, H>,
}

impl<W, E, H> SerializeStruct for BinarySerializeStruct<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = BinarySerializer<W, E, H>;
    type Error = W::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let _ = key;

        self.sequence.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.sequence.end()
    }
}

pub struct BinarySerializeStructVariant<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    sequence: BinarySerializeSeq<W, E, H>,
}

impl<W, E, H> SerializeStructVariant for BinarySerializeStructVariant<W, E, H>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
{
    type Ok = BinarySerializer<W, E, H>;
    type Error = W::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let _ = key;

        self.sequence.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.sequence.end()
    }
}
