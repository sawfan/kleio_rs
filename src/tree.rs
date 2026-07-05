//! Editable tree documents.
//!
//! A tree is the user-facing container for people, events, categories, and
//! explicit relationships. GEDCOM can be attached as a source/backing feature,
//! but the document is deliberately source-agnostic so users can build arbitrary
//! structures without importing a GEDCOM file first.

use rkyv::{Archive, Deserialize, Serialize};

use crate::{Event, EventId, Name, Person, PersonId, Provenance, Sex};

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
pub struct TreeId(pub String);

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
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
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct RelationshipId(pub u64);

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
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
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct CategoryId(pub u64);

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
pub struct TreeMetadata {
    pub id: TreeId,
    pub title: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// A user-defined grouping for people, events, or relationships.
///
/// Examples: "Partner's family", "Research queue", "Fictional court", or
/// "Imported from 2026 GEDCOM".
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
pub struct TreeCategory {
    pub id: CategoryId,
    pub label: String,
    pub description: Option<String>,
    pub color: Option<String>,
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
pub enum RelationshipKind {
    BiologicalParentChild,
    AdoptiveParentChild,
    FosterParentChild,
    StepParentChild,
    GuardianChild,
    Spouse,
    Partner,
    FormerSpouse,
    Sibling,
    Household,
    Associate,
    Other(String),
}

impl RelationshipKind {
    pub fn label(&self) -> &str {
        match self {
            Self::BiologicalParentChild => "biological parent/child",
            Self::AdoptiveParentChild => "adoptive parent/child",
            Self::FosterParentChild => "foster parent/child",
            Self::StepParentChild => "step parent/child",
            Self::GuardianChild => "guardian/child",
            Self::Spouse => "spouse",
            Self::Partner => "partner",
            Self::FormerSpouse => "former spouse",
            Self::Sibling => "sibling",
            Self::Household => "household",
            Self::Associate => "associate",
            Self::Other(label) => label.as_str(),
        }
    }

    pub fn as_value(&self) -> &str {
        match self {
            Self::BiologicalParentChild => "biological-parent-child",
            Self::AdoptiveParentChild => "adoptive-parent-child",
            Self::FosterParentChild => "foster-parent-child",
            Self::StepParentChild => "step-parent-child",
            Self::GuardianChild => "guardian-child",
            Self::Spouse => "spouse",
            Self::Partner => "partner",
            Self::FormerSpouse => "former-spouse",
            Self::Sibling => "sibling",
            Self::Household => "household",
            Self::Associate => "associate",
            Self::Other(label) => label.as_str(),
        }
    }

    pub fn from_value(value: &str) -> Self {
        match value {
            "biological-parent-child" => Self::BiologicalParentChild,
            "adoptive-parent-child" => Self::AdoptiveParentChild,
            "foster-parent-child" => Self::FosterParentChild,
            "step-parent-child" => Self::StepParentChild,
            "guardian-child" => Self::GuardianChild,
            "spouse" => Self::Spouse,
            "partner" => Self::Partner,
            "former-spouse" => Self::FormerSpouse,
            "sibling" => Self::Sibling,
            "household" => Self::Household,
            "associate" => Self::Associate,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn common_kinds() -> Vec<Self> {
        vec![
            Self::BiologicalParentChild,
            Self::AdoptiveParentChild,
            Self::FosterParentChild,
            Self::StepParentChild,
            Self::GuardianChild,
            Self::Spouse,
            Self::Partner,
            Self::FormerSpouse,
            Self::Sibling,
            Self::Household,
            Self::Associate,
        ]
    }

    pub fn is_parent_child(&self) -> bool {
        matches!(
            self,
            Self::BiologicalParentChild
                | Self::AdoptiveParentChild
                | Self::FosterParentChild
                | Self::StepParentChild
                | Self::GuardianChild
        )
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
pub struct TreeRelationship {
    pub id: RelationshipId,
    pub kind: RelationshipKind,

    /// First endpoint. For parent/child kinds this is the parent/guardian.
    pub source: PersonId,

    /// Second endpoint. For parent/child kinds this is the child.
    pub target: PersonId,

    pub label: Option<String>,
    pub events: Vec<EventId>,
    pub categories: Vec<CategoryId>,
    pub provenance: Provenance,
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
pub struct GedcomFileVersion {
    pub id: String,
    pub filename: String,
    pub imported_at: Option<String>,
    pub content_hash: Option<String>,
    pub note: Option<String>,
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
pub struct GedcomBacking {
    pub label: String,
    pub active_version: Option<String>,
    pub versions: Vec<GedcomFileVersion>,
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
pub enum TreeAttachment {
    Gedcom(GedcomBacking),
    Other { label: String, note: Option<String> },
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq))]
pub struct TreeNodeLayout {
    pub person_id: PersonId,
    pub x: f32,
    pub y: f32,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Default,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TreeLayout {
    pub nodes: Vec<TreeNodeLayout>,
}

impl TreeLayout {
    pub fn position(&self, person_id: PersonId) -> Option<(f32, f32)> {
        self.nodes
            .iter()
            .find(|node| node.person_id == person_id)
            .map(|node| (node.x, node.y))
    }

    pub fn set_position(&mut self, person_id: PersonId, x: f32, y: f32) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|node| node.person_id == person_id)
        {
            node.x = x;
            node.y = y;
        } else {
            self.nodes.push(TreeNodeLayout { person_id, x, y });
        }
    }

    pub fn remove_person(&mut self, person_id: PersonId) {
        self.nodes.retain(|node| node.person_id != person_id);
    }
}

#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct TreeDocument {
    pub metadata: TreeMetadata,

    /// Optional anchor person used by the simple `/tree` path and default view.
    pub main_person: Option<PersonId>,

    pub people: Vec<Person>,
    pub events: Vec<Event>,
    pub relationships: Vec<TreeRelationship>,
    pub categories: Vec<TreeCategory>,
    pub attachments: Vec<TreeAttachment>,
    #[serde(default)]
    pub layout: TreeLayout,
}

impl TreeDocument {
    pub fn empty(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            metadata: TreeMetadata {
                id: TreeId(id.into()),
                title: title.into(),
                description: None,
                created_at: None,
                updated_at: None,
            },
            main_person: None,
            people: Vec::new(),
            events: Vec::new(),
            relationships: Vec::new(),
            categories: Vec::new(),
            attachments: Vec::new(),
            layout: TreeLayout::default(),
        }
    }

    pub fn next_person_id(&self) -> PersonId {
        PersonId(self.people.iter().map(|p| p.id.0).max().unwrap_or(0) + 1)
    }

    pub fn next_relationship_id(&self) -> RelationshipId {
        RelationshipId(
            self.relationships
                .iter()
                .map(|relationship| relationship.id.0)
                .max()
                .unwrap_or(0)
                + 1,
        )
    }

    pub fn next_category_id(&self) -> CategoryId {
        CategoryId(
            self.categories
                .iter()
                .map(|category| category.id.0)
                .max()
                .unwrap_or(0)
                + 1,
        )
    }

    pub fn add_person(&mut self, display_name: impl Into<String>, sex: Option<Sex>) -> PersonId {
        let id = self.next_person_id();
        let display_name = display_name.into();
        self.people.push(Person {
            id,
            names: vec![Name {
                display: display_name,
                given: None,
                surname: None,
                aliases: Vec::new(),
                provenance: Provenance::default(),
            }],
            sex,
            events: Vec::new(),
            families_as_child: Vec::new(),
            families_as_spouse: Vec::new(),
            notes: Vec::new(),
            source_record: None,
            provenance: Provenance::default(),
        });
        self.layout.set_position(id, 0.0, 0.0);
        id
    }

    pub fn add_relationship(
        &mut self,
        kind: RelationshipKind,
        source: PersonId,
        target: PersonId,
    ) -> RelationshipId {
        let id = self.next_relationship_id();
        self.relationships.push(TreeRelationship {
            id,
            kind,
            source,
            target,
            label: None,
            events: Vec::new(),
            categories: Vec::new(),
            provenance: Provenance::default(),
        });
        id
    }

    pub fn remove_relationship(&mut self, id: RelationshipId) -> bool {
        let original_len = self.relationships.len();
        self.relationships
            .retain(|relationship| relationship.id != id);
        self.relationships.len() != original_len
    }

    pub fn rename_person(&mut self, id: PersonId, display_name: impl Into<String>) -> bool {
        let display_name = display_name.into();
        let Some(person) = self.people.iter_mut().find(|person| person.id == id) else {
            return false;
        };

        if let Some(name) = person.names.first_mut() {
            name.display = display_name;
        } else {
            person.names.push(Name {
                display: display_name,
                given: None,
                surname: None,
                aliases: Vec::new(),
                provenance: Provenance::default(),
            });
        }

        true
    }

    pub fn remove_person(&mut self, id: PersonId) -> bool {
        let original_len = self.people.len();
        self.people.retain(|person| person.id != id);
        let removed = self.people.len() != original_len;

        if removed {
            self.relationships
                .retain(|relationship| relationship.source != id && relationship.target != id);
            for event in &mut self.events {
                event.participants.retain(|participant| *participant != id);
            }
            if self.main_person == Some(id) {
                self.main_person = None;
            }
            self.layout.remove_person(id);
        }

        removed
    }

    pub fn has_person(&self, id: PersonId) -> bool {
        self.people.iter().any(|person| person.id == id)
    }

    pub fn has_relationship(
        &self,
        kind: &RelationshipKind,
        source: PersonId,
        target: PersonId,
    ) -> bool {
        self.relationships.iter().any(|relationship| {
            relationship.kind == *kind
                && relationship.source == source
                && relationship.target == target
        })
    }

    pub fn person_display_name(&self, id: PersonId) -> Option<&str> {
        self.people
            .iter()
            .find(|person| person.id == id)
            .and_then(|person| person.names.first())
            .map(|name| name.display.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_can_contain_unrelated_people_and_arbitrary_edges() {
        let mut tree = TreeDocument::empty("main", "Main tree");
        let user = tree.add_person("Me", None);
        let partner = tree.add_person("Partner", None);
        let friend = tree.add_person("Family friend", None);

        tree.main_person = Some(user);
        tree.add_relationship(RelationshipKind::Partner, user, partner);
        tree.add_relationship(RelationshipKind::Associate, user, friend);

        assert_eq!(tree.people.len(), 3);
        assert_eq!(tree.relationships.len(), 2);
        assert_eq!(tree.person_display_name(partner), Some("Partner"));
    }

    #[test]
    fn tree_can_track_multiple_gedcom_versions_but_does_not_require_one() {
        let mut tree = TreeDocument::empty("research", "Research tree");
        assert!(tree.attachments.is_empty());

        tree.attachments.push(TreeAttachment::Gedcom(GedcomBacking {
            label: "Smith import".to_string(),
            active_version: Some("v2".to_string()),
            versions: vec![
                GedcomFileVersion {
                    id: "v1".to_string(),
                    filename: "smith-2025.ged".to_string(),
                    imported_at: None,
                    content_hash: Some("hash-1".to_string()),
                    note: None,
                },
                GedcomFileVersion {
                    id: "v2".to_string(),
                    filename: "smith-2026.ged".to_string(),
                    imported_at: None,
                    content_hash: Some("hash-2".to_string()),
                    note: Some("updated cousins branch".to_string()),
                },
            ],
        }));

        let Some(TreeAttachment::Gedcom(backing)) = tree.attachments.first() else {
            panic!("expected GEDCOM backing");
        };
        assert_eq!(backing.versions.len(), 2);
        assert_eq!(backing.active_version.as_deref(), Some("v2"));
    }
}
