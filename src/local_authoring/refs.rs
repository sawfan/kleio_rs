use super::LocalAuthoringError;

pub(super) fn normalize_contextual_id(value: &str, default_prefix: &str) -> String {
    let value = value.trim();
    if value.contains(':') {
        value.to_string()
    } else {
        format!("{default_prefix}:{value}")
    }
}

pub(super) fn normalize_person_id(value: &str) -> String {
    normalize_contextual_id(value, "person")
}

pub(super) fn normalize_place_id(value: &str) -> String {
    normalize_contextual_id(value, "place")
}

pub(super) fn normalize_source_id(value: &str) -> String {
    normalize_contextual_id(value, "source")
}

pub(super) fn resolve_contextual_id(value: &str, normalize: fn(&str) -> String) -> String {
    if value.contains(':') {
        value.trim().to_string()
    } else {
        normalize(value)
    }
}

pub(super) fn validate_contextual_id(value: &str, label: &str) -> Result<(), LocalAuthoringError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} cannot be empty"),
        });
    }

    if trimmed.contains(':') {
        let Some((prefix, local)) = trimmed.split_once(':') else {
            return Err(LocalAuthoringError::Validation {
                message: format!("{label} `{value}` should be a stable id such as person:example"),
            });
        };
        validate_slug_part(prefix, label, value)?;
        validate_slug_part(local, label, value)?;
    } else {
        validate_slug_part(trimmed, label, value)?;
    }

    Ok(())
}

fn validate_slug_part(part: &str, label: &str, value: &str) -> Result<(), LocalAuthoringError> {
    if part.trim().is_empty()
        || part
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '/' | '\\' | ':'))
    {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{label} `{value}` may not contain whitespace, slashes, or empty id parts"
            ),
        });
    }

    Ok(())
}
