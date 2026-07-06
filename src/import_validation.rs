//! Import-batch validation helpers.
//!
//! These helpers apply domain-profile event validation to import candidates so
//! preview UIs can show clear issues before materializing a pack. They do not
//! mutate the batch unless the caller explicitly uses the marking helper.

use rkyv::{Archive, Deserialize, Serialize};

use crate::event_type::DomainProfile;
use crate::event_validation::{
    EventValidationIssue, ValidationSeverity, validate_domain_profile, validate_timeline_event,
};
use crate::import_batch::{
    ImportBatch, ImportCandidate, ImportCandidateItem, ImportCandidateStatus,
};

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
pub struct ImportCandidateValidation {
    pub candidate_id: crate::ImportCandidateId,
    pub issues: Vec<EventValidationIssue>,
}

impl ImportCandidateValidation {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Warning)
    }
}

pub fn validate_import_batch(batch: &ImportBatch) -> Vec<ImportCandidateValidation> {
    let profiles = import_batch_domain_profiles(batch);
    let mut validations = Vec::new();

    for candidate in &batch.candidates {
        let issues = validate_import_candidate(candidate, &profiles);
        if !issues.is_empty() {
            validations.push(ImportCandidateValidation {
                candidate_id: candidate.id.clone(),
                issues,
            });
        }
    }

    validations
}

pub fn validate_import_candidate(
    candidate: &ImportCandidate,
    profiles: &[DomainProfile],
) -> Vec<EventValidationIssue> {
    match &candidate.item {
        ImportCandidateItem::DomainProfile(profile) => validate_domain_profile(profile),
        ImportCandidateItem::Event(event) => validate_timeline_event(event, profiles),
        ImportCandidateItem::Entity(_)
        | ImportCandidateItem::EventRelation(_)
        | ImportCandidateItem::Source(_)
        | ImportCandidateItem::TagValue(_) => Vec::new(),
    }
}

/// Mark candidate validation results directly on an import batch.
///
/// Candidates with validation errors are marked `Conflict` and receive the issue
/// messages. Candidates with only warnings keep their current status and receive
/// messages, so UI code can still allow them to be accepted.
pub fn mark_import_batch_validation(batch: &mut ImportBatch) -> Vec<ImportCandidateValidation> {
    let validations = validate_import_batch(batch);

    for validation in &validations {
        if let Some(candidate) = batch
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == validation.candidate_id)
        {
            if validation.has_errors() {
                candidate.status = ImportCandidateStatus::Conflict;
            }
            for issue in &validation.issues {
                candidate.messages.push(issue.message.clone());
            }
        }
    }

    validations
}

pub fn import_batch_domain_profiles(batch: &ImportBatch) -> Vec<DomainProfile> {
    batch
        .candidates
        .iter()
        .filter_map(|candidate| match &candidate.item {
            ImportCandidateItem::DomainProfile(profile) => Some(profile.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EventId, EventTypeId, ImportCandidateId, ImportSourceKind, TimelineEvent,
        genealogy_domain_profile,
    };

    #[test]
    fn import_validation_reports_event_constraint_errors() {
        let mut batch = ImportBatch::new(
            crate::ImportBatchId::new("import:validation"),
            "manual",
            ImportSourceKind::Manual,
        );
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new("candidate:profile:genealogy"),
                ImportCandidateItem::DomainProfile(genealogy_domain_profile()),
            )
            .accepted(),
        );
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new("candidate:event:birth"),
                ImportCandidateItem::Event(TimelineEvent::new(
                    EventId(1),
                    EventTypeId::new("genealogy.birth"),
                    "Incomplete birth",
                )),
            )
            .accepted(),
        );

        let validations = validate_import_batch(&batch);

        assert_eq!(validations.len(), 1);
        assert!(validations[0].has_errors());
        assert_eq!(
            validations[0].candidate_id.as_str(),
            "candidate:event:birth"
        );
    }

    #[test]
    fn marking_import_validation_turns_errors_into_conflicts() {
        let mut batch = ImportBatch::new(
            crate::ImportBatchId::new("import:validation"),
            "manual",
            ImportSourceKind::Manual,
        );
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new("candidate:profile:genealogy"),
                ImportCandidateItem::DomainProfile(genealogy_domain_profile()),
            )
            .accepted(),
        );
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new("candidate:event:birth"),
                ImportCandidateItem::Event(TimelineEvent::new(
                    EventId(1),
                    EventTypeId::new("genealogy.birth"),
                    "Incomplete birth",
                )),
            )
            .accepted(),
        );

        let validations = mark_import_batch_validation(&mut batch);
        let event_candidate = batch
            .candidates
            .iter()
            .find(|candidate| candidate.id.as_str() == "candidate:event:birth")
            .expect("event candidate");

        assert_eq!(validations.len(), 1);
        assert_eq!(event_candidate.status, ImportCandidateStatus::Conflict);
        assert!(!event_candidate.messages.is_empty());
    }
}
