use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::RelationshipKind;

use super::{LocalDataBundle, LocalTomlDocument};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalDerivedKinshipRelationship {
    pub relationship_kind: String,
    pub source: String,
    pub target: String,
    pub depth: u32,
    pub inferred_from: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectRelationship {
    id: String,
    kind: RelationshipKind,
    source: String,
    target: String,
}

pub fn infer_local_kinship_relationships(
    bundle: &LocalDataBundle,
) -> Vec<LocalDerivedKinshipRelationship> {
    let direct = bundle
        .toml_documents
        .iter()
        .filter_map(direct_relationship_from_document)
        .collect::<Vec<_>>();
    infer_kinship_from_direct_relationships(&direct)
}

fn infer_kinship_from_direct_relationships(
    direct: &[DirectRelationship],
) -> Vec<LocalDerivedKinshipRelationship> {
    let mut parent_to_children = BTreeMap::<String, Vec<ParentChildEdge>>::new();
    let mut child_to_parents = BTreeMap::<String, Vec<ParentChildEdge>>::new();
    let mut authored_symmetric = BTreeSet::<(String, String, String)>::new();

    for relationship in direct {
        if relationship.kind.is_parent_child() {
            let edge = ParentChildEdge {
                parent: relationship.source.clone(),
                child: relationship.target.clone(),
                relationship_id: relationship.id.clone(),
            };
            parent_to_children
                .entry(edge.parent.clone())
                .or_default()
                .push(edge.clone());
            child_to_parents
                .entry(edge.child.clone())
                .or_default()
                .push(edge);
        }

        if is_symmetric_kind(&relationship.kind) {
            authored_symmetric.insert(symmetric_key(
                relationship.kind.as_value(),
                &relationship.source,
                &relationship.target,
            ));
        }
    }

    let mut derived = BTreeMap::<(String, String, String), LocalDerivedKinshipRelationship>::new();

    infer_siblings(&parent_to_children, &authored_symmetric, &mut derived);
    infer_grandparents(&parent_to_children, &mut derived);
    infer_ancestors(&parent_to_children, &mut derived);
    infer_aunts_uncles(&parent_to_children, &mut derived);
    infer_cousins(&parent_to_children, &mut derived);

    derived.into_values().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParentChildEdge {
    parent: String,
    child: String,
    relationship_id: String,
}

fn infer_siblings(
    parent_to_children: &BTreeMap<String, Vec<ParentChildEdge>>,
    authored_symmetric: &BTreeSet<(String, String, String)>,
    derived: &mut BTreeMap<(String, String, String), LocalDerivedKinshipRelationship>,
) {
    for children in parent_to_children.values() {
        for (left_index, left) in children.iter().enumerate() {
            for right in children.iter().skip(left_index + 1) {
                if authored_symmetric.contains(&symmetric_key("sibling", &left.child, &right.child))
                {
                    continue;
                }
                let (source, target) = ordered_pair(&left.child, &right.child);
                insert_derived(
                    derived,
                    "sibling",
                    source,
                    target,
                    1,
                    vec![left.relationship_id.clone(), right.relationship_id.clone()],
                );
            }
        }
    }
}

fn infer_grandparents(
    parent_to_children: &BTreeMap<String, Vec<ParentChildEdge>>,
    derived: &mut BTreeMap<(String, String, String), LocalDerivedKinshipRelationship>,
) {
    for parent_edges in parent_to_children.values() {
        for parent_edge in parent_edges {
            let Some(child_edges) = parent_to_children.get(&parent_edge.child) else {
                continue;
            };
            for child_edge in child_edges {
                insert_derived(
                    derived,
                    "grandparent-grandchild",
                    parent_edge.parent.clone(),
                    child_edge.child.clone(),
                    2,
                    vec![
                        parent_edge.relationship_id.clone(),
                        child_edge.relationship_id.clone(),
                    ],
                );
            }
        }
    }
}

fn infer_ancestors(
    parent_to_children: &BTreeMap<String, Vec<ParentChildEdge>>,
    derived: &mut BTreeMap<(String, String, String), LocalDerivedKinshipRelationship>,
) {
    for ancestor in parent_to_children.keys() {
        let mut queue = VecDeque::<(String, u32, Vec<String>)>::new();
        if let Some(children) = parent_to_children.get(ancestor) {
            for edge in children {
                queue.push_back((edge.child.clone(), 1, vec![edge.relationship_id.clone()]));
            }
        }

        let mut visited = BTreeSet::<String>::new();
        while let Some((person, depth, path)) = queue.pop_front() {
            if !visited.insert(person.clone()) {
                continue;
            }

            if depth >= 2 {
                insert_derived(
                    derived,
                    "ancestor-descendant",
                    ancestor.clone(),
                    person.clone(),
                    depth,
                    path.clone(),
                );
            }

            let Some(children) = parent_to_children.get(&person) else {
                continue;
            };
            for edge in children {
                let mut next_path = path.clone();
                next_path.push(edge.relationship_id.clone());
                queue.push_back((edge.child.clone(), depth + 1, next_path));
            }
        }
    }
}

fn infer_aunts_uncles(
    parent_to_children: &BTreeMap<String, Vec<ParentChildEdge>>,
    derived: &mut BTreeMap<(String, String, String), LocalDerivedKinshipRelationship>,
) {
    for sibling in sibling_links(parent_to_children) {
        infer_aunt_uncle_direction(
            &sibling.left,
            &sibling.right,
            &sibling,
            parent_to_children,
            derived,
        );
        infer_aunt_uncle_direction(
            &sibling.right,
            &sibling.left,
            &sibling,
            parent_to_children,
            derived,
        );
    }
}

fn infer_aunt_uncle_direction(
    relative: &str,
    parent: &str,
    sibling: &SiblingLink,
    parent_to_children: &BTreeMap<String, Vec<ParentChildEdge>>,
    derived: &mut BTreeMap<(String, String, String), LocalDerivedKinshipRelationship>,
) {
    let Some(children) = parent_to_children.get(parent) else {
        return;
    };

    for child in children {
        if child.child == relative {
            continue;
        }
        let mut inferred_from = sibling.inferred_from.clone();
        inferred_from.push(child.relationship_id.clone());
        insert_derived(
            derived,
            "aunt-uncle-nibling",
            relative.to_string(),
            child.child.clone(),
            2,
            inferred_from,
        );
    }
}

fn infer_cousins(
    parent_to_children: &BTreeMap<String, Vec<ParentChildEdge>>,
    derived: &mut BTreeMap<(String, String, String), LocalDerivedKinshipRelationship>,
) {
    for sibling in sibling_links(parent_to_children) {
        let Some(left_children) = parent_to_children.get(&sibling.left) else {
            continue;
        };
        let Some(right_children) = parent_to_children.get(&sibling.right) else {
            continue;
        };

        for left_child in left_children {
            for right_child in right_children {
                if left_child.child == right_child.child {
                    continue;
                }
                let (source, target) = ordered_pair(&left_child.child, &right_child.child);
                let mut inferred_from = sibling.inferred_from.clone();
                inferred_from.push(left_child.relationship_id.clone());
                inferred_from.push(right_child.relationship_id.clone());
                insert_derived(derived, "cousin", source, target, 2, inferred_from);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SiblingLink {
    left: String,
    right: String,
    inferred_from: Vec<String>,
}

fn sibling_links(parent_to_children: &BTreeMap<String, Vec<ParentChildEdge>>) -> Vec<SiblingLink> {
    let mut links = BTreeMap::<(String, String), SiblingLink>::new();
    for children in parent_to_children.values() {
        for (left_index, left) in children.iter().enumerate() {
            for right in children.iter().skip(left_index + 1) {
                let (source, target) = ordered_pair(&left.child, &right.child);
                let mut inferred_from =
                    vec![left.relationship_id.clone(), right.relationship_id.clone()];
                inferred_from.sort();
                inferred_from.dedup();
                links
                    .entry((source.clone(), target.clone()))
                    .or_insert(SiblingLink {
                        left: source,
                        right: target,
                        inferred_from,
                    });
            }
        }
    }
    links.into_values().collect()
}

fn insert_derived(
    derived: &mut BTreeMap<(String, String, String), LocalDerivedKinshipRelationship>,
    relationship_kind: &str,
    source: String,
    target: String,
    depth: u32,
    inferred_from: Vec<String>,
) {
    if source == target {
        return;
    }

    let key = (
        relationship_kind.to_string(),
        source.clone(),
        target.clone(),
    );
    let entry = derived
        .entry(key)
        .or_insert_with(|| LocalDerivedKinshipRelationship {
            relationship_kind: relationship_kind.to_string(),
            source,
            target,
            depth,
            inferred_from: Vec::new(),
        });
    entry.depth = entry.depth.min(depth);
    entry.inferred_from.extend(inferred_from);
    entry.inferred_from.sort();
    entry.inferred_from.dedup();
}

fn direct_relationship_from_document(document: &LocalTomlDocument) -> Option<DirectRelationship> {
    if document.kind.as_deref() != Some("relationship") {
        return None;
    }

    let source = document.data.get("source")?.as_str()?.to_string();
    let target = document.data.get("target")?.as_str()?.to_string();
    let kind = document
        .data
        .get("relationship")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            document
                .data
                .get("relationship_kind")
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| {
            document
                .data
                .get("relation")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("associate");

    Some(DirectRelationship {
        id: document.id.clone().unwrap_or_else(|| document.path.clone()),
        kind: RelationshipKind::from_value(kind),
        source,
        target,
    })
}

fn is_symmetric_kind(kind: &RelationshipKind) -> bool {
    matches!(
        kind,
        RelationshipKind::Spouse
            | RelationshipKind::Partner
            | RelationshipKind::FormerSpouse
            | RelationshipKind::Sibling
            | RelationshipKind::Household
            | RelationshipKind::Associate
    )
}

fn symmetric_key(kind: &str, left: &str, right: &str) -> (String, String, String) {
    let (left, right) = ordered_pair(left, right);
    (kind.to_string(), left, right)
}

fn ordered_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::compile_local_data;

    #[test]
    fn infers_common_kinship_from_parent_child_relationships() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-kinship-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
        for slug in ["alex", "morgan", "riley", "casey", "jordan"] {
            fs::write(
                temp_dir.join(format!("entities/people/{slug}.md")),
                format!(
                    "+++\nid = \"person:{slug}\"\nkind = \"person\"\nprimary_name = \"{slug}\"\n+++\n"
                ),
            )
            .expect("person");
        }
        write_relationship(&temp_dir, "alex-morgan", "person:alex", "person:morgan");
        write_relationship(&temp_dir, "alex-casey", "person:alex", "person:casey");
        write_relationship(&temp_dir, "morgan-riley", "person:morgan", "person:riley");
        write_relationship(&temp_dir, "casey-jordan", "person:casey", "person:jordan");

        let bundle = compile_local_data(&temp_dir).expect("compile local data");
        let derived = infer_local_kinship_relationships(&bundle);

        assert!(has_derived(
            &derived,
            "grandparent-grandchild",
            "person:alex",
            "person:riley"
        ));
        assert!(has_derived(
            &derived,
            "sibling",
            "person:casey",
            "person:morgan"
        ));
        assert!(has_derived(
            &derived,
            "aunt-uncle-nibling",
            "person:casey",
            "person:riley"
        ));
        assert!(has_derived(
            &derived,
            "cousin",
            "person:jordan",
            "person:riley"
        ));
        assert!(has_derived(
            &derived,
            "ancestor-descendant",
            "person:alex",
            "person:jordan"
        ));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    fn write_relationship(temp_dir: &std::path::Path, slug: &str, source: &str, target: &str) {
        fs::write(
            temp_dir.join(format!("relationships/{slug}.toml")),
            format!(
                "id = \"relationship:{slug}\"\nkind = \"relationship\"\nrelationship = \"biological-parent-child\"\nsource = \"{source}\"\ntarget = \"{target}\"\n"
            ),
        )
        .expect("relationship");
    }

    fn has_derived(
        derived: &[LocalDerivedKinshipRelationship],
        kind: &str,
        source: &str,
        target: &str,
    ) -> bool {
        derived.iter().any(|relationship| {
            relationship.relationship_kind == kind
                && relationship.source == source
                && relationship.target == target
        })
    }
}
