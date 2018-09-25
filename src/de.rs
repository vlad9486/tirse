use core::marker;
use core::fmt;
use core::str;

use serde::de::Error;
use serde::de::Visitor;
use serde::Deserializer;

use byteorder::ByteOrder;

use super::io::Read;

#[cfg(feature = "std")]
use std::string::FromUtf8Error;

pub enum BinaryDeserializerError {
    Expired,
    #[cfg(not(feature = "std"))]
    RequiredAlloc,
    #[cfg(feature = "std")]
    FromUtf8Error(FromUtf8Error),
    WrongUtf8Char,
    Utf8Error(str::Utf8Error),
    UnexpectedVariant,
    NotSupported,
    #[cfg(feature = "std")]
    Custom(String),
}

impl fmt::Display for BinaryDeserializerError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::BinaryDeserializerError::*;
        match self {
            &Expired => write!(fmt, "expired"),
            #[cfg(not(feature = "std"))]
            &RequiredAlloc => write!(fmt, "required alloc"),
            #[cfg(feature = "std")]
            &FromUtf8Error(ref e) => write!(fmt, "{}", e),
            &WrongUtf8Char => write!(fmt, "converted integer out of range for `char`"),
            &Utf8Error(ref e) => write!(fmt, "{}", e),
            &UnexpectedVariant => write!(fmt, "unexpected variant"),
            &NotSupported => write!(fmt, "not supported"),
            #[cfg(feature = "std")]
            &Custom(ref s) => write!(fmt, "{}", s)
        }
    }
}

impl fmt::Debug for BinaryDeserializerError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self)
    }
}

#[cfg(not(feature = "std"))]
impl Error for BinaryDeserializerError {
    fn custom<T>(_: T) -> Self
    where
        T: fmt::Display,
    {
        unimplemented!()
    }
}

#[cfg(feature = "std")]
impl Error for BinaryDeserializerError {
    fn custom<T>(desc: T) -> Self
    where
        T: fmt::Display,
    {
        BinaryDeserializerError::Custom(format!("{}", desc))
    }
}

#[cfg(feature = "std")]
use std::error;

#[cfg(feature = "std")]
impl error::Error for BinaryDeserializerError {
    fn description(&self) -> &str {
        use self::BinaryDeserializerError::*;
        match self {
            &Expired => "expired",
            FromUtf8Error(_) => "from_utf8 error",
            &WrongUtf8Char => "converted integer out of range for `char`",
            &Utf8Error(_) => "utf8 error",
            &UnexpectedVariant => "unexpected variant",
            &NotSupported => "not supported",
            &Custom(_) => "external error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        use self::BinaryDeserializerError::*;
        match self {
            &FromUtf8Error(ref e) => Some(e),
            &Utf8Error(ref e) => Some(e),
            _ => None,
        }
    }
}

pub struct BinaryDeserializer<'de, R, E>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
{
    read: R,
    phantom_data: marker::PhantomData<&'de mut E>,
}

impl<'de, R, E> BinaryDeserializer<'de, R, E>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
{
    pub fn new(read: R) -> Self {
        BinaryDeserializer {
            read: read,
            phantom_data: marker::PhantomData,
        }
    }
}

macro_rules! primitive {
    ($ty:ty, $method:ident, $visitor_method:ident, $reader:expr) => {
        fn $method<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            use core::mem;

            (&mut self.read).read(mem::size_of::<$ty>())
                .map_err(|_| BinaryDeserializerError::Expired)
                .and_then(|buffer| visitor.$visitor_method($reader(&buffer)))
        }
    }
}

impl<'a, 'de, R, E> Deserializer<'de> for BinaryDeserializer<'de, R, E>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
{
    type Error = BinaryDeserializerError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(BinaryDeserializerError::NotSupported)
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
        self.read.read_char::<E>()
            .map_err(|_| BinaryDeserializerError::Expired)
            .and_then(|v| {
                v.ok_or(BinaryDeserializerError::WrongUtf8Char)
            }).and_then(|v| visitor.visit_char(v))
    }

    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read.read_array::<E>()
            .map_err(|_| BinaryDeserializerError::Expired)
            .and_then(|slice| str::from_utf8(slice).map_err(BinaryDeserializerError::Utf8Error))
            .and_then(|s| visitor.visit_borrowed_str(s))
    }

    #[cfg(not(feature = "std"))]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(BinaryDeserializerError::RequiredAlloc)
    }

    #[cfg(feature = "std")]
    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read.read_array::<E>()
            .map_err(|_| BinaryDeserializerError::Expired)
            .map(ToOwned::to_owned).and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(BinaryDeserializerError::FromUtf8Error)
                .and_then(|s| visitor.visit_string(s))
        })
    }

    fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read.read_array::<E>()
            .map_err(|_| BinaryDeserializerError::Expired)
            .and_then(|slice| visitor.visit_borrowed_bytes(slice))
    }

    #[cfg(not(feature = "std"))]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(BinaryDeserializerError::RequiredAlloc)
    }

    #[cfg(feature = "std")]
    fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read.read_array::<E>()
            .map_err(|_| BinaryDeserializerError::Expired)
            .and_then(|slice| visitor.visit_byte_buf(slice.to_owned()))
    }

    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read.read_variant::<E>()
            .map_err(|_| BinaryDeserializerError::Expired)
            .and_then(|variant| match variant {
                0 => visitor.visit_none(),
                1 => visitor.visit_some(self),
                _ => Err(BinaryDeserializerError::UnexpectedVariant),
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
        use serde::de::SeqAccess;
        use serde::de::DeserializeSeed;

        impl<'de, R, E> SeqAccess<'de> for BinaryDeserializer<'de, R, E>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
        {
            type Error = <Self as Deserializer<'de>>::Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                seed.deserialize(BinaryDeserializer::<_, E>::new(&mut self.read))
                    .map(Some)
            }
        }

        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use serde::de::SeqAccess;
        use serde::de::DeserializeSeed;

        struct Access<'de, R, E>
        where
            R: Read<'de>,
            E: ByteOrder,
        {
            deserializer: BinaryDeserializer<'de, R, E>,
            len: usize,
        }

        impl<'a, 'de, R, E> SeqAccess<'de> for Access<'de, R, E>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
        {
            type Error = <BinaryDeserializer<'de, R, E> as Deserializer<'de>>::Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                if self.len > 0 {
                    self.len -= 1;
                    self.deserializer.next_element_seed(seed)
                } else {
                    Ok(None)
                }
            }

            fn size_hint(&self) -> Option<usize> {
                Some(self.len)
            }
        }

        visitor.visit_seq(Access {
            deserializer: self,
            len: len,
        })
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
        use serde::de::DeserializeSeed;

        impl<'de, R, E> MapAccess<'de> for BinaryDeserializer<'de, R, E>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
        {
            type Error = <Self as Deserializer<'de>>::Error;

            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
            where
                K: DeserializeSeed<'de>,
            {
                use serde::de::SeqAccess;

                self.next_element_seed(seed)
            }

            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
            where
                V: DeserializeSeed<'de>,
            {
                use serde::de::SeqAccess;

                self.next_element_seed(seed).map(Option::unwrap)
            }
        }

        visitor.visit_map(self)
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
        use serde::de::EnumAccess;
        use serde::de::VariantAccess;
        use serde::de::Deserialize;
        use serde::de::DeserializeSeed;
        use serde::de::IntoDeserializer;

        impl<'de, R, E> EnumAccess<'de> for BinaryDeserializer<'de, R, E>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
        {
            type Error = <BinaryDeserializer<'de, R, E> as Deserializer<'de>>::Error;
            type Variant = Self;

            fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
            where
                V: DeserializeSeed<'de>,
            {
                u32::deserialize(BinaryDeserializer::<_, E>::new(&mut self.read))
                    .map(IntoDeserializer::into_deserializer)
                    .and_then(|variant| seed.deserialize(variant))
                    .map(|value| (value, self))
            }
        }

        impl<'de, R, E> VariantAccess<'de> for BinaryDeserializer<'de, R, E>
        where
            R: Read<'de>,
            E: ByteOrder + 'de,
        {
            type Error = <BinaryDeserializer<'de, R, E> as Deserializer<'de>>::Error;

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
        Err(BinaryDeserializerError::NotSupported)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = visitor;
        Err(BinaryDeserializerError::NotSupported)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
