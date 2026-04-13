use std::fmt::Write;

/// Decode raw protobuf wire-format without a schema.
/// Produces a human-readable field listing like:
///   Field 1 (varint): 150
///   Field 2 (length-delimited): 13 bytes
pub fn decode_wire_format(data: &[u8]) -> String {
    let mut output = String::new();
    let mut pos = 0;

    while pos < data.len() {
        match decode_field(data, &mut pos) {
            Ok(field) => {
                let _ = writeln!(
                    output,
                    "Field {} ({}): {}",
                    field.number, field.wire_type, field.value
                );
            }
            Err(e) => {
                let _ = writeln!(output, "[Error at byte {pos}]: {e}");
                // Show remaining bytes as hex
                if pos < data.len() {
                    let _ = writeln!(
                        output,
                        "Remaining {} bytes: {}",
                        data.len() - pos,
                        hex_preview(&data[pos..])
                    );
                }
                break;
            }
        }
    }

    if output.is_empty() {
        output.push_str("(empty message)");
    }

    output
}

struct WireField {
    number: u32,
    wire_type: &'static str,
    value: String,
}

fn decode_field(data: &[u8], pos: &mut usize) -> Result<WireField, &'static str> {
    let tag = decode_varint(data, pos)?;
    let field_number = (tag >> 3) as u32;
    let wire_type = (tag & 0x07) as u8;

    if field_number == 0 {
        return Err("invalid field number 0");
    }

    let (type_name, value) = match wire_type {
        0 => {
            let v = decode_varint(data, pos)?;
            ("varint", format!("{v}"))
        }
        1 => {
            if *pos + 8 > data.len() {
                return Err("truncated 64-bit value");
            }
            let bytes: [u8; 8] = data[*pos..*pos + 8].try_into().unwrap();
            *pos += 8;
            let v = u64::from_le_bytes(bytes);
            ("64-bit", format!("{v} (0x{v:016x})"))
        }
        2 => {
            let len = decode_varint(data, pos)? as usize;
            if *pos + len > data.len() {
                return Err("truncated length-delimited value");
            }
            let bytes = &data[*pos..*pos + len];
            *pos += len;
            let value = format_length_delimited(bytes);
            ("length-delimited", value)
        }
        5 => {
            if *pos + 4 > data.len() {
                return Err("truncated 32-bit value");
            }
            let bytes: [u8; 4] = data[*pos..*pos + 4].try_into().unwrap();
            *pos += 4;
            let v = u32::from_le_bytes(bytes);
            ("32-bit", format!("{v} (0x{v:08x})"))
        }
        _ => return Err("unknown wire type"),
    };

    Ok(WireField {
        number: field_number,
        wire_type: type_name,
        value,
    })
}

fn decode_varint(data: &[u8], pos: &mut usize) -> Result<u64, &'static str> {
    let mut result: u64 = 0;
    let mut shift = 0;

    loop {
        if *pos >= data.len() {
            return Err("truncated varint");
        }
        let byte = data[*pos];
        *pos += 1;

        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err("varint too long");
        }
    }
}

/// Try to display length-delimited data as UTF-8 string, or fall back to hex.
fn format_length_delimited(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes)
        && s.chars()
            .all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t')
    {
        return format!("\"{s}\"");
    }
    format!("{} bytes: {}", bytes.len(), hex_preview(bytes))
}

fn hex_preview(bytes: &[u8]) -> String {
    let preview: Vec<String> = bytes.iter().take(32).map(|b| format!("{b:02x}")).collect();
    let mut s = preview.join(" ");
    if bytes.len() > 32 {
        s.push_str("...");
    }
    s
}
