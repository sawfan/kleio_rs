use super::{
    LocalAuthoringError, LocalMarkdownRecord,
    event_profiles::{LocalEventType, local_event_type},
    refs::normalize_place_id,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct LocalLocationRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<bool>,
}

pub(super) fn event_location_refs(record: &LocalMarkdownRecord) -> Vec<LocalLocationRef> {
    let default_role = local_event_type(record).and_then(LocalEventType::default_location_role);
    let mut locations: Vec<LocalLocationRef> = record
        .attributes
        .get("places")
        .and_then(serde_json::Value::as_array)
        .map(|places| {
            places
                .iter()
                .filter_map(|place| normalize_place_ref(place, default_role))
                .collect()
        })
        .unwrap_or_default();

    if let Some(location) = record
        .attributes
        .get("location")
        .and_then(inline_location_ref_from_value)
    {
        locations.push(location);
    }

    if let Some(inline_locations) = record.attributes.get("locations") {
        match inline_locations {
            serde_json::Value::Array(values) => {
                locations.extend(values.iter().filter_map(inline_location_ref_from_value));
            }
            value => {
                if let Some(location) = inline_location_ref_from_value(value) {
                    locations.push(location);
                }
            }
        }
    }

    assign_generated_location_entities(record, default_role, &mut locations);

    locations
}

fn assign_generated_location_entities(
    record: &LocalMarkdownRecord,
    default_role: Option<&str>,
    locations: &mut [LocalLocationRef],
) {
    let generated_count = locations
        .iter()
        .filter(|location| location.generated == Some(true))
        .count();
    let mut generated_index = 0;

    for location in locations {
        if location.generated != Some(true) {
            continue;
        }

        generated_index += 1;
        if location.entity.is_none() {
            let suffix = if generated_count > 1 {
                format!("location-{generated_index}")
            } else {
                "location".to_string()
            };
            location.entity = Some(format!(
                "place:inline:{}-{suffix}",
                safe_generated_id_fragment(&record.id)
            ));
        }
        if location.role.is_none() {
            location.role = default_role.map(ToOwned::to_owned);
        }
    }
}

fn safe_generated_id_fragment(value: &str) -> String {
    let mut output = String::new();
    let mut last_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_dash = false;
        } else if !last_dash {
            output.push('-');
            last_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}

pub(super) fn event_locations(record: &LocalMarkdownRecord) -> Vec<serde_json::Value> {
    event_location_refs(record)
        .into_iter()
        .filter_map(|location| serde_json::to_value(location).ok())
        .collect()
}

pub(super) fn validate_inline_location_value(
    path: &str,
    value: &serde_json::Value,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    match value {
        serde_json::Value::String(value) => validate_non_empty_string(path, value, field),
        serde_json::Value::Object(location) => {
            validate_inline_location_object(path, location, field)
        }
        serde_json::Value::Array(values) if field == "locations" => {
            for value in values {
                validate_inline_location_value(path, value, field)?;
            }
            Ok(())
        }
        _ => Err(LocalAuthoringError::Validation {
            message: format!(
                "{path} `{field}` must be a location string, inline location table, or array of inline locations"
            ),
        }),
    }
}

pub(super) fn validate_place_item(
    path: &str,
    item: &serde_json::Value,
    ids: &std::collections::BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let Some(item) = item.as_object() else {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{path} `places` entries must be place references or inline location tables"
            ),
        });
    };

    if let Some(entity_id) = item.get("entity").and_then(serde_json::Value::as_str) {
        let entity_id = normalize_place_entity_id(entity_id);
        if !ids.contains(&entity_id) {
            return Err(LocalAuthoringError::Validation {
                message: format!("{path} references missing places entity `{entity_id}`"),
            });
        }
        return Ok(());
    }

    validate_inline_location_object(path, item, "places")
}

pub(super) fn normalize_place_entity_id(entity: &str) -> String {
    normalize_place_id(entity)
}

fn normalize_place_ref(
    value: &serde_json::Value,
    default_role: Option<&str>,
) -> Option<LocalLocationRef> {
    match value {
        serde_json::Value::String(entity) if !entity.trim().is_empty() => Some(LocalLocationRef {
            entity: Some(normalize_place_entity_id(entity)),
            role: default_role.map(ToOwned::to_owned),
            label: None,
            name: None,
            source_text: None,
            latitude: None,
            longitude: None,
            generated: None,
        }),
        serde_json::Value::Object(values) => {
            if let Some(entity) = values.get("entity").and_then(serde_json::Value::as_str) {
                return Some(LocalLocationRef {
                    entity: Some(normalize_place_entity_id(entity)),
                    role: values
                        .get("role")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| default_role.map(ToOwned::to_owned)),
                    label: None,
                    name: None,
                    source_text: None,
                    latitude: None,
                    longitude: None,
                    generated: None,
                });
            }
            inline_location_ref_from_object(values)
        }
        _ => None,
    }
}

fn inline_location_ref_from_value(value: &serde_json::Value) -> Option<LocalLocationRef> {
    match value {
        serde_json::Value::String(label) if !label.trim().is_empty() => Some(LocalLocationRef {
            entity: None,
            role: None,
            label: Some(label.clone()),
            name: None,
            source_text: None,
            latitude: None,
            longitude: None,
            generated: Some(true),
        }),
        serde_json::Value::Object(values) => inline_location_ref_from_object(values),
        _ => None,
    }
}

fn inline_location_ref_from_object(
    values: &serde_json::Map<String, serde_json::Value>,
) -> Option<LocalLocationRef> {
    Some(LocalLocationRef {
        entity: None,
        role: None,
        label: values
            .get("label")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        name: values
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        source_text: values
            .get("source_text")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        latitude: values.get("latitude").and_then(json_number),
        longitude: values.get("longitude").and_then(json_number),
        generated: Some(true),
    })
}

fn validate_inline_location_object(
    path: &str,
    location: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    let has_text = ["label", "name", "source_text"].into_iter().any(|key| {
        location
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    });
    let has_latitude = location.get("latitude").is_some_and(is_json_number);
    let has_longitude = location.get("longitude").is_some_and(is_json_number);

    if !has_text && !(has_latitude && has_longitude) {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{path} inline `{field}` location must include `label`, `name`, `source_text`, or both `latitude` and `longitude`"
            ),
        });
    }

    if location.contains_key("latitude") != location.contains_key("longitude") {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{path} inline `{field}` location must include both `latitude` and `longitude` when using coordinates"
            ),
        });
    }

    for key in ["latitude", "longitude"] {
        if let Some(value) = location.get(key)
            && !is_json_number(value)
        {
            return Err(LocalAuthoringError::Validation {
                message: format!("{path} inline `{field}` `{key}` must be a number"),
            });
        }
    }

    Ok(())
}

fn validate_non_empty_string(
    path: &str,
    value: &str,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    if value.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{path} `{field}` cannot be empty"),
        });
    }
    Ok(())
}

fn json_number(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))
        .or_else(|| value.as_u64().map(|value| value as f64))
}

fn is_json_number(value: &serde_json::Value) -> bool {
    json_number(value).is_some()
}
