use core::{fmt, marker};
use serde::{
    Serialize,
    Serializer,
    ser::{
        SerializeSeq,
        SerializeTuple,
        SerializeTupleStruct,
        SerializeTupleVariant,
        SerializeMap,
        SerializeStruct,
        SerializeStructVariant
    }
};
use byteorder::ByteOrder;
use either::Either;
use super::{
    io::{
        Write,
        BinarySerializerDelegate
    },
    err::{
        ErrorAdapter,
        DisplayCollector
    }
};

#[derive(Debug)]
pub enum BinarySerializerError {
    // looks like there is nothing
}

impl fmt::Display for BinarySerializerError {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        unreachable!()
    }
}

pub struct BinarySerializer<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: DisplayCollector,
{
    write: W,
    phantom_data: marker::PhantomData<(E, H, D)>,
}

impl<W, E, H, D> BinarySerializer<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: DisplayCollector,
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

impl<W, E, H, D> Serializer for BinarySerializer<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = Self;
    type Error = ErrorAdapter<Either<BinarySerializerError, W::Error>, D>;
    type SerializeSeq = BinarySerializeSeq<W, E, H, D>;
    type SerializeTuple = BinarySerializeTuple<W, E, H, D>;
    type SerializeTupleStruct = BinarySerializeTupleStruct<W, E, H, D>;
    type SerializeTupleVariant = BinarySerializeTupleVariant<W, E, H, D>;
    type SerializeMap = BinarySerializeMap<W, E, H, D>;
    type SerializeStruct = BinarySerializeStruct<W, E, H, D>;
    type SerializeStructVariant = BinarySerializeStructVariant<W, E, H, D>;

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
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .map(|_| mut_self)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<u16>()];
        E::write_u16(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .map(|_| mut_self)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<u32>()];
        E::write_u32(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .map(|_| mut_self)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<u64>()];
        E::write_u64(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .map(|_| mut_self)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<f32>()];
        E::write_f32(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .map(|_| mut_self)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        use core::mem;

        let mut mut_self = self;
        let mut buffer = [0; mem::size_of::<f64>()];
        E::write_f64(&mut buffer, v);
        mut_self
            .write
            .write(&buffer)
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .map(|_| mut_self)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        H::encode_char(v).serialize(self)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let seq = self.serialize_seq(Some(v.len()));
        (0..v.len())
            .fold(seq, |seq, index| {
                seq.and_then(|mut s| s.serialize_element(&v[index]).map(|_| s))
            })
            .and_then(|s| s.end())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        H::encode_variant(0)
            .serialize(self)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        H::encode_variant(1)
            .serialize(self)
            .and_then(|s| value.serialize(s))
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
        H::encode_variant(variant_index).serialize(self)
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
        H::encode_variant(variant_index)
            .serialize(self)
            .and_then(|s| value.serialize(s))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let maybe_self = match len {
            Some(len) => H::encode_length(len).serialize(self),
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
        H::encode_variant(variant_index)
            .serialize(self)
            .and_then(|x| {
                let sequence = BinarySerializeSeq { raw: Ok(x) };
                Ok(BinarySerializeTupleVariant { sequence: sequence })
            })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let maybe_self = match len {
            Some(len) => H::encode_length(len).serialize(self),
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
        H::encode_variant(variant_index)
            .serialize(self)
            .and_then(|x| {
                let sequence = BinarySerializeSeq { raw: Ok(x) };
                Ok(BinarySerializeStructVariant { sequence: sequence })
            })
    }

    #[cfg(not(feature = "use_std"))]
    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + fmt::Display,
    {
        D::display(value).serialize(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct BinarySerializeSeq<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    raw: Result<BinarySerializer<W, E, H, D>, Option<ErrorAdapter<Either<BinarySerializerError, W::Error>, D>>>,
}

impl<W, E, H, D> SerializeSeq for BinarySerializeSeq<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = BinarySerializer<W, E, H, D>;
    type Error = <Self::Ok as Serializer>::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        use core::mem;

        let mut temp = Err(None);
        mem::swap(&mut temp, &mut self.raw);
        let mut temp = temp.and_then(|s| value.serialize(s).map_err(Some));
        mem::swap(&mut temp, &mut self.raw);
        self.raw.as_mut().map(|_| ()).map_err(|e| {
            let mut temp = None;
            mem::swap(&mut temp, e);
            temp.unwrap()
        })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.raw.map_err(Option::unwrap)
    }
}

pub struct BinarySerializeTuple<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    sequence: BinarySerializeSeq<W, E, H, D>,
}

impl<W, E, H, D> SerializeTuple for BinarySerializeTuple<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = BinarySerializer<W, E, H, D>;
    type Error = <Self::Ok as Serializer>::Error;

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

pub struct BinarySerializeTupleStruct<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    sequence: BinarySerializeSeq<W, E, H, D>,
}

impl<W, E, H, D> SerializeTupleStruct for BinarySerializeTupleStruct<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = BinarySerializer<W, E, H, D>;
    type Error = <Self::Ok as Serializer>::Error;

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

pub struct BinarySerializeTupleVariant<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    sequence: BinarySerializeSeq<W, E, H, D>,
}

impl<W, E, H, D> SerializeTupleVariant for BinarySerializeTupleVariant<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = BinarySerializer<W, E, H, D>;
    type Error = <Self::Ok as Serializer>::Error;

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

pub struct BinarySerializeMap<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    sequence: BinarySerializeSeq<W, E, H, D>,
}

impl<W, E, H, D> SerializeMap for BinarySerializeMap<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = BinarySerializer<W, E, H, D>;
    type Error = <Self::Ok as Serializer>::Error;

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

pub struct BinarySerializeStruct<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    sequence: BinarySerializeSeq<W, E, H, D>,
}

impl<W, E, H, D> SerializeStruct for BinarySerializeStruct<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = BinarySerializer<W, E, H, D>;
    type Error = <Self::Ok as Serializer>::Error;

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

pub struct BinarySerializeStructVariant<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    sequence: BinarySerializeSeq<W, E, H, D>,
}

impl<W, E, H, D> SerializeStructVariant for BinarySerializeStructVariant<W, E, H, D>
where
    W: Write,
    E: ByteOrder,
    H: BinarySerializerDelegate,
    D: Serialize + DisplayCollector + fmt::Display + fmt::Debug,
{
    type Ok = BinarySerializer<W, E, H, D>;
    type Error = <Self::Ok as Serializer>::Error;

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
