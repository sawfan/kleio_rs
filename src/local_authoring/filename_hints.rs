use std::path::Path;

use super::{LocalAuthoringError, refs::validate_contextual_id};

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct EventFilenameHints {
    pub event_type: Option<String>,
    pub participant: Option<String>,
    pub time: Option<String>,
    pub time_basis: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

pub(super) fn event_filename_hints(
    relative_path: &Path,
) -> Result<EventFilenameHints, LocalAuthoringError> {
    let path = relative_path.to_string_lossy();
    if !path.starts_with("events/") {
        return Ok(EventFilenameHints::default());
    }

    let Some(stem) = relative_path.file_stem().and_then(|stem| stem.to_str()) else {
        return Ok(EventFilenameHints::default());
    };

    parse_event_filename_hints(stem).map_err(|message| LocalAuthoringError::Validation {
        message: format!("{}: {message}", relative_path.display()),
    })
}

fn parse_event_filename_hints(stem: &str) -> Result<EventFilenameHints, String> {
    let mut parts = stem.split("--");
    let Some(first) = parts.next() else {
        return Ok(EventFilenameHints::default());
    };
    let mut hints = EventFilenameHints {
        event_type: Some(first.to_string()).filter(|value| !value.trim().is_empty()),
        ..EventFilenameHints::default()
    };

    for part in parts {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let value = decode_filename_value(value);
        match key {
            "person" | "participant" => {
                validate_contextual_id(&value, "filename participant")
                    .map_err(|err| err.to_string())?;
                hints.participant = Some(value);
            }
            "local" | "dt" => {
                hints.time = Some(parse_filename_datetime(&value));
                hints.time_basis = Some("local".to_string());
            }
            "utc" => {
                hints.time = Some(parse_filename_datetime(&value));
                hints.time_basis = Some("utc".to_string());
            }
            "unix" => {
                if value.parse::<i64>().is_err() {
                    return Err(format!("filename unix value `{value}` must be an integer"));
                }
                hints.time = Some(value);
                hints.time_basis = Some("unix".to_string());
            }
            "lat" | "latitude" => {
                hints.latitude = Some(parse_f64(&value, key)?);
            }
            "lng" | "lon" | "longitude" => {
                hints.longitude = Some(parse_f64(&value, key)?);
            }
            _ => {}
        }
    }

    if hints.latitude.is_some() != hints.longitude.is_some() {
        return Err("filename latitude and longitude must be provided together".to_string());
    }

    Ok(hints)
}

fn parse_f64(value: &str, key: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("filename {key} value `{value}` must be a number"))
}

fn parse_filename_datetime(value: &str) -> String {
    let Some((date, time)) = value.split_once('T') else {
        return value.to_string();
    };
    let has_utc_suffix = time.ends_with('Z');
    let time = time.trim_end_matches('Z').replace('-', ":");
    if has_utc_suffix {
        format!("{date} {time}Z")
    } else {
        format!("{date} {time}")
    }
}

fn decode_filename_value(value: &str) -> String {
    value.replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_birth_filename_hints() {
        let hints = parse_event_filename_hints(
            "birth--person=alex-example--local=1900-01-01T12-04--lat=40.7128--lng=-74.0060",
        )
        .expect("hints");

        assert_eq!(hints.event_type.as_deref(), Some("birth"));
        assert_eq!(hints.participant.as_deref(), Some("alex-example"));
        assert_eq!(hints.time.as_deref(), Some("1900-01-01 12:04"));
        assert_eq!(hints.time_basis.as_deref(), Some("local"));
        assert_eq!(hints.latitude, Some(40.7128));
        assert_eq!(hints.longitude, Some(-74.0060));
    }
}
