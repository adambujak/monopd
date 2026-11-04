//! Minimal JSON output, so the bindings need no dependencies.
//!
//! `obj! { "key": value, ... }` builds an object from anything `ToJson`;
//! nested objects and arrays are `Raw` JSON text.

/// Already-encoded JSON.
pub struct Raw(pub String);

pub trait ToJson {
    fn to_json(&self) -> String;
}

macro_rules! plain {
    ($($t:ty),*) => { $(impl ToJson for $t { fn to_json(&self) -> String { self.to_string() } })* };
}
plain!(u8, u32, usize, i32, bool);

impl ToJson for str {
    fn to_json(&self) -> String {
        let mut out = String::with_capacity(self.len() + 2);
        out.push('"');
        for ch in self.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }
}

impl ToJson for String {
    fn to_json(&self) -> String {
        self.as_str().to_json()
    }
}

impl ToJson for Raw {
    fn to_json(&self) -> String {
        self.0.clone()
    }
}

impl<T: ToJson + ?Sized> ToJson for &T {
    fn to_json(&self) -> String {
        (**self).to_json()
    }
}

impl<T: ToJson> ToJson for Option<T> {
    fn to_json(&self) -> String {
        self.as_ref().map_or_else(|| "null".to_string(), ToJson::to_json)
    }
}

impl<T: ToJson> ToJson for [T] {
    fn to_json(&self) -> String {
        let items: Vec<String> = self.iter().map(ToJson::to_json).collect();
        format!("[{}]", items.join(","))
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self) -> String {
        self.as_slice().to_json()
    }
}

macro_rules! obj {
    ($($key:literal : $value:expr),+ $(,)?) => {
        $crate::json::Raw(format!(
            "{{{}}}",
            [$(format!("\"{}\":{}", $key, $crate::json::ToJson::to_json(&$value))),+].join(",")
        ))
    };
}
pub(crate) use obj;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_values_and_nesting() {
        let inner = obj! { "a": 1u8, "b": None::<u32> };
        let o = obj! { "s": "say \"hi\"\n", "list": vec![1u8, 2], "inner": inner, "t": true };
        assert_eq!(o.0, r#"{"s":"say \"hi\"\n","list":[1,2],"inner":{"a":1,"b":null},"t":true}"#);
    }
}
