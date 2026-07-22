use super::*;
use crate::RelationshipKind;

#[test]
fn parses_markdown_with_toml_frontmatter() {
    let text = "+++\nid = \"person_alex_example\"\nkind = \"person\"\ndate = 1900-01-01\ntags = [\"example\"]\n+++\n\nNarrative note.\n";
    let path = Path::new("person_alex_example.md");
    let (frontmatter, notes) = split_toml_frontmatter(path, text).expect("frontmatter");
    let mut table = frontmatter.parse::<toml::Table>().expect("toml table");

    assert_eq!(
        take_required_string(&mut table, "id", path).unwrap(),
        "person_alex_example"
    );
    assert_eq!(
        take_optional_string(&mut table, "date", path).unwrap(),
        Some("1900-01-01".to_string())
    );
    assert_eq!(notes.trim(), "Narrative note.");
}

#[test]
fn compiles_markdown_and_toml_into_bundle() {
    let temp_dir = test_temp_dir("bundle");
    fs::create_dir_all(temp_dir.join("records")).expect("records dir");
    fs::create_dir_all(temp_dir.join("places")).expect("places dir");
    fs::create_dir_all(temp_dir.join("compiled")).expect("compiled dir");
    fs::write(temp_dir.join("README.md"), "# ignored docs\n").expect("readme");
    fs::write(temp_dir.join("compiled/old.json"), "{\"ignored\":true}\n").expect("compiled output");
    fs::write(
        temp_dir.join("places/place_example_town.toml"),
        "id = \"place_example_town\"\nkind = \"place\"\ntitle = \"Example Town\"\n",
    )
    .expect("place toml");
    fs::write(
            temp_dir.join("records/person_alex_example.md"),
            "+++\nid = \"person_alex_example\"\nkind = \"person\"\ntitle = \"Alex Example\"\ndate = 1900-01-01\nrelated = []\nplace = \"place_example_town\"\ncustom_field = \"kept\"\n+++\n\n# Note\n",
        )
        .expect("record markdown");

    let bundle = compile_local_data(&temp_dir).expect("compile local data");

    assert_eq!(bundle.markdown_records.len(), 1);
    assert_eq!(bundle.toml_documents.len(), 1);
    assert_eq!(bundle.markdown_records[0].id, "person_alex_example");
    assert_eq!(
        bundle.markdown_records[0].date.as_deref(),
        Some("1900-01-01")
    );
    assert_eq!(
        bundle.markdown_records[0].place.as_deref(),
        Some("place_example_town")
    );
    assert_eq!(
        bundle.markdown_records[0].attributes.get("custom_field"),
        Some(&serde_json::Value::String("kept".to_string()))
    );
    assert_eq!(
        bundle.toml_documents[0].path,
        "places/place_example_town.toml"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_missing_related_record() {
    let temp_dir = test_temp_dir("missing-related");
    fs::create_dir_all(temp_dir.join("records")).expect("records dir");
    fs::write(
            temp_dir.join("records/person_alex_example.md"),
            "+++\nid = \"person_alex_example\"\nkind = \"person\"\nrelated = [\"person_missing_example\"]\n+++\n\n# Note\n",
        )
        .expect("record markdown");

    let err = compile_local_data(&temp_dir).expect_err("missing related should fail");
    assert!(
        err.to_string().contains("person_missing_example"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn writes_compiled_json() {
    let temp_dir = test_temp_dir("write-json");
    fs::create_dir_all(temp_dir.join("places")).expect("places dir");
    fs::write(
        temp_dir.join("places/place_example_town.toml"),
        "id = \"place_example_town\"\nkind = \"place\"\ntitle = \"Example Town\"\n",
    )
    .expect("place toml");

    let output_path = temp_dir.join("compiled/kleio-local-data.json");
    let bundle = write_local_data_json(&temp_dir, &output_path).expect("write json");
    let json = fs::read_to_string(&output_path).expect("compiled json");

    assert_eq!(bundle.toml_documents.len(), 1);
    assert!(json.contains("place_example_town"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn compiles_private_tree_document_from_person_records_and_relationships() {
    let temp_dir = test_temp_dir("tree");
    fs::create_dir_all(temp_dir.join("records")).expect("records dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
            temp_dir.join("registry.toml"),
            "id = \"registry_private_tree\"\nkind = \"registry\"\ntitle = \"Private registry\"\n\n[tree]\nid = \"private-tree\"\ntitle = \"Private tree\"\nmain_person = \"person_alex_example\"\n",
        )
        .expect("registry");
    fs::write(
            temp_dir.join("records/person_alex_example.md"),
            "+++\nid = \"person_alex_example\"\nkind = \"person\"\ntitle = \"Alex Example\"\ngiven = \"Alex\"\nsurname = \"Example\"\nsex = \"unknown\"\nbirth_date = 1900-01-01\nx = 10\ny = 20\nrelated = [\"person_morgan_example\"]\n+++\n\n# Alex note\n",
        )
        .expect("alex");
    fs::write(
            temp_dir.join("records/person_morgan_example.md"),
            "+++\nid = \"person_morgan_example\"\nkind = \"person\"\ntitle = \"Morgan Example\"\nrelated = [\"person_alex_example\"]\n+++\n\n# Morgan note\n",
        )
        .expect("morgan");
    fs::write(
            temp_dir.join("relationships/alex_morgan.toml"),
            "id = \"relationship_alex_morgan_example\"\nkind = \"relationship\"\ntitle = \"Example association\"\nrelationship = \"associate\"\nsource = \"person_alex_example\"\ntarget = \"person_morgan_example\"\n",
        )
        .expect("relationship");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");

    assert_eq!(tree.metadata.id.0, "private-tree");
    assert_eq!(tree.metadata.title, "Private tree");
    assert_eq!(tree.people.len(), 2);
    assert_eq!(tree.events.len(), 1);
    assert_eq!(tree.relationships.len(), 1);
    assert_eq!(
        tree.person_display_name(tree.main_person.expect("main person")),
        Some("Alex Example")
    );
    assert_eq!(tree.relationships[0].kind, RelationshipKind::Associate);
    assert_eq!(tree.layout.position(tree.people[0].id), Some((10.0, 20.0)));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn compiles_local_event_collections_into_timeline_projection() {
    let temp_dir = test_temp_dir("timeline-collections");
    fs::create_dir_all(temp_dir.join("events/observations")).expect("events dir");
    fs::create_dir_all(temp_dir.join("collections")).expect("collections dir");
    fs::write(
        temp_dir.join("events/observations/first.md"),
        "+++\nid = \"event:first\"\nkind = \"observation\"\ntitle = \"First event\"\ndate = 2026-01-01\n+++\n\n# First\n",
    )
    .expect("first event");
    fs::write(
        temp_dir.join("events/observations/second.md"),
        "+++\nid = \"event:second\"\nkind = \"observation\"\ntitle = \"Second event\"\ndate = 2026-01-02\n+++\n\n# Second\n",
    )
    .expect("second event");
    fs::write(
        temp_dir.join("collections/comparison.toml"),
        "schema_version = 1\nid = \"collection:comparison\"\nkind = \"event-collection\"\ntitle = \"Comparison\"\ncollection_kind = \"set\"\n\n[[members]]\nevent = \"event:first\"\nlabel = \"First\"\nrole = \"reference\"\n\n[[members]]\nevent = \"event:second\"\nlabel = \"Second\"\nrole = \"comparison\"\n",
    )
    .expect("collection");

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(timeline.events.len(), 2);
    assert_eq!(timeline.collections.len(), 1);
    assert_eq!(timeline.collections[0].id, "collection:comparison");
    assert_eq!(timeline.collections[0].members.len(), 2);
    assert_eq!(timeline.collections[0].members[0].event, "event:first");

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_local_event_collection_missing_member() {
    let temp_dir = test_temp_dir("collection-missing-member");
    fs::create_dir_all(temp_dir.join("collections")).expect("collections dir");
    fs::write(
        temp_dir.join("collections/missing.toml"),
        "schema_version = 1\nid = \"collection:missing\"\nkind = \"event-collection\"\ntitle = \"Missing\"\ncollection_kind = \"set\"\n\n[[members]]\nevent = \"event:missing\"\n",
    )
    .expect("collection");

    let err = compile_local_data(&temp_dir).expect_err("missing collection member should fail");
    assert!(
        err.to_string().contains("event:missing"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn writes_private_tree_json() {
    let temp_dir = test_temp_dir("write-tree-json");
    fs::create_dir_all(temp_dir.join("records")).expect("records dir");
    fs::write(
            temp_dir.join("records/person_alex_example.md"),
            "+++\nid = \"person_alex_example\"\nkind = \"person\"\ntitle = \"Alex Example\"\n+++\n\n# Note\n",
        )
        .expect("person");

    let output_path = temp_dir.join("compiled/kleio-tree.json");
    let tree = write_local_tree_json(&temp_dir, &output_path).expect("write tree json");
    let json = fs::read_to_string(&output_path).expect("compiled tree json");

    assert_eq!(tree.people.len(), 1);
    assert!(json.contains("person_alex_example"));
    assert!(json.contains("Alex Example"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_missing_event_assertion_reference() {
    let temp_dir = test_temp_dir("missing-assertion");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/person-alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\nprimary_name = \"Alex Example\"\n+++\n\n# Note\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
        "+++\nid = \"event:birth-alex-example\"\nkind = \"birth\"\nparticipants = [{ entity = \"person:alex-example\", role = \"subject\" }]\nassertions = [\"assertion:missing\"]\n+++\n\n# Note\n",
    )
    .expect("event");

    let err = compile_local_data(&temp_dir).expect_err("missing assertion should fail");
    assert!(
        err.to_string().contains("assertion:missing"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_missing_assertion_target_reference() {
    let temp_dir = test_temp_dir("missing-assertion-target");
    fs::create_dir_all(temp_dir.join("assertions")).expect("assertions dir");
    fs::write(
        temp_dir.join("assertions/example-claim.md"),
        "+++\nid = \"assertion:example-claim\"\nkind = \"identity\"\ntarget = \"person:missing#name\"\nvalue = \"Missing Example\"\n+++\n\n# Note\n",
    )
    .expect("assertion");

    let err = compile_local_data(&temp_dir).expect_err("missing target should fail");
    assert!(
        err.to_string().contains("person:missing"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_missing_relationship_reference() {
    let temp_dir = test_temp_dir("missing-relationship");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
        temp_dir.join("entities/people/person-alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\nprimary_name = \"Alex Example\"\n+++\n\n# Note\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("relationships/alex-missing.toml"),
        "id = \"relationship:alex-missing\"\nkind = \"relationship\"\ntitle = \"Missing relation\"\nrelationship = \"associate\"\nsource = \"person:alex-example\"\ntarget = \"person:missing\"\n",
    )
    .expect("relationship");

    let err = compile_local_data(&temp_dir).expect_err("missing relationship target should fail");
    assert!(
        err.to_string().contains("person:missing"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn filters_tree_view_by_configured_generations() {
    let temp_dir = test_temp_dir("tree-generations");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::create_dir_all(temp_dir.join("views/trees")).expect("tree views dir");
    for slug in ["grandparent", "parent", "root", "child", "grandchild"] {
        fs::write(
            temp_dir.join(format!("entities/people/{slug}.md")),
            format!(
                "+++\nid = \"person:{slug}\"\nkind = \"person\"\nprimary_name = \"{slug}\"\n+++\n\n# Note\n"
            ),
        )
        .expect("person");
    }
    for (slug, source, target) in [
        ("grandparent-parent", "person:grandparent", "person:parent"),
        ("parent-root", "person:parent", "person:root"),
        ("root-child", "person:root", "person:child"),
        ("child-grandchild", "person:child", "person:grandchild"),
    ] {
        fs::write(
            temp_dir.join(format!("relationships/{slug}.toml")),
            format!(
                "id = \"relationship:{slug}\"\nkind = \"relationship\"\nrelationship = \"biological-parent-child\"\nsource = \"{source}\"\ntarget = \"{target}\"\n"
            ),
        )
        .expect("relationship");
    }
    fs::write(
        temp_dir.join("views/trees/root-tree.toml"),
        "schema_version = 1\nid = \"tree:root-tree\"\nkind = \"tree-view\"\ntitle = \"Root tree\"\n\n[root]\nentity = \"person:root\"\n\n[filter]\nrelationship_kinds = [\"biological-parent-child\"]\ngenerations_up = 1\ngenerations_down = 1\n",
    )
    .expect("tree view");

    let tree = compile_local_tree_with_view(&temp_dir, Some("root-tree")).expect("compile tree");
    let names = tree
        .people
        .iter()
        .filter_map(|person| tree.person_display_name(person.id))
        .collect::<Vec<_>>();

    assert_eq!(tree.people.len(), 3);
    assert!(names.contains(&"parent"));
    assert!(names.contains(&"root"));
    assert!(names.contains(&"child"));
    assert!(!names.contains(&"grandparent"));
    assert!(!names.contains(&"grandchild"));
    assert_eq!(tree.relationships.len(), 2);

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_missing_tree_view_root_reference() {
    let temp_dir = test_temp_dir("missing-tree-root");
    fs::create_dir_all(temp_dir.join("views/trees")).expect("tree views dir");
    fs::write(
        temp_dir.join("views/trees/root.toml"),
        "schema_version = 1\nid = \"tree:root\"\nkind = \"tree-view\"\ntitle = \"Root\"\n\n[root]\nentity = \"person:missing\"\n",
    )
    .expect("tree view");

    let err = compile_local_data(&temp_dir).expect_err("missing tree root should fail");
    assert!(
        err.to_string().contains("person:missing"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_missing_timeline_view_subject_reference() {
    let temp_dir = test_temp_dir("missing-timeline-subject");
    fs::create_dir_all(temp_dir.join("views/timelines")).expect("timeline views dir");
    fs::write(
        temp_dir.join("views/timelines/life.toml"),
        "schema_version = 1\nid = \"timeline:life\"\nkind = \"timeline-view\"\ntitle = \"Life\"\n\n[subject]\nentity = \"person:missing\"\n",
    )
    .expect("timeline view");

    let err = compile_local_data(&temp_dir).expect_err("missing timeline subject should fail");
    assert!(
        err.to_string().contains("person:missing"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_assertion_missing_target() {
    let temp_dir = test_temp_dir("assertion-missing-target");
    fs::create_dir_all(temp_dir.join("assertions")).expect("assertions dir");
    fs::write(
        temp_dir.join("assertions/missing-target.md"),
        "+++\nid = \"assertion:missing-target\"\nkind = \"identity\"\nvalue = \"Alex\"\n+++\n",
    )
    .expect("assertion");

    let err = compile_local_data(&temp_dir).expect_err("missing target should fail");
    assert!(
        err.to_string().contains("target"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn allows_event_support_assertion_without_value() {
    let temp_dir = test_temp_dir("assertion-support-no-value");
    fs::create_dir_all(temp_dir.join("events/observations")).expect("events dir");
    fs::create_dir_all(temp_dir.join("assertions")).expect("assertions dir");
    fs::write(
        temp_dir.join("events/observations/example.md"),
        "+++\nid = \"event:example\"\nkind = \"observation\"\ntitle = \"Example\"\nassertions = [\"assertion:example-support\"]\n+++\n\n# Example\n",
    )
    .expect("event");
    fs::write(
        temp_dir.join("assertions/example-support.md"),
        "+++\nid = \"assertion:example-support\"\nkind = \"event-support\"\ntarget = \"event:example#date\"\nconfidence = \"medium\"\n+++\n\n# Support\n",
    )
    .expect("assertion");

    let bundle =
        compile_local_data(&temp_dir).expect("targeted event date support without value is valid");

    assert!(
        bundle
            .markdown_records
            .iter()
            .any(|record| record.id == "assertion:example-support")
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_assertion_missing_source_reference() {
    let temp_dir = test_temp_dir("assertion-missing-source");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("assertions")).expect("assertions dir");
    fs::write(
        temp_dir.join("entities/people/person-alex.md"),
        "+++\nid = \"person:alex\"\nkind = \"person\"\nprimary_name = \"Alex\"\n+++\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("assertions/missing-source.md"),
        "+++\nid = \"assertion:missing-source\"\nkind = \"identity\"\ntarget = \"person:alex#name\"\nvalue = \"Alex\"\nsources = [\"source:missing\"]\n+++\n",
    )
    .expect("assertion");

    let err = compile_local_data(&temp_dir).expect_err("missing source should fail");
    assert!(
        err.to_string().contains("source:missing"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

fn test_temp_dir(label: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kleio-local-authoring-{label}-{}-{unique}",
        std::process::id()
    ))
}
