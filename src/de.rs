use core::marker;

use serde::de::Error;
use serde::de::Visitor;
use serde::Deserializer;

use byteorder::ByteOrder;

use super::io::Read;
use super::io::BinaryDeserializerDelegate;

pub struct BinaryDeserializer<'de, R, E, H>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
{
    reader: &'de mut R,
    phantom_data: marker::PhantomData<(E, H)>,
}

impl<'de, R, E, H> BinaryDeserializer<'de, R, E, H>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
{
    pub fn new(read: &'de mut R) -> Self {
        BinaryDeserializer {
            reader: read,
            phantom_data: marker::PhantomData,
        }
    }

    fn read_ex(&mut self) -> Result<&'de [u8], R::Error> {
        let length = H::read_length::<_, E>(self.reader)?;
        self.reader.read(length)
    }
}

macro_rules! primitive {
    ($ty:ty, $method:ident, $visitor_method:ident, $reader:expr) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            use core::mem;

            self.reader.read(mem::size_of::<$ty>())
                .and_then(|buffer| visitor.$visitor_method($reader(&buffer)))
                .map_err(Into::into)
        }
    }
}

impl<'a, 'de, R, E, H> Deserializer<'de> for &'a mut BinaryDeserializer<'de, R, E, H>
where
    R: Read<'de>,
    E: ByteOrder + 'de,
    H: BinaryDeserializerDelegate,
{
    type Error = R::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(<R::Error as Error>::custom("not supported"))
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

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = visitor;
        unimplemented!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use core::str;

        let slice = self.read_ex()?;

        str::from_utf8(slice)
            .map_err(|e| <R::Error as Error>::custom(e))
            .and_then(|s| visitor.visit_borrowed_str(s))
    }

    #[cfg(not(feature = "std"))]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(Error::custom("not supported"))
    }

    #[cfg(feature = "std")]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let slice = self.read_ex()?;

        String::from_utf8(slice.to_owned())
            .map_err(|e| <R::Error as Error>::custom(e))
            .and_then(|s| visitor.visit_string(s))
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read_ex()
            .and_then(|slice| visitor.visit_borrowed_bytes(slice))
    }

    #[cfg(not(feature = "std"))]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(Error::custom("not supported"))
    }

    #[cfg(feature = "std")]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.read_ex()
            .and_then(|slice| visitor.visit_byte_buf(slice.to_owned()))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let variant = H::read_variant::<_, E>(self.reader)?;

        match variant {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(self),
            _ => Err(<R::Error as Error>::custom("unexpected variant")),
        }
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
        let length = H::read_length::<_, E>(self.reader)?;
        self.deserialize_tuple(length, visitor)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use serde::de::SeqAccess;
        use serde::de::DeserializeSeed;

        struct Access<'a, 'de, R, E, H>
        where
            R: Read<'de>,
            E: ByteOrder,
            H: BinaryDeserializerDelegate,
        {
            deserializer: &'a mut BinaryDeserializer<'de, R, E, H>,
            len: usize,
        }

        impl<'a, 'de, R, E, H> SeqAccess<'de> for Access<'a, 'de, R, E, H>
        where
            R: Read<'de>,
            E: ByteOrder,
            H: BinaryDeserializerDelegate,
        {
            type Error = R::Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, R::Error>
            where
                T: DeserializeSeed<'de>,
            {
                if self.len > 0 {
                    self.len -= 1;
                    DeserializeSeed::deserialize(seed, &mut *self.deserializer).map(Some)
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
        let _ = self;
        let _ = visitor;
        unimplemented!()
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
        let _ = self;
        let _ = name;
        let _ = variants;
        let _ = visitor;
        unimplemented!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = visitor;
        Err(<R::Error as Error>::custom("not supported"))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self;
        let _ = visitor;
        Err(<R::Error as Error>::custom("not supported"))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
