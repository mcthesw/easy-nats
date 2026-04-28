use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) struct JsonSchemaCatalog {
    pub(super) validators: HashMap<String, jsonschema::Validator>,
    pub(super) schemas: HashMap<String, serde_json::Value>,
}

impl JsonSchemaCatalog {
    pub(super) fn entries(&self) -> Vec<String> {
        let mut entries: Vec<String> = self.validators.keys().cloned().collect();
        entries.sort();
        entries
    }

    pub(super) fn validate(&self, entry: &str, value: &serde_json::Value) -> Result<(), String> {
        let validator = self
            .validators
            .get(entry)
            .ok_or_else(|| format!("Unknown JSON Schema entry: {entry}"))?;
        validator.validate(value).map_err(|error| error.to_string())
    }

    pub(super) fn template(&self, entry: &str) -> Result<String, String> {
        let schema = self
            .schemas
            .get(entry)
            .ok_or_else(|| format!("Unknown JSON Schema entry: {entry}"))?;
        let template = json_schema_template(schema);
        serde_json::to_string_pretty(&template)
            .map_err(|error| format!("JSON template serialization failed: {error}"))
    }
}

pub(super) fn load_json_schema_catalog(path: &Path) -> Result<JsonSchemaCatalog, String> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        collect_json_files(path, &mut files)
            .map_err(|e| format!("Failed to scan directory: {e}"))?;
    } else {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    if files.is_empty() {
        return Err(format!("No JSON schema files found in {}", path.display()));
    }

    let mut validators = HashMap::new();
    let mut schemas = HashMap::new();
    let mut used_entries = HashSet::new();
    for file in files {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read {}: {e}", file.display()))?;
        let schema = serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| format!("Failed to parse {}: {e}", file.display()))?;
        let validator = jsonschema::validator_for(&schema)
            .map_err(|e| format!("Failed to compile {}: {e}", file.display()))?;
        let mut entry = schema_entry_name(path, &file);
        if !used_entries.insert(entry.clone()) {
            entry = file.to_string_lossy().replace('\\', "/");
        }
        validators.insert(entry.clone(), validator);
        schemas.insert(entry, schema);
    }
    Ok(JsonSchemaCatalog {
        validators,
        schemas,
    })
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

pub(super) fn schema_entry_name(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .ok()
        .filter(|relative| relative.components().count() > 0)
        .unwrap_or(file)
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}

pub(super) fn json_schema_template(schema: &serde_json::Value) -> serde_json::Value {
    json_schema_template_inner(schema, schema, &mut HashSet::new())
}

fn json_schema_template_inner(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    seen_refs: &mut HashSet<String>,
) -> serde_json::Value {
    if let Some(reference) = schema["$ref"].as_str()
        && let Some(referenced_schema) = resolve_local_json_schema_ref(root, reference)
    {
        if !seen_refs.insert(reference.to_string()) {
            return serde_json::Value::Null;
        }
        let value = json_schema_template_inner(root, referenced_schema, seen_refs);
        seen_refs.remove(reference);
        return value;
    }

    for key in ["default", "const"] {
        if !schema[key].is_null() {
            return schema[key].clone();
        }
    }
    if let Some(example) = schema["examples"]
        .as_array()
        .and_then(|examples| examples.first())
    {
        return example.clone();
    }
    if let Some(value) = schema["enum"].as_array().and_then(|values| values.first()) {
        return value.clone();
    }
    if let Some(value) = json_schema_template_from_combinator(root, schema, seen_refs) {
        return value;
    }

    match json_schema_type(schema) {
        Some("object") => json_schema_object_template(root, schema, seen_refs),
        None if schema["properties"].is_object() => {
            json_schema_object_template(root, schema, seen_refs)
        }
        Some("array") => {
            let item = if schema["items"].is_object() {
                json_schema_template_inner(root, &schema["items"], seen_refs)
            } else {
                serde_json::Value::Null
            };
            serde_json::Value::Array(vec![item])
        }
        Some("integer") => schema["minimum"].clone().as_i64().map_or_else(
            || serde_json::json!(0),
            |minimum| serde_json::json!(minimum),
        ),
        Some("number") => schema["minimum"].clone().as_f64().map_or_else(
            || serde_json::json!(0.0),
            |minimum| serde_json::json!(minimum),
        ),
        Some("boolean") => serde_json::json!(false),
        Some("null") => serde_json::Value::Null,
        Some("string") => json_schema_string_template(schema),
        _ => json_schema_string_template(schema),
    }
}

fn json_schema_template_from_combinator(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    seen_refs: &mut HashSet<String>,
) -> Option<serde_json::Value> {
    for key in ["oneOf", "anyOf"] {
        if let Some(first) = schema[key].as_array().and_then(|schemas| schemas.first()) {
            return Some(json_schema_template_inner(root, first, seen_refs));
        }
    }

    let schemas = schema["allOf"].as_array()?;
    let mut merged = serde_json::Map::new();
    for item in schemas {
        match json_schema_template_inner(root, item, seen_refs) {
            serde_json::Value::Object(object) => merged.extend(object),
            value if merged.is_empty() => return Some(value),
            _ => {}
        }
    }
    Some(serde_json::Value::Object(merged))
}

fn json_schema_object_template(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    seen_refs: &mut HashSet<String>,
) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    let Some(properties) = schema["properties"].as_object() else {
        return serde_json::Value::Object(object);
    };

    let mut keys = Vec::new();
    if let Some(required) = schema["required"].as_array() {
        for key in required.iter().filter_map(|key| key.as_str()) {
            if properties.contains_key(key) {
                keys.push(key.to_string());
            }
        }
    }
    for key in properties.keys() {
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.clone());
        }
    }

    for key in keys {
        if let Some(property_schema) = properties.get(&key) {
            object.insert(
                key,
                json_schema_template_inner(root, property_schema, seen_refs),
            );
        }
    }
    serde_json::Value::Object(object)
}

fn json_schema_type(schema: &serde_json::Value) -> Option<&str> {
    if let Some(schema_type) = schema["type"].as_str() {
        return Some(schema_type);
    }
    schema["type"].as_array().and_then(|types| {
        types
            .iter()
            .filter_map(|schema_type| schema_type.as_str())
            .find(|schema_type| *schema_type != "null")
    })
}

fn json_schema_string_template(schema: &serde_json::Value) -> serde_json::Value {
    match schema["format"].as_str() {
        Some("date-time") => serde_json::json!("2026-01-01T00:00:00Z"),
        Some("date") => serde_json::json!("2026-01-01"),
        Some("email") => serde_json::json!("user@example.com"),
        Some("uri") | Some("url") => serde_json::json!("https://example.com"),
        _ => serde_json::json!(""),
    }
}

fn resolve_local_json_schema_ref<'a>(
    root: &'a serde_json::Value,
    reference: &str,
) -> Option<&'a serde_json::Value> {
    let pointer = reference.strip_prefix('#')?;
    root.pointer(pointer)
}
