use std::collections::HashMap;

use super::{Class, Module, PickleError, Value};

pub trait PickleTryFrom: Sized {
    fn pickle_try_from(value: Value) -> Result<Self, PickleError>;
}

impl PickleTryFrom for Value {
    fn pickle_try_from(value: Value) -> Result<Self, PickleError> {
        Ok(value)
    }
}

impl<T: PickleTryFrom> PickleTryFrom for Option<T> {
    fn pickle_try_from(value: Value) -> Result<Self, PickleError> {
        if let Value::None = value {
            return Ok(None);
        }
        Ok(Some(T::pickle_try_from(value)?))
    }
}

impl<T: PickleTryFrom> PickleTryFrom for Vec<T> {
    fn pickle_try_from(value: Value) -> Result<Self, PickleError> {
        let items = match value {
            Value::List(vec) => vec.into_boxed_slice(),
            Value::Tuple(vec) => vec,
            _ => return Err(PickleError::FailToExtract("Expected list or tuple")),
        };
        items
            .into_iter()
            .map(|v| T::pickle_try_from(v))
            .collect::<Result<_, _>>()
    }
}

impl<T: PickleTryFrom> PickleTryFrom for HashMap<String, T> {
    fn pickle_try_from(value: Value) -> Result<Self, PickleError> {
        let items = match value {
            Value::Dict(dict) => dict,
            _ => return Err(PickleError::FailToExtract("Expected dict")),
        };
        items
            .into_iter()
            .map(|(k, v)| Ok((k, T::pickle_try_from(v)?)))
            .collect::<Result<_, _>>()
    }
}

macro_rules! impl_try_from_value_simple {
    ($type:ty; $($match:pat => $expr:expr$(,)*)*) => {
        impl PickleTryFrom for $type {
            fn pickle_try_from(value: Value) -> Result<Self, PickleError> {
                Ok(match value {
                    $(
                        $match => $expr,
                    )*
                    _ => {
                        return Err(PickleError::FailToExtract(
                            "Expected specific type to try into",
                        ))
                    }
                })
            }
        }
    };
}

impl_try_from_value_simple!(u8; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(u16; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(u32; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(u64; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(u128; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(i8; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(i16; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(i32; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(i64; Value::Int(int) => int.try_into()?);
impl_try_from_value_simple!(i128; Value::Int(int) => int);

impl_try_from_value_simple!(f32; Value::Float(float) => float as f32);
impl_try_from_value_simple!(f64; Value::Float(float) => float);

impl_try_from_value_simple!(bool; Value::Bool(bool) => bool);
impl_try_from_value_simple!(String; Value::String(str) => str);
impl_try_from_value_simple!(Box<[u8]>; Value::Binary(bin) => bin);

// impl_try_from_value!(Vec<Value>; Value::List(list) => list, Value::Tuple(tuple) => tuple.to_vec());
// impl_try_from_value!(Box<[Value]>; Value::List(list) => list.into_boxed_slice(), Value::Tuple(tuple) => tuple);
// impl_try_from_value!(HashMap<String, Value>; Value::Dict(dict) => dict);

impl_try_from_value_simple!(Module; Value::Module(module) => module);
impl_try_from_value_simple!(Class; Value::Class(class) => *class);

// TODO: Pls clean up :) ik its bad.
// The macro could just be 1 argument but I'm just too lazy to figure it out.
macro_rules! impl_try_from_value_tuple {
    ($count:expr; $($index:expr => $ident:ident),+) => {
        impl<
            $(
                $ident: PickleTryFrom
            ),+
        > PickleTryFrom for ($($ident),+) {
            fn pickle_try_from(value: Value) -> Result<Self, PickleError> {
                Ok(match value {
                    Value::Tuple(tuple) => {
                        if tuple.len() < $count {
                            return Err(PickleError::FailToExtract(
                                concat!("Expected tuple of length >= ", $count),
                            ))
                        }
                        let mut tuple = tuple.into_vec();
                        tuple.truncate($count);
                        let tuple: [Value; $count] = tuple.try_into().unwrap();
                        #[allow(non_snake_case)]
                        let [
                            $($ident),+
                        ] = tuple;
                        (
                            $(
                                $ident::pickle_try_from($ident)?
                            ),+
                        )
                    }
                    _ => {
                        return Err(PickleError::FailToExtract(
                            "Expected tuple",
                        ))
                    }
                })
            }
        }
    };
}

impl_try_from_value_tuple!(2; 0 => T0, 1 => T1);
impl_try_from_value_tuple!(3; 0 => T0, 1 => T1, 2 => T2);
impl_try_from_value_tuple!(4; 0 => T0, 1 => T1, 2 => T2, 3 => T3);
impl_try_from_value_tuple!(5; 0 => T0, 1 => T1, 2 => T2, 3 => T3, 4 => T4);
impl_try_from_value_tuple!(6; 0 => T0, 1 => T1, 2 => T2, 3 => T3, 4 => T4, 5 => T5);
impl_try_from_value_tuple!(7; 0 => T0, 1 => T1, 2 => T2, 3 => T3, 4 => T4, 5 => T5, 6 => T6);
impl_try_from_value_tuple!(8; 0 => T0, 1 => T1, 2 => T2, 3 => T3, 4 => T4, 5 => T5, 6 => T6, 7 => T7);
impl_try_from_value_tuple!(9; 0 => T0, 1 => T1, 2 => T2, 3 => T3, 4 => T4, 5 => T5, 6 => T6, 7 => T7, 8 => T8);

#[derive(Debug)]
pub enum PickleExtractPathValue {
    ListIndex(usize),
    DictKey(String),
    TupleIndex(usize),
    ClassArg,
    ClassState,
    ClassDataDictKey(String),
}

pub fn pickle_extract<T: PickleTryFrom>(
    pickle: &Value,
    path: &[PickleExtractPathValue],
) -> Result<T, PickleError> {
    let mut pickle = pickle;
    for segment in path {
        match segment {
            PickleExtractPathValue::ListIndex(index) => {
                let Value::List(list) = pickle else {
                    return Err(PickleError::FailToExtract("Expected list"));
                };
                pickle = list
                    .get(*index)
                    .ok_or(PickleError::FailToExtract("Expected list index"))?;
            }
            PickleExtractPathValue::DictKey(key) => {
                let Value::Dict(dict) = pickle else {
                    return Err(PickleError::FailToExtract("Expected dict"));
                };
                pickle = dict
                    .get(key)
                    .ok_or(PickleError::FailToExtract("Expected dict key"))?;
            }
            PickleExtractPathValue::TupleIndex(index) => {
                let Value::Tuple(tuple) = pickle else {
                    return Err(PickleError::FailToExtract("Expected tuple"));
                };
                pickle = tuple
                    .get(*index)
                    .ok_or(PickleError::FailToExtract("Expected tuple index"))?;
            }
            PickleExtractPathValue::ClassArg => {
                let Value::Class(class) = pickle else {
                    return Err(PickleError::FailToExtract("Expected class"));
                };
                pickle = &class.args;
            }
            PickleExtractPathValue::ClassState => {
                let Value::Class(class) = pickle else {
                    return Err(PickleError::FailToExtract("Expected class"));
                };
                pickle = &class.state;
            }
            PickleExtractPathValue::ClassDataDictKey(key) => {
                let Value::Class(class) = pickle else {
                    return Err(PickleError::FailToExtract("Expected class"));
                };
                pickle = class
                    .data
                    .get(key)
                    .ok_or(PickleError::FailToExtract("Expected class dict key"))?;
            }
        }
    }
    T::pickle_try_from(pickle.clone())
}

pub fn pickle_extract_class<T: PickleTryFrom>(
    class: &Class,
    path: &[PickleExtractPathValue],
) -> Result<T, PickleError> {
    match path.first() {
        Some(PickleExtractPathValue::ClassArg) => pickle_extract(&class.args, &path[1..]),
        Some(PickleExtractPathValue::ClassState) => pickle_extract(&class.state, &path[1..]),
        Some(PickleExtractPathValue::ClassDataDictKey(key)) => pickle_extract(
            class
                .data
                .get(key)
                .ok_or(PickleError::FailToExtract("Expected class dict key"))?,
            &path[1..],
        ),
        Some(_) => Err(PickleError::FailToExtract(
            "first path value for pickle_extract_class needs to be class specific",
        )),
        None => Err(PickleError::FailToExtract(
            "Need atleast 1 path item for pickle_extract_class",
        )),
    }
}

#[macro_export]
macro_rules! pickle_extract_path {
    ($($inner:tt)*) => {
        $crate::pickle_extract_path_helper!(@collect[],$($inner)*)
    };
}

#[macro_export]
macro_rules! pickle_extract_path_helper {
    (@collect[$($items:expr),*],) => {
        [ $($items),* ]
    };

    (@collect[$($items:expr),*], index[$index:expr] $($rest:tt)*) => {
        $crate::pickle_extract_path_helper!(@collect[$($items,)* $crate::pickle::PickleExtractPathValue::ListIndex($index)], $($rest)*)
    };
    (@collect[$($items:expr),*], key[$key:expr] $($rest:tt)*) => {
        $crate::pickle_extract_path_helper!(@collect[$($items,)* $crate::pickle::PickleExtractPathValue::DictKey($key.to_owned())], $($rest)*)
    };
    (@collect[$($items:expr),*], tuple[$index:expr] $($rest:tt)*) => {
        $crate::pickle_extract_path_helper!(@collect[$($items,)* $crate::pickle::PickleExtractPathValue::TupleIndex($index)], $($rest)*)
    };
    (@collect[$($items:expr),*], class_arg $($rest:tt)*) => {
        $crate::pickle_extract_path_helper!(@collect[$($items,)* $crate::pickle::PickleExtractPathValue::ClassArg], $($rest)*)
    };
    (@collect[$($items:expr),*], class_state $($rest:tt)*) => {
        $crate::pickle_extract_path_helper!(@collect[$($items,)* $crate::pickle::PickleExtractPathValue::ClassState], $($rest)*)
    };
    (@collect[$($items:expr),*], class_data[$key:expr] $($rest:tt)*) => {
        $crate::pickle_extract_path_helper!(@collect[$($items,)* $crate::pickle::PickleExtractPathValue::ClassDataDictKey($key.to_owned())], $($rest)*)
    };
}
