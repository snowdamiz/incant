//! Trusted one-way observer only. Structs retain named fields; maps retain an
//! ordered array of key/value rows, including tuple keys rejected by JSON maps.
use crate::alloc_prelude::*;
use serde::{Serialize, ser::*};
use serde_json::{Map, Value};
use std::string::{String, ToString};
type Result<T> = std::result::Result<T, serde_json::Error>;
pub fn capture<T: Serialize + ?Sized>(v: &T) -> Value {
    v.serialize(Observer).unwrap()
}
struct Observer;
macro_rules! scalar {($($name:ident:$t:ty),*)=>{$(fn $name(self,v:$t)->Result<Value>{serde::Serializer::$name(serde_json::value::Serializer,v)})*};}
impl serde::Serializer for Observer {
    type Ok = Value;
    type Error = serde_json::Error;
    type SerializeSeq = Array;
    type SerializeTuple = Array;
    type SerializeTupleStruct = Array;
    type SerializeTupleVariant = Array;
    type SerializeMap = Entries;
    type SerializeStruct = Fields;
    type SerializeStructVariant = Fields;
    scalar!(serialize_bool:bool,serialize_i8:i8,serialize_i16:i16,serialize_i32:i32,serialize_i64:i64,serialize_i128:i128,serialize_u8:u8,serialize_u16:u16,serialize_u32:u32,serialize_u64:u64,serialize_f32:f32,serialize_f64:f64,serialize_char:char,serialize_str:&str,serialize_bytes:&[u8]);
    fn serialize_u128(self, v: u128) -> Result<Value> {
        Ok(Value::String(v.to_string()))
    }
    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<Value> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, v: &'static str) -> Result<Value> {
        Ok(Value::String(v.into()))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<Value> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        v: &T,
    ) -> Result<Value> {
        Ok(wrap(Some(k), v.serialize(self)?))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Array> {
        Ok(Array(vec![], None))
    }
    fn serialize_tuple(self, _: usize) -> Result<Array> {
        Ok(Array(vec![], None))
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Array> {
        Ok(Array(vec![], None))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        _: usize,
    ) -> Result<Array> {
        Ok(Array(vec![], Some(k)))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Entries> {
        Ok(Entries {
            rows: vec![],
            key: None,
        })
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Fields> {
        Ok(Fields(Map::new(), None))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        _: usize,
    ) -> Result<Fields> {
        Ok(Fields(Map::new(), Some(k)))
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}
fn wrap(k: Option<&'static str>, v: Value) -> Value {
    if let Some(k) = k {
        let mut m = Map::new();
        m.insert(k.into(), v);
        Value::Object(m)
    } else {
        v
    }
}
struct Array(Vec<Value>, Option<&'static str>);
macro_rules! array {
    ($trait:ident,$method:ident) => {
        impl $trait for Array {
            type Ok = Value;
            type Error = serde_json::Error;
            fn $method<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
                self.0.push(v.serialize(Observer)?);
                Ok(())
            }
            fn end(self) -> Result<Value> {
                Ok(wrap(self.1, Value::Array(self.0)))
            }
        }
    };
}
array!(SerializeSeq, serialize_element);
array!(SerializeTuple, serialize_element);
array!(SerializeTupleStruct, serialize_field);
array!(SerializeTupleVariant, serialize_field);
struct Entries {
    rows: Vec<Value>,
    key: Option<Value>,
}
impl SerializeMap for Entries {
    type Ok = Value;
    type Error = serde_json::Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<()> {
        assert!(self.key.is_none());
        self.key = Some(k.serialize(Observer)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        self.rows.push(Value::Array(vec![
            self.key.take().unwrap(),
            v.serialize(Observer)?,
        ]));
        Ok(())
    }
    fn end(self) -> Result<Value> {
        assert!(self.key.is_none());
        Ok(Value::Array(self.rows))
    }
}
struct Fields(Map<String, Value>, Option<&'static str>);
macro_rules! fields {
    ($trait:ident) => {
        impl $trait for Fields {
            type Ok = Value;
            type Error = serde_json::Error;
            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                k: &'static str,
                v: &T,
            ) -> Result<()> {
                assert!(self.0.insert(k.into(), v.serialize(Observer)?).is_none());
                Ok(())
            }
            fn end(self) -> Result<Value> {
                Ok(wrap(self.1, Value::Object(self.0)))
            }
        }
    };
}
fields!(SerializeStruct);
fields!(SerializeStructVariant);

#[test]
fn observer_keeps_tuple_keys_order_and_finite_float_bits() {
    struct Rows;
    impl Serialize for Rows {
        fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
            let mut m = s.serialize_map(Some(2))?;
            m.serialize_entry(&(9u32, 2u32), &-0.0f32)?;
            m.serialize_entry(&(1u32, 7u32), &f32::from_bits(1))?;
            m.end()
        }
    }
    assert!(
        serde_json::to_value(Rows).is_err(),
        "JSON object cannot preserve tuple keys"
    );
    let rows = capture(&Rows);
    assert_eq!(rows[0][0], serde_json::json!([9, 2]));
    assert_eq!(rows[1][0], serde_json::json!([1, 7]));
    assert_eq!((rows[0][1].as_f64().unwrap() as f32).to_bits(), 0x8000_0000);
    assert_eq!((rows[1][1].as_f64().unwrap() as f32).to_bits(), 1);
}

#[test]
fn exact_u128_observation_is_not_limited_to_json_integer_range() {
    assert_eq!(capture(&u128::MAX), Value::String(u128::MAX.to_string()));
}
