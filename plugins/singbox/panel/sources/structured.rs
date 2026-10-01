use super::parse::{ImportError, MAX_BODY, MAX_DEPTH, MAX_SCALAR};
use saphyr_parser::{Event, Parser, ScalarStyle};
use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, fmt};

const MAX_VALUES: usize = 100_000;

#[derive(Default)]
struct Budget {
    values: usize,
    bytes: usize,
}

impl Budget {
    fn charge(&mut self, depth: usize, bytes: usize) -> Result<(), ImportError> {
        self.values += 1;
        self.bytes = self.bytes.saturating_add(bytes);
        if depth > MAX_DEPTH || self.values > MAX_VALUES || self.bytes > MAX_BODY {
            Err(ImportError("structure_limit"))
        } else {
            Ok(())
        }
    }

    fn duplicate(&mut self, value: &Value, depth: usize) -> Result<(), ImportError> {
        self.charge(depth, value.as_str().map_or(0, str::len))?;
        match value {
            Value::Array(values) => {
                for value in values {
                    self.duplicate(value, depth + 1)?;
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    self.charge(depth + 1, key.len())?;
                    self.duplicate(value, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) fn json(text: &str) -> Result<Value, ImportError> {
    let mut budget = Budget::default();
    let mut decoder = serde_json::Deserializer::from_str(text);
    let value = Seed {
        budget: &mut budget,
        depth: 0,
    }
    .deserialize(&mut decoder)
    .map_err(|_| ImportError("invalid_json_or_limit"))?;
    decoder
        .end()
        .map_err(|_| ImportError("invalid_json_or_limit"))?;
    Ok(value)
}

struct Seed<'a> {
    budget: &'a mut Budget,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        self.budget
            .charge(self.depth, 0)
            .map_err(D::Error::custom)?;
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bounded configuration")
    }
    fn visit_bool<E: Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: Error>(self, value: i64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_u64<E: Error>(self, value: u64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_f64<E: Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("invalid number"))
    }
    fn visit_unit<E: Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<Value, E> {
        if value.len() > MAX_SCALAR {
            return Err(E::custom("scalar limit"));
        }
        self.budget
            .charge(self.depth, value.len())
            .map_err(E::custom)?;
        Ok(Value::String(value.into()))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element_seed(Seed {
            budget: self.budget,
            depth: self.depth + 1,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if key.len() > MAX_SCALAR || values.contains_key(&key) {
                return Err(A::Error::custom("duplicate or oversized key"));
            }
            self.budget
                .charge(self.depth + 1, key.len())
                .map_err(A::Error::custom)?;
            let value = access.next_value_seed(Seed {
                budget: self.budget,
                depth: self.depth + 1,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

struct Frame {
    value: Value,
    key: Option<String>,
    anchor: usize,
}

fn push_value(
    frames: &mut [Frame],
    root: &mut Option<Value>,
    value: Value,
) -> Result<(), ImportError> {
    if let Some(frame) = frames.last_mut() {
        match &mut frame.value {
            Value::Array(values) => values.push(value),
            Value::Object(values) => {
                if let Some(key) = frame.key.take() {
                    if values.insert(key, value).is_some() {
                        return Err(ImportError("duplicate_yaml_key"));
                    }
                } else {
                    frame.key = Some(
                        value
                            .as_str()
                            .ok_or(ImportError("invalid_yaml_key"))?
                            .to_owned(),
                    );
                }
            }
            _ => return Err(ImportError("invalid_yaml")),
        }
    } else if root.replace(value).is_some() {
        return Err(ImportError("multiple_yaml_documents"));
    }
    Ok(())
}

fn merge_keys(value: &mut Value, budget: &mut Budget, depth: usize) -> Result<(), ImportError> {
    let Some(map) = value.as_object_mut() else {
        return Ok(());
    };
    if let Some(merge) = map.remove("<<") {
        let maps = match merge {
            Value::Array(values) => values,
            value => vec![value],
        };
        for value in maps {
            let source = value.as_object().ok_or(ImportError("invalid_yaml_merge"))?;
            for (key, value) in source {
                if !map.contains_key(key) {
                    budget.charge(depth + 1, key.len())?;
                    budget.duplicate(value, depth + 1)?;
                    map.insert(key.clone(), value.clone());
                }
            }
        }
    }
    Ok(())
}

pub(super) fn yaml(text: &str) -> Result<Value, ImportError> {
    let mut frames: Vec<Frame> = Vec::new();
    let mut anchors = BTreeMap::new();
    let mut root = None;
    let mut budget = Budget::default();
    let mut documents = 0;
    for item in Parser::new_from_str(text) {
        let (event, _) = item.map_err(|_| ImportError("invalid_yaml"))?;
        match event {
            Event::DocumentStart(_) => {
                documents += 1;
                if documents > 1 {
                    return Err(ImportError("multiple_yaml_documents"));
                }
            }
            Event::Scalar(value, style, anchor, tag) => {
                if value.len() > MAX_SCALAR {
                    return Err(ImportError("scalar_limit"));
                }
                budget.charge(frames.len(), value.len())?;
                let scalar = match tag.as_deref() {
                    Some(tag) if tag.is_yaml_core_schema() && tag.suffix == "str" => {
                        Value::String(value.into_owned())
                    }
                    Some(_) => return Err(ImportError("unsupported_yaml_tag")),
                    None if style == ScalarStyle::Plain => scalar(&value)?,
                    None => Value::String(value.into_owned()),
                };
                if anchor != 0 {
                    budget.duplicate(&scalar, 0)?;
                    anchors.insert(anchor, scalar.clone());
                }
                push_value(&mut frames, &mut root, scalar)?;
            }
            Event::SequenceStart(anchor, ref tag) | Event::MappingStart(anchor, ref tag) => {
                if tag.is_some() {
                    return Err(ImportError("unsupported_yaml_tag"));
                }
                budget.charge(frames.len() + 1, 0)?;
                let value = if matches!(event, Event::SequenceStart(..)) {
                    Value::Array(Vec::new())
                } else {
                    Value::Object(Map::new())
                };
                frames.push(Frame {
                    value,
                    key: None,
                    anchor,
                });
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let mut frame = frames.pop().ok_or(ImportError("invalid_yaml"))?;
                if frame.key.is_some() {
                    return Err(ImportError("invalid_yaml"));
                }
                merge_keys(&mut frame.value, &mut budget, frames.len())?;
                if frame.anchor != 0 {
                    budget.duplicate(&frame.value, 0)?;
                    anchors.insert(frame.anchor, frame.value.clone());
                }
                push_value(&mut frames, &mut root, frame.value)?;
            }
            Event::Alias(anchor) => {
                let value = anchors
                    .get(&anchor)
                    .ok_or(ImportError("recursive_or_unknown_yaml_alias"))?;
                budget.duplicate(value, frames.len())?;
                push_value(&mut frames, &mut root, value.clone())?;
            }
            _ => {}
        }
    }
    if !frames.is_empty() {
        return Err(ImportError("invalid_yaml"));
    }
    root.ok_or(ImportError("empty_configuration"))
}

fn scalar(value: &str) -> Result<Value, ImportError> {
    match value {
        "null" | "Null" | "NULL" | "~" | "" => Ok(Value::Null),
        "true" | "True" | "TRUE" => Ok(Value::Bool(true)),
        "false" | "False" | "FALSE" => Ok(Value::Bool(false)),
        _ => {
            if let Ok(value) = value.parse::<i64>() {
                return Ok(value.into());
            }
            if let Ok(value) = value.parse::<u64>() {
                return Ok(value.into());
            }
            if let Ok(value) = value.parse::<f64>() {
                return serde_json::Number::from_f64(value)
                    .map(Value::Number)
                    .ok_or(ImportError("invalid_yaml_number"));
            }
            Ok(Value::String(value.into()))
        }
    }
}
