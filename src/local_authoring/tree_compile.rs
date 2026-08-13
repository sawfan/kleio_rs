use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{
    Attribute, DateValue, EventId, GenealogyEvent, GenealogyEventKind, Name, NameOrder, ParentRole,
    Person, PersonId, Provenance, RelationshipKind, Sex, SourceRef, Tag, TreeDocument,
};

use super::{
    LocalAuthoringError, LocalDataBundle, LocalMarkdownRecord, LocalTomlDocument,
    event_profiles::{
        default_event_label, event_participants as normalized_event_participants,
        local_event_type_id,
    },
    refs::{normalize_person_id, normalize_source_id, resolve_contextual_id},
};

pub(super) fn tree_from_local_data_bundle(
    bundle: &LocalDataBundle,
) -> Result<TreeDocument, LocalAuthoringError> {
    tree_from_local_data_bundle_with_view(bundle, None)
}

pub(super) fn tree_from_local_data_bundle_with_view(
    bundle: &LocalDataBundle,
    view_slug: Option<&str>,
) -> Result<TreeDocument, LocalAuthoringError> {
    let registry = bundle.toml_documents.iter().find(|document| {
        document.kind.as_deref() == Some("registry") || document.path == "world.toml"
    });
    let tree_view = select_tree_view(&bundle.toml_documents, view_slug);
    let relationship_filter = tree_view.and_then(tree_view_relationship_filter);
    let tree_id = tree_view
        .and_then(|document| document.id.as_deref())
        .or_else(|| {
            registry
                .and_then(|document| document.data.get("tree"))
                .and_then(|tree| tree.get("id"))
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("local-tree");
    let tree_title = tree_view
        .and_then(|document| document.title.as_deref())
        .or_else(|| {
            registry
                .and_then(|document| document.data.get("tree"))
                .and_then(|tree| tree.get("title"))
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| registry.and_then(|document| document.title.as_deref()))
        .unwrap_or("Local private tree");

    let mut tree = TreeDocument::empty(tree_id, tree_title);
    tree.metadata.description = registry
        .and_then(|document| document.data.get("tree"))
        .and_then(|tree| tree.get("description"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);

    let mut person_ids = BTreeMap::<String, PersonId>::new();
    let mut person_names = BTreeMap::<String, String>::new();
    let mut person_sexes = BTreeMap::<String, Sex>::new();
    let mut explicit_preferred_surnames = BTreeSet::<String>::new();
    for record in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "person")
    {
        let person_id = tree.next_person_id();
        person_ids.insert(record.id.clone(), person_id);
        let display = markdown_title(record).unwrap_or_else(|| record.id.clone());
        person_names.insert(record.id.clone(), display.clone());
        let sex = record
            .attributes
            .get("sex")
            .and_then(serde_json::Value::as_str)
            .map(parse_sex);
        if let Some(sex) = &sex {
            person_sexes.insert(record.id.clone(), sex.clone());
        }
        if has_explicit_preferred_surname(record) {
            explicit_preferred_surnames.insert(record.id.clone());
        }
        let mut provenance = local_record_provenance(record);
        provenance.tags.extend(record.tags.iter().cloned().map(Tag));
        tree.people.push(Person {
            id: person_id,
            names: person_names_from_record(record, display),
            sex,
            events: Vec::new(),
            families_as_child: Vec::new(),
            families_as_spouse: Vec::new(),
            notes: Vec::new(),
            source_record: Some(SourceRef(format!("local:{}", record.id))),
            provenance,
        });
    }

    let mut has_explicit_layout_positions = false;
    for (index, record) in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "person")
        .enumerate()
    {
        let Some(person_id) = person_ids.get(&record.id).copied() else {
            continue;
        };
        let x = numeric_attribute(record.attributes.get("x"));
        let y = numeric_attribute(record.attributes.get("y"));
        if x.is_some() || y.is_some() {
            has_explicit_layout_positions = true;
            tree.layout.set_position(
                person_id,
                x.unwrap_or((index as f32) * 180.0),
                y.unwrap_or(0.0),
            );
        }
        add_person_life_event(
            &mut tree,
            person_id,
            record,
            "birth_date",
            GenealogyEventKind::Birth,
        );
        add_person_life_event(
            &mut tree,
            person_id,
            record,
            "death_date",
            GenealogyEventKind::Death,
        );
    }

    for record in bundle
        .markdown_records
        .iter()
        .filter(|record| is_timeline_event_record(record))
    {
        let participants = event_participants(record, &person_ids)?;
        if participants.is_empty() {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} event has no known person participants", record.path),
            });
        }

        let event_id = next_event_id(&tree);
        let mut provenance = local_record_provenance(record);
        for source_id in string_array_attribute(record.attributes.get("sources")) {
            provenance
                .sources
                .push(SourceRef(normalize_source_id(&source_id)));
        }
        let event = GenealogyEvent {
            id: event_id,
            kind: event_type_from_local_type(
                local_event_type_id(record)
                    .as_deref()
                    .unwrap_or(record.kind.as_str()),
            ),
            date: record
                .date
                .as_ref()
                .or_else(|| {
                    record
                        .attributes
                        .get("time")
                        .and_then(toml_json_value_as_string_ref)
                })
                .map(|date| {
                    DateValue::from_original(date.clone(), local_record_provenance(record))
                }),
            time: record
                .attributes
                .get("time")
                .and_then(toml_json_value_as_string),
            time_zone: record
                .attributes
                .get("time_zone")
                .and_then(toml_json_value_as_string),
            place: None,
            description: markdown_title(record)
                .or_else(|| record.summary.clone())
                .or_else(|| {
                    default_event_label(record, |entity| person_names.get(entity).cloned())
                }),
            participants: participants.clone(),
            provenance,
        };

        tree.events.push(event);
        for person_id in participants {
            if let Some(person) = tree.people.iter_mut().find(|person| person.id == person_id) {
                person.events.push(event_id);
            }
        }
    }

    for document in bundle
        .toml_documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("relationship"))
    {
        let source = required_json_string(document, "source")?;
        let target = required_json_string(document, "target")?;
        let source_key = resolve_person_key(source, &person_ids);
        let target_key = resolve_person_key(target, &person_ids);
        let source_id = person_ids.get(&source_key).copied().ok_or_else(|| {
            LocalAuthoringError::Validation {
                message: format!(
                    "{} references missing source person `{source_key}`",
                    document.path
                ),
            }
        })?;
        let target_id = person_ids.get(&target_key).copied().ok_or_else(|| {
            LocalAuthoringError::Validation {
                message: format!(
                    "{} references missing target person `{target_key}`",
                    document.path
                ),
            }
        })?;
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
        let relationship_kind = RelationshipKind::from_value(kind);
        if !relationship_kind_allowed(&relationship_kind, relationship_filter.as_deref()) {
            continue;
        }
        let relationship_id = tree.add_relationship(relationship_kind, source_id, target_id);
        if let Some(relationship) = tree
            .relationships
            .iter_mut()
            .find(|relationship| relationship.id == relationship_id)
        {
            relationship.label = document.title.clone();
            relationship.parent_role =
                relationship_parent_role(document, &relationship.kind, &source_key, &person_sexes);
            relationship.provenance.sources.push(SourceRef(format!(
                "local:{}",
                document.id.clone().unwrap_or_else(|| document.path.clone())
            )));
        }
    }

    apply_standard_married_name_defaults(
        &mut tree,
        &person_ids,
        &person_sexes,
        &explicit_preferred_surnames,
    );

    if let Some(main_person) = tree_view
        .and_then(|document| document.data.get("root"))
        .and_then(|root| root.get("entity"))
        .and_then(serde_json::Value::as_str)
        .map(|id| resolve_person_key(id, &person_ids))
        .and_then(|id| person_ids.get(&id).copied())
        .or_else(|| {
            registry
                .and_then(|document| document.data.get("tree"))
                .and_then(|tree| tree.get("main_person"))
                .and_then(serde_json::Value::as_str)
                .map(|id| resolve_person_key(id, &person_ids))
                .and_then(|id| person_ids.get(&id).copied())
        })
        .or_else(|| tree.people.first().map(|person| person.id))
    {
        tree.main_person = Some(main_person);
    }

    if let (Some(document), Some(main_person)) = (tree_view, tree.main_person)
        && tree_view_has_root(document)
    {
        apply_tree_generation_filter(&mut tree, document, main_person);
    }

    if !has_explicit_layout_positions {
        apply_default_tree_layout(&mut tree);
    }

    Ok(tree)
}

fn relationship_parent_role(
    document: &LocalTomlDocument,
    relationship_kind: &RelationshipKind,
    source_key: &str,
    person_sexes: &BTreeMap<String, Sex>,
) -> Option<ParentRole> {
    if !relationship_kind.is_parent_child() {
        return None;
    }

    document
        .data
        .get("parent_role")
        .and_then(serde_json::Value::as_str)
        .and_then(ParentRole::from_value)
        .or_else(|| person_sexes.get(source_key).and_then(ParentRole::from_sex))
}

fn apply_default_tree_layout(tree: &mut TreeDocument) {
    let Some(root) = tree
        .main_person
        .or_else(|| tree.people.first().map(|person| person.id))
    else {
        return;
    };

    let mut levels = BTreeMap::<PersonId, i32>::new();
    levels.insert(root, 0);
    let mut queue = VecDeque::from([root]);

    while let Some(person_id) = queue.pop_front() {
        let Some(level) = levels.get(&person_id).copied() else {
            continue;
        };

        for parent in tree_parent_ids(tree, person_id) {
            if let std::collections::btree_map::Entry::Vacant(entry) = levels.entry(parent) {
                entry.insert(level - 1);
                queue.push_back(parent);
            }
        }
        for child in tree_child_ids(tree, person_id) {
            if let std::collections::btree_map::Entry::Vacant(entry) = levels.entry(child) {
                entry.insert(level + 1);
                queue.push_back(child);
            }
        }
        for spouse in tree_spouse_ids(tree, person_id) {
            if let std::collections::btree_map::Entry::Vacant(entry) = levels.entry(spouse) {
                entry.insert(level);
                queue.push_back(spouse);
            }
        }
    }

    for person in &tree.people {
        levels.entry(person.id).or_insert(0);
    }

    let mut rows = BTreeMap::<i32, Vec<PersonId>>::new();
    for (person_id, level) in levels {
        rows.entry(level).or_default().push(person_id);
    }

    let min_level = rows.keys().next().copied().unwrap_or(0);
    let center_x = 760.0;
    let start_y = 180.0;
    let x_gap = 260.0;
    let y_gap = 170.0;

    for (level, mut people) in rows {
        people.sort_by_key(|person_id| person_id.0);
        let total_width = people.len().saturating_sub(1) as f32 * x_gap;
        let start_x = center_x - total_width / 2.0;
        let y = start_y + (level - min_level) as f32 * y_gap;
        for (index, person_id) in people.into_iter().enumerate() {
            tree.layout
                .set_position(person_id, start_x + index as f32 * x_gap, y);
        }
    }
}

fn tree_parent_ids(tree: &TreeDocument, child: PersonId) -> Vec<PersonId> {
    tree.relationships
        .iter()
        .filter(|relationship| relationship.kind.is_parent_child() && relationship.target == child)
        .map(|relationship| relationship.source)
        .collect()
}

fn tree_child_ids(tree: &TreeDocument, parent: PersonId) -> Vec<PersonId> {
    tree.relationships
        .iter()
        .filter(|relationship| relationship.kind.is_parent_child() && relationship.source == parent)
        .map(|relationship| relationship.target)
        .collect()
}

fn tree_spouse_ids(tree: &TreeDocument, person_id: PersonId) -> Vec<PersonId> {
    tree.relationships
        .iter()
        .filter(|relationship| {
            matches!(
                relationship.kind,
                RelationshipKind::Spouse
                    | RelationshipKind::Partner
                    | RelationshipKind::FormerSpouse
            ) && (relationship.source == person_id || relationship.target == person_id)
        })
        .map(|relationship| {
            if relationship.source == person_id {
                relationship.target
            } else {
                relationship.source
            }
        })
        .collect()
}

fn apply_standard_married_name_defaults(
    tree: &mut TreeDocument,
    person_ids: &BTreeMap<String, PersonId>,
    person_sexes: &BTreeMap<String, Sex>,
    explicit_preferred_surnames: &BTreeSet<String>,
) {
    let person_keys_by_id = person_ids
        .iter()
        .map(|(key, id)| (*id, key.as_str()))
        .collect::<BTreeMap<_, _>>();
    let effective_person_sexes = effective_person_sexes(tree, person_ids, person_sexes);
    let former_spouse_pairs = tree
        .relationships
        .iter()
        .filter(|relationship| relationship.kind == RelationshipKind::FormerSpouse)
        .map(|relationship| ordered_person_pair(relationship.source, relationship.target))
        .collect::<BTreeSet<_>>();
    let spouse_pairs = tree
        .relationships
        .iter()
        .filter(|relationship| relationship.kind == RelationshipKind::Spouse)
        .filter(|relationship| {
            !former_spouse_pairs.contains(&ordered_person_pair(
                relationship.source,
                relationship.target,
            ))
        })
        .map(|relationship| (relationship.source, relationship.target))
        .collect::<Vec<_>>();
    let mut surname_sources_by_person = BTreeMap::<PersonId, Vec<PersonId>>::new();

    for (source, target) in spouse_pairs {
        let Some(source_key) = person_keys_by_id.get(&source).copied() else {
            continue;
        };
        let Some(target_key) = person_keys_by_id.get(&target).copied() else {
            continue;
        };

        match (
            effective_person_sexes.get(source_key),
            effective_person_sexes.get(target_key),
        ) {
            (Some(Sex::Female), Some(Sex::Male)) => surname_sources_by_person
                .entry(source)
                .or_default()
                .push(target),
            (Some(Sex::Male), Some(Sex::Female)) => surname_sources_by_person
                .entry(target)
                .or_default()
                .push(source),
            _ => {}
        }
    }

    for (person_id, spouse_ids) in surname_sources_by_person {
        let [spouse_id] = spouse_ids.as_slice() else {
            continue;
        };
        let Some(person_key) = person_keys_by_id.get(&person_id).copied() else {
            continue;
        };
        inherit_spouse_surname(
            tree,
            person_id,
            *spouse_id,
            person_key,
            explicit_preferred_surnames,
        );
    }
}

fn ordered_person_pair(first: PersonId, second: PersonId) -> (PersonId, PersonId) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

fn effective_person_sexes(
    tree: &TreeDocument,
    person_ids: &BTreeMap<String, PersonId>,
    person_sexes: &BTreeMap<String, Sex>,
) -> BTreeMap<String, Sex> {
    let mut effective = person_sexes.clone();
    let person_keys_by_id = person_ids
        .iter()
        .map(|(key, id)| (*id, key.as_str()))
        .collect::<BTreeMap<_, _>>();

    for relationship in &tree.relationships {
        let Some(parent_role) = relationship.parent_role else {
            continue;
        };
        let Some(person_key) = person_keys_by_id.get(&relationship.source) else {
            continue;
        };
        let sex = match parent_role {
            ParentRole::Father => Sex::Male,
            ParentRole::Mother => Sex::Female,
            ParentRole::Parent | ParentRole::Unknown => continue,
        };
        effective.entry((*person_key).to_string()).or_insert(sex);
    }

    effective
}

fn inherit_spouse_surname(
    tree: &mut TreeDocument,
    person_id: PersonId,
    spouse_id: PersonId,
    person_key: &str,
    explicit_preferred_surnames: &BTreeSet<String>,
) {
    if explicit_preferred_surnames.contains(person_key) {
        return;
    }

    let Some(spouse_surname) = tree
        .people
        .iter()
        .find(|person| person.id == spouse_id)
        .and_then(primary_surname)
        .map(ToOwned::to_owned)
    else {
        return;
    };

    let Some(person) = tree.people.iter_mut().find(|person| person.id == person_id) else {
        return;
    };
    let Some(preferred_index) = preferred_name_index(person) else {
        return;
    };

    let preferred = &mut person.names[preferred_index];
    if preferred.surname.as_deref() == Some(spouse_surname.as_str()) {
        return;
    }

    preferred.surname = Some(spouse_surname);
    update_name_display_from_parts(preferred);
}

fn preferred_name_index(person: &Person) -> Option<usize> {
    person
        .names
        .iter()
        .position(|name| name.usage.as_deref() == Some("preferred"))
        .or_else(|| (!person.names.is_empty()).then_some(0))
}

fn primary_surname(person: &Person) -> Option<&str> {
    person
        .names
        .iter()
        .find(|name| name.usage.as_deref() == Some("preferred"))
        .and_then(|name| name.surname.as_deref())
        .or_else(|| person.names.iter().find_map(|name| name.surname.as_deref()))
}

fn update_name_display_from_parts(name: &mut Name) {
    if let Some(order) = name.order.as_ref()
        && !order.supports_family_name_rewrite()
    {
        return;
    }

    let order = name
        .order
        .as_ref()
        .cloned()
        .unwrap_or_else(|| NameOrder(NameOrder::GIVEN_MIDDLE_FAMILY.to_string()));
    if let Some(display) = order.format_parts(
        name.given.as_deref(),
        name.middle.as_deref(),
        name.surname.as_deref(),
    ) {
        name.display = display;
    }
}

fn select_tree_view<'a>(
    documents: &'a [LocalTomlDocument],
    view_slug: Option<&str>,
) -> Option<&'a LocalTomlDocument> {
    let trees = documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("tree-view"));

    if let Some(view_slug) = view_slug {
        let view_id = format!("tree:{view_slug}");
        return trees.into_iter().find(|document| {
            document.id.as_deref() == Some(view_id.as_str())
                || document.path == format!("views/trees/{view_slug}.toml")
        });
    }

    trees.into_iter().next()
}

fn tree_view_relationship_filter(document: &LocalTomlDocument) -> Option<Vec<String>> {
    tree_view_projection_table(document)
        .and_then(|projection| projection.get("relationship_kinds"))
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
}

fn tree_view_projection_table(document: &LocalTomlDocument) -> Option<&serde_json::Value> {
    document
        .data
        .get("projection")
        .or_else(|| document.data.get("filter"))
}

fn tree_view_projection_u64_filter(document: &LocalTomlDocument, key: &str) -> Option<u64> {
    tree_view_projection_table(document)
        .and_then(|projection| projection.get(key))
        .and_then(serde_json::Value::as_u64)
}

fn tree_view_projection_bool_filter(
    document: &LocalTomlDocument,
    key: &str,
    default: bool,
) -> bool {
    tree_view_projection_table(document)
        .and_then(|projection| projection.get(key))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default)
}

fn apply_tree_generation_filter(
    tree: &mut TreeDocument,
    document: &LocalTomlDocument,
    root: PersonId,
) {
    let generations_up =
        tree_view_projection_u64_filter(document, "generations_up").map(|value| value as u32);
    let generations_down =
        tree_view_projection_u64_filter(document, "generations_down").map(|value| value as u32);
    if generations_up.is_none() && generations_down.is_none() {
        return;
    }

    let mut parents_by_child = BTreeMap::<PersonId, Vec<PersonId>>::new();
    let mut children_by_parent = BTreeMap::<PersonId, Vec<PersonId>>::new();
    let mut spouses_by_person = BTreeMap::<PersonId, Vec<PersonId>>::new();
    for relationship in &tree.relationships {
        if relationship.kind.is_parent_child() {
            parents_by_child
                .entry(relationship.target)
                .or_default()
                .push(relationship.source);
            children_by_parent
                .entry(relationship.source)
                .or_default()
                .push(relationship.target);
        } else if matches!(
            relationship.kind,
            RelationshipKind::Spouse | RelationshipKind::Partner | RelationshipKind::FormerSpouse
        ) {
            spouses_by_person
                .entry(relationship.source)
                .or_default()
                .push(relationship.target);
            spouses_by_person
                .entry(relationship.target)
                .or_default()
                .push(relationship.source);
        }
    }

    let mut keep = BTreeSet::from([root]);
    if let Some(generations_up) = generations_up {
        collect_tree_relatives(root, generations_up, &parents_by_child, &mut keep);
    }
    if let Some(generations_down) = generations_down {
        collect_tree_relatives(root, generations_down, &children_by_parent, &mut keep);
    }
    if tree_view_projection_bool_filter(document, "include_partners", true) {
        for person in keep.clone() {
            if let Some(spouses) = spouses_by_person.get(&person) {
                keep.extend(spouses.iter().copied());
            }
        }
    }
    if tree_view_projection_bool_filter(document, "include_siblings", false) {
        for person in keep.clone() {
            if let Some(siblings) = sibling_ids_in_tree(tree, person, &parents_by_child) {
                keep.extend(siblings);
            }
        }
    }
    if tree_view_projection_bool_filter(document, "include_unconnected", false) {
        keep.extend(tree.people.iter().map(|person| person.id));
    }

    tree.people.retain(|person| keep.contains(&person.id));
    tree.relationships.retain(|relationship| {
        keep.contains(&relationship.source) && keep.contains(&relationship.target)
    });
    tree.events.retain(|event| {
        event
            .participants
            .iter()
            .any(|person| keep.contains(person))
    });
    let kept_events = tree
        .events
        .iter()
        .map(|event| event.id)
        .collect::<BTreeSet<_>>();
    for person in &mut tree.people {
        person.events.retain(|event| kept_events.contains(event));
    }
    tree.layout
        .nodes
        .retain(|node| keep.contains(&node.person_id));
}

fn tree_view_has_root(document: &LocalTomlDocument) -> bool {
    document
        .data
        .get("root")
        .and_then(|root| root.get("entity"))
        .and_then(serde_json::Value::as_str)
        .is_some()
}

fn sibling_ids_in_tree(
    tree: &TreeDocument,
    person: PersonId,
    parents_by_child: &BTreeMap<PersonId, Vec<PersonId>>,
) -> Option<Vec<PersonId>> {
    let parents = parents_by_child
        .get(&person)?
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    Some(
        tree.relationships
            .iter()
            .filter(|relationship| {
                relationship.kind.is_parent_child()
                    && relationship.target != person
                    && parents.contains(&relationship.source)
            })
            .map(|relationship| relationship.target)
            .collect(),
    )
}

fn collect_tree_relatives(
    root: PersonId,
    max_depth: u32,
    edges: &BTreeMap<PersonId, Vec<PersonId>>,
    keep: &mut BTreeSet<PersonId>,
) {
    let mut queue = VecDeque::from([(root, 0)]);
    let mut visited = BTreeSet::from([root]);
    while let Some((person, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let Some(next_people) = edges.get(&person) else {
            continue;
        };
        for next in next_people {
            if visited.insert(*next) {
                keep.insert(*next);
                queue.push_back((*next, depth + 1));
            }
        }
    }
}

fn relationship_kind_allowed(kind: &RelationshipKind, filter: Option<&[String]>) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    filter
        .iter()
        .any(|value| relationship_kind_matches(kind, value))
}

fn relationship_kind_matches(kind: &RelationshipKind, filter_value: &str) -> bool {
    let normalized = filter_value.trim();
    if normalized == kind.as_value() {
        return true;
    }

    match normalized {
        "parent" | "child" | "parent-child" => kind.is_parent_child(),
        "partner-or-spouse" => matches!(
            kind,
            RelationshipKind::Spouse | RelationshipKind::Partner | RelationshipKind::FormerSpouse
        ),
        _ => false,
    }
}

fn preferred_name_table(record: &LocalMarkdownRecord) -> Option<&serde_json::Value> {
    record.attributes.get("names")?.get("preferred")
}

fn legal_name_table(record: &LocalMarkdownRecord) -> Option<&serde_json::Value> {
    record.attributes.get("names")?.get("legal")
}

fn name_table_string(table: &serde_json::Value, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn name_order_from_table(table: &serde_json::Value) -> Option<NameOrder> {
    name_table_string(table, "order").and_then(NameOrder::new)
}

fn has_explicit_preferred_surname(record: &LocalMarkdownRecord) -> bool {
    record.name_hints.explicit_preferred_surname
}

fn name_display_from_table(table: &serde_json::Value) -> Option<String> {
    name_table_string(table, "display")
        .or_else(|| name_table_string(table, "full"))
        .or_else(|| {
            let given = name_table_string(table, "given");
            let middle = name_table_string(table, "middle");
            let family = name_table_string(table, "family");
            let surname = name_table_string(table, "surname");
            let order = name_order_from_table(table)
                .unwrap_or_else(|| NameOrder(NameOrder::GIVEN_MIDDLE_FAMILY.to_string()));
            order.format_parts(
                given.as_deref(),
                middle.as_deref(),
                family.or(surname).as_deref(),
            )
        })
}

fn name_from_table(usage: &str, table: &serde_json::Value, provenance: Provenance) -> Option<Name> {
    Some(Name {
        usage: Some(usage.to_string()),
        display: name_display_from_table(table)?,
        full: name_table_string(table, "full"),
        given: name_table_string(table, "given"),
        middle: name_table_string(table, "middle"),
        surname: name_table_string(table, "family").or_else(|| name_table_string(table, "surname")),
        order: name_order_from_table(table),
        aliases: string_array_attribute(table.get("aliases")),
        provenance,
    })
}

fn person_names_from_record(record: &LocalMarkdownRecord, display: String) -> Vec<Name> {
    let provenance = local_record_provenance(record);
    let mut names = Vec::new();

    if let Some(preferred) = preferred_name_table(record)
        && let Some(name) = name_from_table("preferred", preferred, provenance.clone())
    {
        names.push(name);
    }
    if let Some(legal) = legal_name_table(record)
        && let Some(name) = name_from_table("legal", legal, provenance.clone())
    {
        names.push(name);
    }

    if names.is_empty() {
        names.push(Name {
            usage: Some("legal".to_string()),
            display,
            full: legal_name_table(record).and_then(|legal| name_table_string(legal, "full")),
            given: markdown_given(record),
            middle: legal_name_table(record).and_then(|legal| name_table_string(legal, "middle")),
            surname: markdown_surname(record),
            order: Some(NameOrder(NameOrder::GIVEN_MIDDLE_FAMILY.to_string())),
            aliases: string_array_attribute(record.attributes.get("aliases")),
            provenance,
        });
    }

    names
}

fn markdown_title(record: &LocalMarkdownRecord) -> Option<String> {
    record
        .title
        .clone()
        .or_else(|| preferred_name_table(record).and_then(name_display_from_table))
        .or_else(|| legal_name_table(record).and_then(name_display_from_table))
}

fn markdown_given(record: &LocalMarkdownRecord) -> Option<String> {
    record
        .attributes
        .get("given")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            record
                .attributes
                .get("names")
                .and_then(|names| names.get("preferred"))
                .and_then(|preferred| preferred.get("given"))
                .and_then(serde_json::Value::as_str)
        })
        .map(ToOwned::to_owned)
}

fn markdown_surname(record: &LocalMarkdownRecord) -> Option<String> {
    record
        .attributes
        .get("surname")
        .or_else(|| record.attributes.get("family"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            record
                .attributes
                .get("names")
                .and_then(|names| names.get("preferred"))
                .and_then(|preferred| preferred.get("family").or_else(|| preferred.get("surname")))
                .and_then(serde_json::Value::as_str)
        })
        .map(ToOwned::to_owned)
}

fn is_timeline_event_record(record: &LocalMarkdownRecord) -> bool {
    record.kind == "event"
}

fn event_type_from_local_type(event_type: &str) -> GenealogyEventKind {
    match event_type {
        "birth" => GenealogyEventKind::Birth,
        "death" => GenealogyEventKind::Death,
        "marriage" => GenealogyEventKind::Marriage,
        "baptism" => GenealogyEventKind::Baptism,
        "burial" => GenealogyEventKind::Burial,
        "residence" => GenealogyEventKind::Residence,
        "occupation" => GenealogyEventKind::Occupation,
        other => GenealogyEventKind::Other(other.to_string()),
    }
}

fn event_participants(
    record: &LocalMarkdownRecord,
    person_ids: &BTreeMap<String, PersonId>,
) -> Result<Vec<PersonId>, LocalAuthoringError> {
    if !record.attributes.contains_key("participants") && !record.attributes.contains_key("subject")
    {
        return Ok(record
            .related
            .iter()
            .filter_map(|id| person_ids.get(id).copied())
            .collect());
    }

    if let Some(value) = record.attributes.get("participants")
        && !value.is_array()
    {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `participants` must be an array", record.path),
        });
    }

    let mut participants = Vec::new();
    for item in normalized_event_participants(record) {
        let entity_id = item
            .get("entity")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| LocalAuthoringError::Validation {
                message: format!("{} participant missing `entity`", record.path),
            })?;
        let person_key = resolve_person_key(entity_id, person_ids);
        let person_id = person_ids.get(&person_key).copied().ok_or_else(|| {
            LocalAuthoringError::Validation {
                message: format!(
                    "{} references missing participant `{person_key}`",
                    record.path
                ),
            }
        })?;
        if !participants.contains(&person_id) {
            participants.push(person_id);
        }
    }

    Ok(participants)
}

fn resolve_person_key(value: &str, person_ids: &BTreeMap<String, PersonId>) -> String {
    if person_ids.contains_key(value) {
        value.to_string()
    } else {
        resolve_contextual_id(value, normalize_person_id)
    }
}

fn required_json_string<'a>(
    document: &'a LocalTomlDocument,
    key: &str,
) -> Result<&'a str, LocalAuthoringError> {
    document
        .data
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| LocalAuthoringError::Validation {
            message: format!("{} missing required `{key}`", document.path),
        })
}

fn parse_sex(value: &str) -> Sex {
    match value {
        "male" | "m" | "Male" => Sex::Male,
        "female" | "f" | "Female" => Sex::Female,
        "other" | "Other" => Sex::Other,
        _ => Sex::Unknown,
    }
}

fn add_person_life_event(
    tree: &mut TreeDocument,
    person_id: PersonId,
    record: &LocalMarkdownRecord,
    field: &str,
    kind: GenealogyEventKind,
) {
    let Some(date) = record
        .attributes
        .get(field)
        .and_then(toml_json_value_as_string)
    else {
        return;
    };

    let id = next_event_id(tree);
    let label = match kind {
        GenealogyEventKind::Birth => "Birth",
        GenealogyEventKind::Death => "Death",
        _ => "Life event",
    };
    let event = GenealogyEvent {
        id,
        kind,
        date: Some(DateValue::from_original(
            date,
            local_record_provenance(record),
        )),
        time: record
            .attributes
            .get(&format!("{field}_time"))
            .and_then(toml_json_value_as_string),
        time_zone: record
            .attributes
            .get(&format!("{field}_time_zone"))
            .and_then(toml_json_value_as_string),
        place: None,
        description: Some(format!("{label} for {}", record.id)),
        participants: vec![person_id],
        provenance: local_record_provenance(record),
    };

    tree.events.push(event);
    if let Some(person) = tree.people.iter_mut().find(|person| person.id == person_id) {
        person.events.push(id);
    }
}

fn next_event_id(tree: &TreeDocument) -> EventId {
    EventId(
        tree.events
            .iter()
            .map(|event| event.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

fn toml_json_value_as_string_ref(value: &serde_json::Value) -> Option<&String> {
    value.as_str()?;
    match value {
        serde_json::Value::String(value) => Some(value),
        _ => None,
    }
}

fn toml_json_value_as_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
        .or_else(|| value.as_u64().map(|value| value.to_string()))
        .or_else(|| value.as_f64().map(|value| value.to_string()))
}

fn local_record_provenance(record: &LocalMarkdownRecord) -> Provenance {
    let mut provenance = Provenance::default();
    provenance
        .sources
        .push(SourceRef(format!("local:{}", record.id)));
    provenance.attributes.push(Attribute {
        key: "local_path".to_string(),
        value: record.path.clone(),
    });
    if !record.notes_markdown.is_empty() {
        provenance.attributes.push(Attribute {
            key: "notes_markdown".to_string(),
            value: record.notes_markdown.clone(),
        });
    }
    if let Some(date) = &record.date {
        provenance.attributes.push(Attribute {
            key: "date".to_string(),
            value: date.clone(),
        });
    }
    if let Some(summary) = &record.summary {
        provenance.attributes.push(Attribute {
            key: "summary".to_string(),
            value: summary.clone(),
        });
    }
    provenance
}

fn string_array_attribute(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn numeric_attribute(value: Option<&serde_json::Value>) -> Option<f32> {
    value
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
}
