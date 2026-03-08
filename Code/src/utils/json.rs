//! JSON utilities
//!
//! Simple JSON helpers without external dependencies.

/// Escape a string for JSON
pub fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for ch in s.chars() {
        match ch {
            '"' => result.push_str(r#"\""#),
            '\\' => result.push_str(r"\\"),
            '\n' => result.push_str(r"\n"),
            '\r' => result.push_str(r"\r"),
            '\t' => result.push_str(r"\t"),
            _ if ch.is_control() => {
                result.push_str(&format!(r"\u{:04x}", ch as u32));
            }
            _ => result.push(ch),
        }
    }

    result
}

/// Create a simple JSON object
pub fn json_object(fields: &[(&str, &str)]) -> String {
    if fields.is_empty() {
        return "{}".to_string();
    }

    let mut json = String::from("{");

    for (i, (key, value)) in fields.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(r#""{}":"{}""#, escape_string(key), escape_string(value)));
    }

    json.push('}');
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\"world"), r#"hello\"world"#);
    }
}