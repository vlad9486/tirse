use core::{str, fmt, marker};
use serde::{de::Visitor, Deserializer};
use byteorder::ByteOrder;
use either::Either;
use super::{io::{Read, BinaryDeserializerDelegate}, err::{ErrorAdapter, DisplayCollector}};

#[derive(Debug)]
pub enum BinaryDeserializerError {
    #[cfg(not(feature = "use_std"))]
    RequiredAlloc,
    #[cfg(feature = "use_std")]
    FromUtf8Error(std::string::FromUtf8Error),
    WrongChar(u32),
    Utf8Error(str::Utf8Error),
    UnexpectedVariant(u32),
    NotSupported,
    CannotReadBorrowed,
}

impl fmt::Display for BinaryDeserializerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::BinaryDeserializerError::*;

        match self {
            #[cfg(not(feature = "use_std"))]
            &RequiredAlloc => write!(f, "required alloc"),
            #[cfg(feature = "use_std")]
            &FromUtf8Error(ref e) => write!(f, "{}", e),
            &WrongChar(code) => write!(f, "wrong char code: {}", code),
            &Utf8Error(ref e) => write!(f, "{}", e),
            &UnexpectedVariant(code) => write!(f, "unexpected variant code: {}", code),
            &NotSupported => write!(f, "not supported"),
            &CannotReadBorrowed => write!(f, "cannot read borrowed"),
        }
    }
}

pub struct BinaryDeserializer<'de, R, E, H, D>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
    D: DisplayCollector,
{
    read: R,
    phantom_data: marker::PhantomData<&'de mut (E, H, D)>,
}

impl<'de, R, E, H, D> BinaryDeserializer<'de, R, E, H, D>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
    D: DisplayCollector,
{
    pub fn new(read: R) -> Self {
        BinaryDeserializer {
            read: read,
            phantom_data: marker::PhantomData,
        }
    }

    pub fn split(&mut self) -> BinaryDeserializer<'de, &mut R, E, H, D> {
        BinaryDeserializer::new(&mut self.read)
    }
}

macro_rules! primitive {
    ($ty:ty, $method:ident, $visitor_method:ident, $reader:expr) => {
        fn $method<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            use core::mem;

            self.read.read(mem::size_of::<$ty>())
                .map(|x| x.map($reader))
                .unwrap_or_else(|| {
                    let mut buffer = <H::SmallBuffer as Default>::default();
                    self.read.read_in_buffer(&mut buffer, mem::size_of::<$ty>())
                        .map(move |()| $reader(buffer.as_ref()))
                })
                .map_err(Either::Right)
                .map_err(ErrorAdapter::Inner)
                .and_then(|x| visitor.$visitor_method(x))
        }
    }
}

impl<'a, 'de, R, E, H, D> Deserializer<'de> for BinaryDeserializer<'de, R, E, H, D>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
    D: DisplayCollector + fmt::Display + fmt::Debug,
{
    type Error = ErrorAdapter<Either<BinaryDeserializerError, R::Error>, D>;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(ErrorAdapter::Inner(Either::Left(BinaryDeserializerError::NotSupported)))
    }

    primitive!(bool, deserialize_bool, visit_bool, |b: &[u8]| b[0] != 0);

    primitive!(i8, deserialize_i8, visit_i8, |b: &[u8]| b[0] as i8);
    primitive!(i16, deserialize_i16, visit_i16, E::read_i16);
    primitive!(i32, deserialize_i32, visit_i32, E::read_i32);
    primitive!(i64, deserialize_i64, visit_i64, E::read_i64);

    primitive!(u8, deserialize_u8, visit_u8, |b: &[u8]| b[0]);
    primitive!(u16, deserialize_u16, visit_u16, E::read_u16);
    primitive!(u32, deserialize_u32, visit_u32, E::read_u32);
    primitive!(u64, deserialize_u64, visit_u64, E::read_u64);

    primitive!(f32, deserialize_f32, visit_f32, E::read_f32);
    primitive!(f64, deserialize_f64, visit_f64, E::read_f64);

    fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read
            .read(H::char_size())
            .map(|x| x.map(H::decode_char::<E>))
            .unwrap_or_else(|| {
                let mut buffer = <H::SmallBuffer as Default>::default();
                self.read.read_in_buffer(&mut buffer, H::char_size())
                    .map(move |()| H::decode_char::<E>(&buffer.as_ref()))
            })
            .map_err(Either::Right)
            .and_then(|v| v
                .map_err(BinaryDeserializerError::WrongChar)
                .map_err(Either::Left)
            )
            .map_err(ErrorAdapter::Inner)
            .and_then(|v| visitor.visit_char(v))
    }

    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read
            .read(H::length_size())
            .map(|x| {
                x
                    .map(H::decode_length::<E>)
                    .map_err(Either::Right)
            })
            .unwrap_or(Err(Either::Left(BinaryDeserializerError::CannotReadBorrowed)))
            .map_err(ErrorAdapter::Inner)
            .and_then(|length| {
                self.read
                    .read(length)
                    .unwrap()
                    .map_err(Either::Right)
                    .and_then(|slice| str::from_utf8(slice)
                        .map_err(BinaryDeserializerError::Utf8Error)
                        .map_err(Either::Left)
                    )
                    .map_err(ErrorAdapter::Inner)
                    .and_then(|s| visitor.visit_borrowed_str(s))
            })
    }

    #[cfg(not(feature = "use_std"))]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(ErrorAdapter::Inner(Either::Left(BinaryDeserializerError::RequiredAlloc)))
    }

    #[cfg(feature = "use_std")]
    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read
            .read(H::length_size())
            .map(|x| x.map(H::decode_length::<E>))
            .unwrap_or_else(|| {
                let mut buffer = <H::SmallBuffer as Default>::default();
                self.read.read_in_buffer(&mut buffer, H::length_size())
                    .map(move |()| H::decode_length::<E>(&buffer.as_ref()[..H::length_size()]))
            })
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .and_then(|length| {
                self.read
                    .read(length)
                    .map(|x| x.map(ToOwned::to_owned))
                    .unwrap_or_else(|| {
                        let mut buffer = Vec::new();
                        buffer.resize(length, 0);
                        self.read.read_in_buffer(&mut buffer, length)
                            .map(move |()| buffer)
                    })
                    .map_err(Either::Right)
                    .map_err(ErrorAdapter::Inner)
                    .and_then(|bytes| {
                        String::from_utf8(bytes)
                            .map_err(BinaryDeserializerError::FromUtf8Error)
                            .map_err(Either::Left)
                            .map_err(ErrorAdapter::Inner)
                            .and_then(|s| visitor.visit_string(s))
                    })
            })
    }

    fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read
            .read(H::length_size())
            .map(|x| {
                x
                    .map(H::decode_length::<E>)
                    .map_err(Either::Right)
            })
            .unwrap_or(Err(Either::Left(BinaryDeserializerError::CannotReadBorrowed)))
            .map_err(ErrorAdapter::Inner)
            .and_then(|length| {
                self.read
                    .read(length)
                    .unwrap()
                    .map_err(Either::Right)
                    .map_err(ErrorAdapter::Inner)
                    .and_then(|slice| visitor.visit_borrowed_bytes(slice))
            })
    }

    #[cfg(not(feature = "use_std"))]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(ErrorAdapter::Inner(Either::Left(BinaryDeserializerError::RequiredAlloc)))
    }

    #[cfg(feature = "use_std")]
    fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read
            .read(H::length_size())
            .map(|x| x.map(H::decode_length::<E>))
            .unwrap_or_else(|| {
                let mut buffer = <H::SmallBuffer as Default>::default();
                self.read.read_in_buffer(&mut buffer, H::length_size())
                    .map(move |()| H::decode_length::<E>(buffer.as_ref()))
            })
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .and_then(|length| {
                self.read
                    .read(length)
                    .map(|x| x.map(ToOwned::to_owned))
                    .unwrap_or_else(|| {
                        let mut buffer = Vec::new();
                        buffer.resize(length, 0);
                        self.read.read_in_buffer(&mut buffer, length)
                            .map(move |()| buffer)
                    })
                    .map_err(Either::Right)
                    .map_err(ErrorAdapter::Inner)
                    .and_then(|x| visitor.visit_byte_buf(x))
            })
    }

    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read
            .read(H::variant_size())
            .map(|x| x.map(H::decode_variant::<E>))
            .unwrap_or_else(|| {
                let mut buffer = <H::SmallBuffer as Default>::default();
                self.read.read_in_buffer(&mut buffer, H::variant_size())
                    .map(move |()| H::decode_variant::<E>(buffer.as_ref()))
            })
            .map_err(Either::Right)
            .map_err(ErrorAdapter::Inner)
            .and_then(|variant| match variant {
                0 => visitor.visit_none(),
                1 => visitor.visit_some(self),
                t @ _ => Err(ErrorAdapter::Inner(Either::Left(BinaryDeserializerError::UnexpectedVariant(t)))),
            })
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = name;
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = name;
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SequenceAccess::new(self))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SequenceAccess::new_with_length(self, len))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = name;
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use serde::de::MapAccess;

        impl<'de, R, E, H, D> MapAccess<'de> for SequenceAccess<'de, R, E, H, D>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
            H: BinaryDeserializerDelegate,
            D: DisplayCollector + fmt::Display + fmt::Debug,
        {
            type Error = ErrorAdapter<Either<BinaryDeserializerError, R::Error>, D>;

            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
            where
                K: DeserializeSeed<'de>,
            {
                self.next_element_seed(seed)
            }

            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
            where
                V: DeserializeSeed<'de>,
            {
                self.next_element_seed(seed).map(Option::unwrap)
            }
        }

        visitor.visit_map(SequenceAccess::new(self))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = name;
        self.deserialize_tuple(fields.len(), visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use serde::de::{EnumAccess, VariantAccess, Deserialize, IntoDeserializer};

        impl<'de, R, E, H, D> EnumAccess<'de> for BinaryDeserializer<'de, R, E, H, D>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
            H: BinaryDeserializerDelegate,
            D: DisplayCollector + fmt::Display + fmt::Debug,
        {
            type Error = ErrorAdapter<Either<BinaryDeserializerError, R::Error>, D>;
            type Variant = Self;

            fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
            where
                V: DeserializeSeed<'de>,
            {
                u32::deserialize(self.split())
                    .map(IntoDeserializer::into_deserializer)
                    .and_then(|variant| seed.deserialize(variant))
                    .map(|value| (value, self))
            }
        }

        impl<'de, R, E, H, D> VariantAccess<'de> for BinaryDeserializer<'de, R, E, H, D>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
            H: BinaryDeserializerDelegate,
            D: DisplayCollector + fmt::Display + fmt::Debug,
        {
            type Error = ErrorAdapter<Either<BinaryDeserializerError, R::Error>, D>;

            fn unit_variant(self) -> Result<(), Self::Error> {
                Ok(())
            }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                seed.deserialize(self)
            }

            fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                self.deserialize_tuple(len, visitor)
            }

            fn struct_variant<V>(
                self,
                fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                self.deserialize_tuple(fields.len(), visitor)
            }
        }

        let _ = name;
        let _ = variants;
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = visitor;
        Err(ErrorAdapter::Inner(Either::Left(BinaryDeserializerError::NotSupported)))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = visitor;
        Err(ErrorAdapter::Inner(Either::Left(BinaryDeserializerError::NotSupported)))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

use serde::de::{SeqAccess, DeserializeSeed};

struct SequenceAccess<'de, R, E, H, D>
where
    R: Read<'de>,
    E: ByteOrder,
    H: BinaryDeserializerDelegate,
    D: DisplayCollector + fmt::Display + fmt::Debug,
{
    deserializer: BinaryDeserializer<'de, R, E, H, D>,
    len: Option<usize>,
}

impl<'de, R, E, H, D> SequenceAccess<'de, R, E, H, D>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
    D: DisplayCollector + fmt::Display + fmt::Debug,
{
    fn new(d: BinaryDeserializer<'de, R, E, H, D>) -> Self {
        SequenceAccess {
            deserializer: d,
            len: None,
        }
    }

    fn new_with_length(d: BinaryDeserializer<'de, R, E, H, D>, length: usize) -> Self {
        SequenceAccess {
            deserializer: d,
            len: Some(length),
        }
    }
}

impl<'de, R, E, H, D> SeqAccess<'de> for SequenceAccess<'de, R, E, H, D>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
    D: DisplayCollector + fmt::Display + fmt::Debug,
{
    type Error = ErrorAdapter<Either<BinaryDeserializerError, R::Error>, D>;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let d = self.deserializer.split();

        let length = match self.len {
            Some(length) => length,
            None => {
                d.read
                    .read(H::sequence_length_size())
                    .map(|x| x.map(H::decode_sequence_length::<E>))
                    .unwrap_or_else(|| {
                        let mut buffer = <H::SmallBuffer as Default>::default();
                        d.read.read_in_buffer(&mut buffer, H::sequence_length_size())
                            .map(move |()| H::decode_sequence_length::<E>(buffer.as_ref()))
                    })
                    .map_err(Either::Right)
                    .map_err(ErrorAdapter::Inner)?
                    .unwrap_or(usize::max_value())
            }
        };

        if length > 0 {
            self.len = Some(length - 1);
            d.read.is()
                .map(|()| seed.deserialize(d).map(Some))
                .unwrap_or(Ok(None))
        } else {
            Ok(None)
        }
    }
}
