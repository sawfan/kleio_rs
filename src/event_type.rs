//! Domain-scoped event type and role definitions.
//!
//! Kleio stores events as generic historical facts. Domain profiles describe
//! what a specific event type means for a given domain, which participant roles
//! it expects, and how UIs should present it. This keeps the core model from
//! becoming one giant enum containing every genealogy, journal, military,
//! political, legal, or project-local event kind.

use rkyv::{Archive, Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct DomainId(pub String);

impl DomainId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct EventTypeId(pub String);

impl EventTypeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn from_parts(domain: &str, key: &str) -> Self {
        Self(format!("{domain}.{key}"))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct RoleDef {
    pub id: RoleId,
    pub key: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum EventConstraint {
    /// The event should include this role within the given count range.
    RoleCount {
        role: RoleId,
        min: u16,
        max: Option<u16>,
    },

    /// UIs should invite a place value for this type, but imports may omit it.
    PlaceRecommended,

    /// UIs should invite a time/date value for this type, but imports may omit it.
    TimeRecommended,

    /// One participant for this role should be preferred for layouts/summaries.
    PreferredSingleRole { role: RoleId },
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct EventPresentation {
    pub icon: Option<String>,
    pub color: Option<String>,
    pub summary_template: Option<String>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct EventTypeDef {
    pub id: EventTypeId,
    pub domain: DomainId,
    pub key: String,
    pub label: String,
    pub description: Option<String>,
    pub allowed_roles: Vec<RoleId>,
    pub constraints: Vec<EventConstraint>,
    pub presentation: EventPresentation,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct DomainProfile {
    pub id: DomainId,
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub role_types: Vec<RoleDef>,
    pub event_types: Vec<EventTypeDef>,
}

impl DomainProfile {
    pub fn event_type(&self, id: &EventTypeId) -> Option<&EventTypeDef> {
        self.event_types
            .iter()
            .find(|event_type| &event_type.id == id)
    }

    pub fn role_type(&self, id: &RoleId) -> Option<&RoleDef> {
        self.role_types.iter().find(|role| &role.id == id)
    }
}

pub fn genealogy_domain_profile() -> DomainProfile {
    let domain = DomainId::new("genealogy");
    let role_types = vec![
        role(
            "child",
            "Child / subject",
            Some("The person whose birth or childhood fact is being recorded."),
        ),
        role("subject", "Subject", None),
        role("biological_mother", "Biological mother", None),
        role("biological_father", "Biological father", None),
        role("adoptive_parent", "Adoptive parent", None),
        role("spouse", "Spouse / partner", None),
        role("witness", "Witness", None),
        role("informant", "Informant", None),
        role("resident", "Resident", None),
        role("deceased", "Deceased", None),
    ];

    DomainProfile {
        id: domain.clone(),
        title: "Genealogy".to_string(),
        version: "0.1.0".to_string(),
        description: Some(
            "Family-tree and life-event vocabulary with conventional genealogy hints.".to_string(),
        ),
        role_types,
        event_types: vec![
            event_type(
                &domain,
                "birth",
                "Birth",
                Some("A person's birth."),
                &[
                    "child",
                    "biological_mother",
                    "biological_father",
                    "adoptive_parent",
                    "informant",
                ],
                vec![
                    role_count("child", 1, Some(1)),
                    EventConstraint::PreferredSingleRole {
                        role: RoleId::new("child"),
                    },
                    EventConstraint::PlaceRecommended,
                    EventConstraint::TimeRecommended,
                ],
                Some("{child} was born"),
            ),
            event_type(
                &domain,
                "death",
                "Death",
                Some("A person's death."),
                &["deceased", "informant"],
                vec![
                    role_count("deceased", 1, Some(1)),
                    EventConstraint::PreferredSingleRole {
                        role: RoleId::new("deceased"),
                    },
                    EventConstraint::PlaceRecommended,
                    EventConstraint::TimeRecommended,
                ],
                Some("{deceased} died"),
            ),
            event_type(
                &domain,
                "marriage",
                "Marriage / partnership",
                Some("A marriage, civil union, or partnership event."),
                &["spouse", "witness"],
                vec![
                    role_count("spouse", 2, None),
                    EventConstraint::TimeRecommended,
                ],
                Some("{spouse} married/partnered"),
            ),
            event_type(
                &domain,
                "baptism",
                "Baptism",
                Some("A baptism or christening event."),
                &["subject", "witness", "informant"],
                vec![
                    role_count("subject", 1, Some(1)),
                    EventConstraint::TimeRecommended,
                ],
                Some("{subject} was baptized"),
            ),
            event_type(
                &domain,
                "burial",
                "Burial",
                Some("A burial or interment event."),
                &["deceased", "informant"],
                vec![
                    role_count("deceased", 1, Some(1)),
                    EventConstraint::PlaceRecommended,
                    EventConstraint::TimeRecommended,
                ],
                Some("{deceased} was buried"),
            ),
            event_type(
                &domain,
                "residence",
                "Residence",
                Some("A person or household living at a place for an instant or range."),
                &["resident"],
                vec![
                    role_count("resident", 1, None),
                    EventConstraint::PlaceRecommended,
                    EventConstraint::TimeRecommended,
                ],
                Some("{resident} lived at {place}"),
            ),
            event_type(
                &domain,
                "occupation",
                "Occupation",
                Some("A person's occupation, office, or work fact."),
                &["subject"],
                vec![
                    role_count("subject", 1, Some(1)),
                    EventConstraint::TimeRecommended,
                ],
                Some("{subject} worked as {title}"),
            ),
            event_type(
                &domain,
                "life",
                "Life / existence",
                Some(
                    "A scale-relative composite interval representing a person's life/existence between boundary events.",
                ),
                &["subject"],
                vec![
                    role_count("subject", 1, Some(1)),
                    EventConstraint::TimeRecommended,
                ],
                Some("{subject} lived"),
            ),
        ],
    }
}

pub fn journal_domain_profile() -> DomainProfile {
    let domain = DomainId::new("journal");
    let role_types = vec![
        role("author", "Author", None),
        role("subject", "Subject", None),
        role("mentioned", "Mentioned entity", None),
        role("location", "Location", None),
    ];

    DomainProfile {
        id: domain.clone(),
        title: "Journal".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Personal journal and retrospective note vocabulary.".to_string()),
        role_types,
        event_types: vec![event_type(
            &domain,
            "entry",
            "Journal entry",
            Some("A user-authored journal or retrospective note."),
            &["author", "subject", "mentioned", "location"],
            vec![EventConstraint::TimeRecommended],
            Some("{title}"),
        )],
    }
}

pub fn research_domain_profile() -> DomainProfile {
    let domain = DomainId::new("research");
    let role_types = vec![
        role("researcher", "Researcher", None),
        role("repository", "Repository", None),
        role("source_examined", "Source examined", None),
        role("subject", "Subject", None),
    ];

    DomainProfile {
        id: domain.clone(),
        title: "Research".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Research log and source examination vocabulary.".to_string()),
        role_types,
        event_types: vec![event_type(
            &domain,
            "archive_visit",
            "Archive visit",
            Some("A research session at a repository or archive."),
            &["researcher", "repository", "source_examined", "subject"],
            vec![
                role_count("researcher", 1, None),
                EventConstraint::TimeRecommended,
            ],
            Some("{researcher} visited {repository}"),
        )],
    }
}

pub fn built_in_domain_profiles() -> Vec<DomainProfile> {
    vec![
        genealogy_domain_profile(),
        journal_domain_profile(),
        research_domain_profile(),
    ]
}

fn role(key: &str, label: &str, description: Option<&str>) -> RoleDef {
    RoleDef {
        id: RoleId::new(key),
        key: key.to_string(),
        label: label.to_string(),
        description: description.map(str::to_string),
    }
}

fn role_count(role: &str, min: u16, max: Option<u16>) -> EventConstraint {
    EventConstraint::RoleCount {
        role: RoleId::new(role),
        min,
        max,
    }
}

fn event_type(
    domain: &DomainId,
    key: &str,
    label: &str,
    description: Option<&str>,
    allowed_roles: &[&str],
    constraints: Vec<EventConstraint>,
    summary_template: Option<&str>,
) -> EventTypeDef {
    EventTypeDef {
        id: EventTypeId::from_parts(domain.as_str(), key),
        domain: domain.clone(),
        key: key.to_string(),
        label: label.to_string(),
        description: description.map(str::to_string),
        allowed_roles: allowed_roles
            .iter()
            .map(|role| RoleId::new(*role))
            .collect(),
        constraints,
        presentation: EventPresentation {
            icon: None,
            color: None,
            summary_template: summary_template.map(str::to_string),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genealogy_birth_profile_has_expected_constraints() {
        let profile = genealogy_domain_profile();
        let birth_id = EventTypeId::new("genealogy.birth");
        let birth = profile.event_type(&birth_id).expect("birth event type");

        assert_eq!(birth.label, "Birth");
        assert!(birth.allowed_roles.contains(&RoleId::new("child")));
        assert!(birth.constraints.contains(&EventConstraint::RoleCount {
            role: RoleId::new("child"),
            min: 1,
            max: Some(1),
        }));
    }

    #[test]
    fn built_in_profiles_use_stable_domain_scoped_ids() {
        let event_type_ids: Vec<String> = built_in_domain_profiles()
            .into_iter()
            .flat_map(|profile| profile.event_types)
            .map(|event_type| event_type.id.0)
            .collect();

        assert!(event_type_ids.contains(&"genealogy.birth".to_string()));
        assert!(event_type_ids.contains(&"journal.entry".to_string()));
        assert!(event_type_ids.contains(&"research.archive_visit".to_string()));
    }
}
