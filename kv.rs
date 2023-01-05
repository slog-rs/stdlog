use log::kv::value::Error as ValueError;
use slog::{Record, Serializer};

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

impl slog::KV for SourceKV<'_> {
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

// TODO: support going the other way
