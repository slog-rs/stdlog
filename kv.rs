use slog::{Level, Record, Serializer, KV};

pub(crate) struct Visitor {
    kvs: Vec<(String, String)>,
}

impl Visitor {
    pub fn new() -> Self {
        Self { kvs: vec![] }
    }
}

impl<'kvs, 'a> log::kv::Visitor<'kvs> for Visitor {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        val: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let key = key.to_string();
        if let Some(val) = val.to_borrowed_str() {
            let val = val.to_string();
            self.kvs.push((key, val));
        }
        Ok(())
    }
}

fn string_to_static_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

impl slog::KV for Visitor {
    fn serialize(&self, _record: &Record, serializer: &mut dyn Serializer) -> slog::Result {
        for (key, val) in &self.kvs {
            let key = string_to_static_str(key.to_owned());
            serializer.emit_str(key, val.as_str())?;
        }
        Ok(())
    }
}