use log::kv::value::Error as ValueError;
use slog::{Record, Serializer, KV};

use std::fmt::Arguments;

struct Visitor<'s> {
    serializer: &'s mut dyn Serializer,
}

impl<'s> Visitor<'s> {
    pub fn new(serializer: &'s mut dyn Serializer) -> Self {
        Self { serializer }
    }
}

pub(crate) struct SourceKV<'kvs>(pub &'kvs dyn log::kv::source::Source);

struct KeyVisit<'s> {
    serializer: &'s mut dyn Serializer,
    key: &'s str,
}

impl<'kvs> log::kv::Visitor<'kvs> for Visitor<'kvs> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        val: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        val.visit(KeyVisit {
            serializer: self.serializer,
            key: key.as_str(),
        })
    }
}

macro_rules! visit_to_emit {
    ($t:ty : $vname:ident -> $ename:ident) => {
        fn $vname(&mut self, value: $t) -> Result<(), ValueError> {
            self.serializer
                .$ename(self.key.to_string().into(), value)
                .map_err(to_value_err)
        }
    };
}

impl<'s> log::kv::value::Visit<'s> for KeyVisit<'s> {
    fn visit_any(&mut self, value: log::kv::Value<'_>) -> Result<(), ValueError> {
        let key = self.key.to_string().into();
        self.serializer
            .emit_arguments(key, &format_args!("{}", value))
            .map_err(to_value_err)
    }

    visit_to_emit!(u64: visit_u64 -> emit_u64);
    visit_to_emit!(i64: visit_i64 -> emit_i64);
    visit_to_emit!(u128: visit_u128 -> emit_u128);
    visit_to_emit!(i128: visit_i128 -> emit_i128);
    visit_to_emit!(f64: visit_f64 -> emit_f64);
    visit_to_emit!(bool: visit_bool -> emit_bool);
    visit_to_emit!(&str: visit_str -> emit_str);
    visit_to_emit!(char: visit_char -> emit_char);
    visit_to_emit!(&(dyn std::error::Error + 'static): visit_error -> emit_error);
}

impl KV for SourceKV<'_> {
    fn serialize(&self, _record: &Record, serializer: &mut dyn Serializer) -> slog::Result {
        // Unfortunately, there isn't  a way for use to pass the original error through.
        self.0
            .visit(&mut Visitor::new(serializer))
            .map_err(|_| slog::Error::Other)
    }
}

fn to_value_err(err: slog::Error) -> ValueError {
    use slog::Error::*;

    match err {
        Io(e) => e.into(),
        Fmt(e) => e.into(),
        Other => ValueError::boxed(err),
    }
}

/// Create a [`log::kv::Source`] for the key-value pairs for a slog record.
pub(crate) fn get_kv_source<'a>(
    record: &'a slog::Record<'a>,
    logger_kv: &'a slog::OwnedKVList,
) -> std::io::Result<Vec<(String, OwnedValue)>> {
    let mut serialized_source = LogSerializer(vec![]);

    record.kv().serialize(record, &mut serialized_source)?;
    logger_kv.serialize(record, &mut serialized_source)?;
    Ok(serialized_source.0)
}

/// A wrapper around [`log::kv::Value`], that owns the data included.
///
/// In particular this is necessary for strings, and large integers (u128, and i128), because the
/// `Value` type itself only supports references, which must survive for the lifetime of the
/// visitor.
pub(crate) enum OwnedValue {
    Value(log::kv::Value<'static>),
    Str(String),
    U128(Box<u128>),
    I128(Box<i128>),
}

impl log::kv::value::ToValue for OwnedValue {
    fn to_value(&self) -> log::kv::Value<'_> {
        use OwnedValue::*;

        match self {
            Value(v) => v.to_value(),
            Str(s) => s.to_value(),
            U128(v) => v.to_value(),
            I128(v) => v.to_value(),
        }
    }
}

struct LogSerializer(Vec<(String, OwnedValue)>);

impl LogSerializer {
    fn add(&mut self, key: slog::Key, val: OwnedValue) -> slog::Result {
        self.0.push((key.into(), val));
        Ok(())
    }
}

macro_rules! emit_to_value {
    ($f:ident : $t:ty) => {
        fn $f(&mut self, key: slog::Key, val: $t) -> slog::Result {
            self.add(key, OwnedValue::Value(val.into()))
        }
    };
}

impl Serializer for LogSerializer {
    fn emit_arguments(&mut self, key: slog::Key, val: &Arguments<'_>) -> slog::Result {
        self.add(key, OwnedValue::Str(val.to_string()))
    }

    emit_to_value!(emit_usize: usize);
    emit_to_value!(emit_isize: isize);
    emit_to_value!(emit_bool: bool);
    emit_to_value!(emit_char: char);
    emit_to_value!(emit_u8: u8);
    emit_to_value!(emit_i8: i8);
    emit_to_value!(emit_u16: u16);
    emit_to_value!(emit_i16: i16);
    emit_to_value!(emit_u32: u32);
    emit_to_value!(emit_i32: i32);
    emit_to_value!(emit_f32: f32);
    emit_to_value!(emit_f64: f64);

    fn emit_u128(&mut self, key: slog::Key, val: u128) -> slog::Result {
        self.add(key, OwnedValue::U128(Box::new(val)))
    }

    fn emit_i128(&mut self, key: slog::Key, val: i128) -> slog::Result {
        self.add(key, OwnedValue::I128(Box::new(val)))
    }

    fn emit_str(&mut self, key: slog::Key, val: &str) -> slog::Result {
        self.add(key, OwnedValue::Str(val.to_string()))
    }

    fn emit_unit(&mut self, key: slog::Key) -> slog::Result {
        use log::kv::ToValue;
        self.add(key, OwnedValue::Value(().to_value()))
    }

    fn emit_none(&mut self, key: slog::Key) -> slog::Result {
        self.emit_unit(key)
    }
}
