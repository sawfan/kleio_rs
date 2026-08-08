use std::collections::BTreeSet;

use super::{LocalAuthoringError, LocalMarkdownRecord, refs::normalize_person_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalEventType<'a> {
    Birth,
    Death,
    Marriage,
    Baptism,
    Burial,
    Residence,
    Occupation,
    Divorce,
    Adoption,
    Education,
    MilitaryService,
    Immigration,
    Emigration,
    Naturalization,
    Census,
    NameChange,
    Custom(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct LocalParticipantRef {
    pub entity: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalEventProfile<'a> {
    event_type: &'a str,
    default_participant_role: Option<&'static str>,
    default_location_role: Option<&'static str>,
    label: LabelTemplate,
}

macro_rules! profile {
    ($event_type:expr, $participant_role:expr, $location_role:expr, $label:expr) => {
        LocalEventProfile {
            event_type: $event_type,
            default_participant_role: $participant_role,
            default_location_role: $location_role,
            label: $label,
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelTemplate {
    SubjectWas(&'static str),
    SubjectDid(&'static str),
    Marriage,
    Divorce,
    None,
}

impl LabelTemplate {
    fn render(self, participant_names: &[String]) -> Option<String> {
        let subject = participant_names.first()?;
        match self {
            Self::SubjectWas(action) => Some(format!("{subject} was {action}")),
            Self::SubjectDid(action) => Some(format!("{subject} {action}")),
            Self::Marriage => match participant_names {
                [first, second, ..] => Some(format!("{first} and {second} married")),
                [name] => Some(format!("{name} married")),
                [] => None,
            },
            Self::Divorce => match participant_names {
                [first, second, ..] => Some(format!("{first} and {second} divorced")),
                [name] => Some(format!("{name} divorced")),
                [] => None,
            },
            Self::None => None,
        }
    }
}

pub(super) fn validate_event_type_value(
    value: &str,
    path: &str,
) -> Result<(), LocalAuthoringError> {
    if value.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{path} event `type` cannot be empty"),
        });
    }

    if value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '/' | '\\' | ':'))
    {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{path} event type `{value}` may not contain whitespace, slashes, or colons"
            ),
        });
    }

    Ok(())
}

impl<'a> LocalEventType<'a> {
    pub(super) fn from_record(record: &'a LocalMarkdownRecord) -> Option<Self> {
        (record.kind == "event").then_some(())?;
        let value = record
            .attributes
            .get("type")
            .and_then(serde_json::Value::as_str)?;

        Some(Self::from_str(value))
    }

    pub(super) fn default_label(self, participant_names: &[String]) -> Option<String> {
        self.profile().label.render(participant_names)
    }

    pub(super) fn default_participant_role(self) -> Option<&'static str> {
        self.profile().default_participant_role
    }

    pub(super) fn default_location_role(self) -> Option<&'static str> {
        self.profile().default_location_role
    }

    fn profile(self) -> LocalEventProfile<'a> {
        match self {
            Self::Birth => profile!(
                "birth",
                Some("subject"),
                Some("birthplace"),
                LabelTemplate::SubjectWas("born")
            ),
            Self::Death => profile!(
                "death",
                Some("subject"),
                Some("death-place"),
                LabelTemplate::SubjectDid("died")
            ),
            Self::Marriage => profile!(
                "marriage",
                Some("partner"),
                Some("marriage-place"),
                LabelTemplate::Marriage
            ),
            Self::Baptism => profile!(
                "baptism",
                Some("subject"),
                Some("baptism-place"),
                LabelTemplate::SubjectWas("baptized")
            ),
            Self::Burial => profile!(
                "burial",
                Some("subject"),
                Some("burial-place"),
                LabelTemplate::SubjectWas("buried")
            ),
            Self::Residence => profile!(
                "residence",
                Some("subject"),
                Some("residence"),
                LabelTemplate::SubjectDid("lived somewhere")
            ),
            Self::Occupation => profile!(
                "occupation",
                Some("subject"),
                Some("workplace"),
                LabelTemplate::SubjectDid("had an occupation")
            ),
            Self::Divorce => profile!(
                "divorce",
                Some("partner"),
                Some("divorce-place"),
                LabelTemplate::Divorce
            ),
            Self::Adoption => profile!(
                "adoption",
                Some("subject"),
                Some("adoption-place"),
                LabelTemplate::SubjectWas("adopted")
            ),
            Self::Education => profile!(
                "education",
                Some("student"),
                Some("school"),
                LabelTemplate::SubjectDid("studied")
            ),
            Self::MilitaryService => profile!(
                "military-service",
                Some("service-member"),
                Some("service-place"),
                LabelTemplate::SubjectDid("served in the military")
            ),
            Self::Immigration => profile!(
                "immigration",
                Some("migrant"),
                Some("destination"),
                LabelTemplate::SubjectDid("immigrated")
            ),
            Self::Emigration => profile!(
                "emigration",
                Some("migrant"),
                Some("origin"),
                LabelTemplate::SubjectDid("emigrated")
            ),
            Self::Naturalization => profile!(
                "naturalization",
                Some("subject"),
                Some("naturalization-place"),
                LabelTemplate::SubjectWas("naturalized")
            ),
            Self::Census => profile!(
                "census",
                Some("enumerated-person"),
                Some("enumeration-place"),
                LabelTemplate::SubjectWas("recorded in a census")
            ),
            Self::NameChange => profile!(
                "name-change",
                Some("subject"),
                None,
                LabelTemplate::SubjectDid("changed names")
            ),
            Self::Custom(value) => profile!(value, None, None, LabelTemplate::None),
        }
    }

    fn from_str(value: &'a str) -> Self {
        match value {
            "birth" => Self::Birth,
            "death" => Self::Death,
            "marriage" => Self::Marriage,
            "baptism" => Self::Baptism,
            "burial" => Self::Burial,
            "residence" => Self::Residence,
            "occupation" => Self::Occupation,
            "divorce" => Self::Divorce,
            "adoption" => Self::Adoption,
            "education" => Self::Education,
            "military-service" => Self::MilitaryService,
            "immigration" => Self::Immigration,
            "emigration" => Self::Emigration,
            "naturalization" => Self::Naturalization,
            "census" => Self::Census,
            "name-change" => Self::NameChange,
            other => Self::Custom(other),
        }
    }
}

pub(super) fn local_event_type(record: &LocalMarkdownRecord) -> Option<LocalEventType<'_>> {
    LocalEventType::from_record(record)
}

pub(super) fn local_event_type_id(record: &LocalMarkdownRecord) -> Option<String> {
    local_event_type(record).map(|event_type| event_type.profile().event_type.to_string())
}

pub(super) fn default_event_label(
    record: &LocalMarkdownRecord,
    entity_name: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let event_type = local_event_type(record)?;
    let participant_names = event_participant_names(record, entity_name);
    event_type.default_label(&participant_names)
}

pub(super) fn event_participant_names(
    record: &LocalMarkdownRecord,
    entity_name: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    event_participant_entities(record)
        .into_iter()
        .filter_map(|entity| entity_name(&entity).or(Some(entity)))
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

pub(super) fn event_participant_refs(record: &LocalMarkdownRecord) -> Vec<LocalParticipantRef> {
    let default_role = local_event_type(record).and_then(LocalEventType::default_participant_role);
    let mut participants = Vec::new();

    if let Some(subject) = record
        .attributes
        .get("subject")
        .and_then(serde_json::Value::as_str)
        .filter(|subject| !subject.trim().is_empty())
    {
        participants.push(LocalParticipantRef {
            entity: normalize_participant_entity_id(subject),
            role: default_role.or(Some("subject")).map(ToOwned::to_owned),
        });
    }

    if let Some(mut explicit_participants) = record
        .attributes
        .get("participants")
        .and_then(serde_json::Value::as_array)
        .map(|participants| {
            participants
                .iter()
                .filter_map(|participant| {
                    normalize_participant_ref(record, participant, default_role)
                })
                .collect::<Vec<_>>()
        })
    {
        for participant in explicit_participants.drain(..) {
            if !participants
                .iter()
                .any(|existing| existing.entity == participant.entity)
            {
                participants.push(participant);
            }
        }
    }

    participants
}

pub(super) fn event_participants(record: &LocalMarkdownRecord) -> Vec<serde_json::Value> {
    event_participant_refs(record)
        .into_iter()
        .filter_map(|participant| serde_json::to_value(participant).ok())
        .collect()
}

pub(super) fn event_participant_entity_ids(record: &LocalMarkdownRecord) -> Vec<String> {
    event_participant_refs(record)
        .into_iter()
        .map(|participant| participant.entity)
        .collect()
}

pub(super) fn normalize_participant_entity_id(entity: &str) -> String {
    normalize_person_id(entity)
}

fn infer_self_participant_entity(record: &LocalMarkdownRecord) -> Option<String> {
    record
        .attributes
        .get("subject")
        .and_then(serde_json::Value::as_str)
        .filter(|subject| *subject != "self" && !subject.trim().is_empty())
        .map(normalize_participant_entity_id)
        .or_else(|| {
            (record.kind == "event")
                .then(|| record.id.strip_prefix("event:birth-"))
                .flatten()
                .filter(|slug| !slug.trim().is_empty())
                .map(normalize_participant_entity_id)
        })
        .or_else(|| {
            record
                .path
                .split('/')
                .next_back()
                .and_then(|filename| filename.strip_suffix(".md"))
                .and_then(|stem| {
                    stem.split("--").find_map(|part| {
                        part.strip_prefix("person=")
                            .or_else(|| part.strip_prefix("participant="))
                    })
                })
                .filter(|slug| !slug.trim().is_empty())
                .map(normalize_participant_entity_id)
        })
}

fn normalize_participant_ref(
    record: &LocalMarkdownRecord,
    value: &serde_json::Value,
    default_role: Option<&str>,
) -> Option<LocalParticipantRef> {
    match value {
        serde_json::Value::String(entity) if entity == "self" => Some(LocalParticipantRef {
            entity: infer_self_participant_entity(record)?,
            role: default_role.map(ToOwned::to_owned),
        }),
        serde_json::Value::String(entity) if !entity.trim().is_empty() => {
            Some(LocalParticipantRef {
                entity: normalize_participant_entity_id(entity),
                role: default_role.map(ToOwned::to_owned),
            })
        }
        serde_json::Value::Object(values) => {
            let entity = values
                .get("entity")
                .and_then(serde_json::Value::as_str)
                .and_then(|entity| {
                    if entity == "self" {
                        infer_self_participant_entity(record)
                    } else {
                        Some(normalize_participant_entity_id(entity))
                    }
                })?;
            let role = values
                .get("role")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| default_role.map(ToOwned::to_owned));
            Some(LocalParticipantRef { entity, role })
        }
        _ => None,
    }
}

fn event_participant_entities(record: &LocalMarkdownRecord) -> Vec<String> {
    let participants = event_participant_refs(record);

    participants
        .iter()
        .filter(|participant| {
            participant.role.as_deref().is_none_or(|role| {
                matches!(
                    role,
                    "subject" | "person" | "child" | "deceased" | "spouse" | "partner"
                )
            })
        })
        .map(|participant| participant.entity.clone())
        .chain(
            participants
                .iter()
                .map(|participant| participant.entity.clone())
                .filter(|entity| !seen_in_subject_roles(&participants, entity)),
        )
        .collect()
}

fn seen_in_subject_roles(participants: &[LocalParticipantRef], entity: &str) -> bool {
    participants.iter().any(|participant| {
        participant.entity == entity
            && participant.role.as_deref().is_none_or(|role| {
                matches!(
                    role,
                    "subject" | "person" | "child" | "deceased" | "spouse" | "partner"
                )
            })
    })
}
