use std::{collections::HashMap, fmt::Display};

use serde::{
    de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor},
    Deserialize,
};

use super::{PickleError, Value};

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int(v as i128))
            }

            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int(v))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int(v as i128))
            }

            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int(
                    std::convert::TryInto::<i128>::try_into(v)
                        .map_err(|err| E::custom(err.to_string()))?,
                ))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Float(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(v.to_owned()))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Binary(v.to_vec().into_boxed_slice()))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::None)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                // TODO: Is this correct?
                match deserializer.deserialize_any(self)? {
                    Value::None => Err(serde::de::Error::custom("Expected some")),
                    val => Ok(val),
                }
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::None)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();

                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }

                Ok(Value::List(vec))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = HashMap::new();

                while let Some((key, value)) = map.next_entry()? {
                    values.insert(key, value);
                }

                Ok(Value::Dict(values))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl serde::de::Error for PickleError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        PickleError::SerdeDeserializeError(msg.to_string())
    }
}

struct PickleValueListVisitor<L: Iterator<Item = Value>>(L);

impl<'de, L: Iterator<Item = Value>> SeqAccess<'de> for PickleValueListVisitor<L> {
    type Error = PickleError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.0
            .next()
            .map(|next| seed.deserialize(PickleValueDeserializer(next)))
            .transpose()
    }
}

struct PickleValueDictVisitor<M: Iterator<Item = (String, Value)>> {
    map: M,
    stored_value: Option<Value>,
}

impl<'de, M: Iterator<Item = (String, Value)>> MapAccess<'de> for PickleValueDictVisitor<M> {
    type Error = PickleError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.stored_value.is_some() {
            panic!();
        }
        let Some((key, value)) = self.map.next() else {
            return Ok(None);
        };
        self.stored_value = Some(value);
        Ok(Some(seed.deserialize(PickleValueDeserializer(
            Value::String(key),
        ))?))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let Some(value) = self.stored_value.take() else {
            panic!();
        };
        seed.deserialize(PickleValueDeserializer(value))
    }
}

pub(super) struct PickleValueDeserializer(pub Value);

macro_rules! deserialize_numtype {
    ($type:ty) => {
        paste::paste! {
            fn [<deserialize_ $type>]<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                match self.0 {
                    Value::Int(int) => visitor.[<visit_ $type>](int.try_into()?),
                    _ => Err(PickleError::custom("Invalid type")),
                }
            }
        }
    };
}

impl<'de> serde::de::Deserializer<'de> for PickleValueDeserializer {
    type Error = PickleError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.0 {
            Value::None => visitor.visit_none(),
            Value::Bool(bool) => visitor.visit_bool(bool),
            Value::Int(int) => visitor.visit_i128(int),
            Value::Float(float) => visitor.visit_f64(float),
            Value::String(string) => visitor.visit_string(string),
            Value::Binary(items) => visitor.visit_bytes(&items),
            Value::List(list) => visitor.visit_seq(PickleValueListVisitor(list.into_iter())),
            Value::Dict(dict) => visitor.visit_map(PickleValueDictVisitor {
                map: dict.into_iter(),
                stored_value: None,
            }),
            Value::Tuple(tuple) => visitor.visit_seq(PickleValueListVisitor(tuple.into_iter())),
            // TODO: Deserialize module & class
            Value::Module(_module) => visitor.visit_unit(),
            Value::Class(_class) => visitor.visit_unit(),
        }
    }

    deserialize_numtype!(i8);
    deserialize_numtype!(i16);
    deserialize_numtype!(i32);
    deserialize_numtype!(i64);
    deserialize_numtype!(i128);
    deserialize_numtype!(u8);
    deserialize_numtype!(u16);
    deserialize_numtype!(u32);
    deserialize_numtype!(u64);
    deserialize_numtype!(u128);

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool
        f32 f64 char str string
        bytes byte_buf seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
