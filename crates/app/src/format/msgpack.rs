use std::io::Cursor;

use rmpv::{Value, decode::read_value_with_max_depth};

const MAX_DEPTH: usize = 100;

pub fn decode_value(data: &[u8]) -> Result<Value, String> {
    if data.is_empty() {
        return Err("empty payload".to_string());
    }

    let mut cursor = Cursor::new(data);
    let value = read_value_with_max_depth(&mut cursor, MAX_DEPTH)
        .map_err(|error| format!("decode failed: {error}"))?;
    let consumed = cursor.position() as usize;
    if consumed != data.len() {
        return Err(format!("{} trailing byte(s)", data.len() - consumed));
    }
    Ok(value)
}

pub fn is_confident_auto(data: &[u8]) -> bool {
    matches!(
        decode_value(data),
        Ok(Value::Map(_) | Value::Array(_) | Value::Binary(_) | Value::Ext(_, _))
    )
}

pub fn format_msgpack(data: &[u8]) -> String {
    match decode_value(data) {
        Ok(value) => render_value(&value, 0),
        Err(error) => format!("MsgPack decode error: {error}"),
    }
}

fn render_value(value: &Value, indent: usize) -> String {
    match value {
        Value::Nil => "nil".to_string(),
        Value::Boolean(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::F32(value) => value.to_string(),
        Value::F64(value) => value.to_string(),
        Value::String(value) => render_string(value),
        Value::Binary(value) => format!("bin({}): {}", value.len(), hex_bytes(value)),
        Value::Array(values) => render_array(values, indent),
        Value::Map(values) => render_map(values, indent),
        Value::Ext(ty, value) => format!(
            "ext(type: {ty}, len: {}): {}",
            value.len(),
            hex_bytes(value)
        ),
    }
}

fn render_array(values: &[Value], indent: usize) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }

    let child_indent = indent + 2;
    let child_padding = " ".repeat(child_indent);
    let mut out = String::from("[\n");
    for (index, value) in values.iter().enumerate() {
        out.push_str(&child_padding);
        out.push_str(&indent_multiline(
            &render_value(value, child_indent),
            child_indent,
        ));
        if index + 1 < values.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&" ".repeat(indent));
    out.push(']');
    out
}

fn render_map(values: &[(Value, Value)], indent: usize) -> String {
    if values.is_empty() {
        return "{}".to_string();
    }

    let child_indent = indent + 2;
    let child_padding = " ".repeat(child_indent);
    let mut out = String::from("{\n");
    for (key, value) in values {
        out.push_str(&child_padding);
        out.push_str(&render_key(key));
        out.push_str(": ");
        out.push_str(&indent_multiline(
            &render_value(value, child_indent),
            child_indent,
        ));
        out.push('\n');
    }
    out.push_str(&" ".repeat(indent));
    out.push('}');
    out
}

fn render_key(value: &Value) -> String {
    match value {
        Value::String(value) => render_string(value),
        other => format!("[{}]", render_compact(other)),
    }
}

fn render_compact(value: &Value) -> String {
    match value {
        Value::Nil => "nil".to_string(),
        Value::Boolean(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::F32(value) => value.to_string(),
        Value::F64(value) => value.to_string(),
        Value::String(value) => render_string(value),
        Value::Binary(value) => format!("bin({}):{}", value.len(), hex_bytes(value)),
        Value::Ext(ty, value) => format!("ext(type:{ty},len:{}):{}", value.len(), hex_bytes(value)),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(render_compact)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Map(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("{}: {}", render_key(key), render_compact(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_string(value: &rmpv::Utf8String) -> String {
    if let Some(value) = value.as_str() {
        serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
    } else {
        format!(
            "str(invalid utf-8, len: {}): {}",
            value.as_bytes().len(),
            hex_bytes(value.as_bytes())
        )
    }
}

fn indent_multiline(value: &str, indent: usize) -> String {
    let padding = " ".repeat(indent);
    value.replace('\n', &format!("\n{padding}"))
}

fn hex_bytes(value: &[u8]) -> String {
    value
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
