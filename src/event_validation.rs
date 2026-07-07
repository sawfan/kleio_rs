//! Validation helpers for domain-profile event constraints.
//!
//! Validation is intentionally non-destructive. It reports errors/warnings that
//! UI/import code can show during authoring or preview without changing source
//! events or domain profiles.

use std::collections::{BTreeMap, BTreeSet};

use rkyv::{Archive, Deserialize, Serialize};

use crate::event::{EventBoundaryKind, EventRelation, EventRelationKind, TimeSpec, TimelineEvent};
use crate::event_type::{DomainProfile, EventConstraint, EventTypeDef, RoleId};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ValidationSeverity {
    Error,
    Warning,
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
pub enum EventValidationIssueKind {
    UnknownEventType,
    DuplicateEventTypeId,
    DuplicateRoleId {
        role: RoleId,
    },
    RoleNotAllowed {
        role: RoleId,
    },
    RoleCountTooLow {
        role: RoleId,
        actual: u16,
        min: u16,
    },
    RoleCountTooHigh {
        role: RoleId,
        actual: u16,
        max: u16,
    },
    BoundaryRoleMismatch {
        event_id: crate::EventId,
        relation: EventRelationKind,
        boundary: EventBoundaryKind,
    },
    BoundaryRoleInferred {
        event_id: crate::EventId,
        relation: EventRelationKind,
        inferred: EventBoundaryKind,
    },
    PlaceRecommended,
    TimeRecommended,
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
pub struct EventValidationIssue {
    pub severity: ValidationSeverity,
    pub kind: EventValidationIssueKind,
    pub message: String,
}

impl EventValidationIssue {
    pub fn error(kind: EventValidationIssueKind, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            kind,
            message: message.into(),
        }
    }

    pub fn warning(kind: EventValidationIssueKind, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for EventValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EventValidationIssue {}

pub fn validate_domain_profile(profile: &DomainProfile) -> Vec<EventValidationIssue> {
    let mut issues = Vec::new();
    let mut role_ids = BTreeSet::new();
    for role in &profile.role_types {
        if !role_ids.insert(role.id.clone()) {
            issues.push(EventValidationIssue::error(
                EventValidationIssueKind::DuplicateRoleId {
                    role: role.id.clone(),
                },
                format!(
                    "duplicate role id `{}` in profile `{}`",
                    role.id.as_str(),
                    profile.id.as_str()
                ),
            ));
        }
    }

    let mut event_type_ids = BTreeSet::new();
    for event_type in &profile.event_types {
        if !event_type_ids.insert(event_type.id.clone()) {
            issues.push(EventValidationIssue::error(
                EventValidationIssueKind::DuplicateEventTypeId,
                format!(
                    "duplicate event type id `{}` in profile `{}`",
                    event_type.id.as_str(),
                    profile.id.as_str()
                ),
            ));
        }
    }

    issues
}

pub fn validate_event_relations(
    events: &[TimelineEvent],
    relations: &[EventRelation],
) -> Vec<EventValidationIssue> {
    let mut issues = Vec::new();

    for relation in relations {
        let Some(child) = events
            .iter()
            .find(|event| event.id == relation.child_event_id)
        else {
            continue;
        };
        let Some(implied_boundary) = relation.kind.implied_child_boundary() else {
            continue;
        };
        let actual_boundary = child.boundary_kind();
        if actual_boundary == EventBoundaryKind::None {
            issues.push(EventValidationIssue::warning(
                EventValidationIssueKind::BoundaryRoleInferred {
                    event_id: child.id,
                    relation: relation.kind.clone(),
                    inferred: implied_boundary.clone(),
                },
                format!(
                    "event #{} is used as a {:?} boundary but has no explicit boundary role; inferred {:?}",
                    child.id.0, relation.kind, implied_boundary
                ),
            ));
            continue;
        }

        let matches_relation = match implied_boundary {
            EventBoundaryKind::Start => actual_boundary.includes_start(),
            EventBoundaryKind::End => actual_boundary.includes_end(),
            EventBoundaryKind::None | EventBoundaryKind::StartAndEnd => true,
        };
        if !matches_relation {
            issues.push(EventValidationIssue::warning(
                EventValidationIssueKind::BoundaryRoleMismatch {
                    event_id: child.id,
                    relation: relation.kind.clone(),
                    boundary: actual_boundary,
                },
                format!(
                    "event #{} is linked via {:?} but has boundary role {:?}",
                    child.id.0,
                    relation.kind,
                    child.boundary_kind()
                ),
            ));
        }
    }

    issues
}

pub fn validate_timeline_event(
    event: &TimelineEvent,
    profiles: &[DomainProfile],
) -> Vec<EventValidationIssue> {
    let Some(event_type) = find_event_type(profiles, event) else {
        return vec![EventValidationIssue::warning(
            EventValidationIssueKind::UnknownEventType,
            format!("unknown event type `{}`", event.type_ref.as_str()),
        )];
    };

    validate_timeline_event_with_type(event, event_type)
}

pub fn validate_timeline_event_with_type(
    event: &TimelineEvent,
    event_type: &EventTypeDef,
) -> Vec<EventValidationIssue> {
    let mut issues = Vec::new();
    let allowed_roles: BTreeSet<RoleId> = event_type.allowed_roles.iter().cloned().collect();
    let mut role_counts: BTreeMap<RoleId, u16> = BTreeMap::new();

    for participant in &event.participants {
        *role_counts.entry(participant.role.clone()).or_default() += 1;
        if !allowed_roles.is_empty() && !allowed_roles.contains(&participant.role) {
            issues.push(EventValidationIssue::warning(
                EventValidationIssueKind::RoleNotAllowed {
                    role: participant.role.clone(),
                },
                format!(
                    "role `{}` is not listed for event type `{}`",
                    participant.role.as_str(),
                    event_type.id.as_str()
                ),
            ));
        }
    }

    for constraint in &event_type.constraints {
        match constraint {
            EventConstraint::RoleCount { role, min, max } => {
                let actual = role_counts.get(role).copied().unwrap_or_default();
                if actual < *min {
                    issues.push(EventValidationIssue::error(
                        EventValidationIssueKind::RoleCountTooLow {
                            role: role.clone(),
                            actual,
                            min: *min,
                        },
                        format!(
                            "event type `{}` expects at least {} participant(s) with role `{}`, found {}",
                            event_type.id.as_str(),
                            min,
                            role.as_str(),
                            actual
                        ),
                    ));
                }
                if let Some(max) = max
                    && actual > *max
                {
                    issues.push(EventValidationIssue::error(
                        EventValidationIssueKind::RoleCountTooHigh {
                            role: role.clone(),
                            actual,
                            max: *max,
                        },
                        format!(
                            "event type `{}` expects at most {} participant(s) with role `{}`, found {}",
                            event_type.id.as_str(),
                            max,
                            role.as_str(),
                            actual
                        ),
                    ));
                }
            }
            EventConstraint::PlaceRecommended if event.place.is_none() => {
                issues.push(EventValidationIssue::warning(
                    EventValidationIssueKind::PlaceRecommended,
                    format!(
                        "event type `{}` usually has a place",
                        event_type.id.as_str()
                    ),
                ));
            }
            EventConstraint::TimeRecommended if matches!(event.time, TimeSpec::Unknown) => {
                issues.push(EventValidationIssue::warning(
                    EventValidationIssueKind::TimeRecommended,
                    format!(
                        "event type `{}` usually has a time/date",
                        event_type.id.as_str()
                    ),
                ));
            }
            EventConstraint::PreferredSingleRole { .. }
            | EventConstraint::PlaceRecommended
            | EventConstraint::TimeRecommended => {}
        }
    }

    issues
}

fn find_event_type<'a>(
    profiles: &'a [DomainProfile],
    event: &TimelineEvent,
) -> Option<&'a EventTypeDef> {
    profiles
        .iter()
        .find_map(|profile| profile.event_type(&event.type_ref))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DateValue, EventId, EventParticipant, EventTypeId, PersonId, Provenance, TimeSpec,
        genealogy_domain_profile,
    };

    #[test]
    fn birth_event_requires_child_role() {
        let profile = genealogy_domain_profile();
        let event = TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.birth"), "Birth");

        let issues = validate_timeline_event(&event, &[profile]);

        assert!(
            issues.iter().any(|issue| matches!(
                issue.kind,
                EventValidationIssueKind::RoleCountTooLow { .. }
            ))
        );
    }

    #[test]
    fn complete_birth_event_only_warns_for_missing_place() {
        let profile = genealogy_domain_profile();
        let event = TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.birth"), "Birth")
            .with_time(TimeSpec::from_date_value(DateValue::from_original(
                "1901",
                Provenance::default(),
            )))
            .with_participant(EventParticipant::new(PersonId(7), "child"));

        let issues = validate_timeline_event(&event, &[profile]);

        assert!(
            issues
                .iter()
                .all(|issue| issue.severity == ValidationSeverity::Warning)
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.kind == EventValidationIssueKind::PlaceRecommended)
        );
    }

    #[test]
    fn disallowed_role_is_warning() {
        let profile = genealogy_domain_profile();
        let event = TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.birth"), "Birth")
            .with_participant(EventParticipant::new(PersonId(7), "commander"));

        let issues = validate_timeline_event(&event, &[profile]);

        assert!(issues.iter().any(|issue| matches!(
            &issue.kind,
            EventValidationIssueKind::RoleNotAllowed { role } if role.as_str() == "commander"
        )));
    }

    #[test]
    fn relation_validation_accepts_matching_boundary_roles() {
        let events = vec![
            TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.life"), "Life")
                .with_composition_kind(crate::EventCompositionKind::Composite)
                .with_temporal_kind(crate::EventTemporalKind::Interval),
            TimelineEvent::new(EventId(2), EventTypeId::new("genealogy.birth"), "Birth")
                .with_boundary_kind(EventBoundaryKind::Start),
        ];
        let relations = vec![EventRelation::new(
            EventId(1),
            EventId(2),
            EventRelationKind::Starts,
        )];

        let issues = validate_event_relations(&events, &relations);

        assert!(issues.is_empty());
    }

    #[test]
    fn relation_validation_warns_for_boundary_role_mismatch() {
        let events = vec![
            TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.life"), "Life")
                .with_composition_kind(crate::EventCompositionKind::Composite)
                .with_temporal_kind(crate::EventTemporalKind::Interval),
            TimelineEvent::new(EventId(2), EventTypeId::new("genealogy.death"), "Death")
                .with_boundary_kind(EventBoundaryKind::End),
        ];
        let relations = vec![EventRelation::new(
            EventId(1),
            EventId(2),
            EventRelationKind::Starts,
        )];

        let issues = validate_event_relations(&events, &relations);

        assert!(issues.iter().any(|issue| matches!(
            issue.kind,
            EventValidationIssueKind::BoundaryRoleMismatch { .. }
        )));
    }
}
