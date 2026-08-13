use super::*;
use crate::{NameOrder, RelationshipKind};

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
fn infers_legal_name_from_person_filename_and_uses_preferred_name() {
    let temp_dir = test_temp_dir("person-filename-name");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::write(
        temp_dir.join("entities/people/john-quincy-smith.md"),
        "+++\nid = \"person:john-quincy-smith\"\nkind = \"person\"\npreferred_name = \"Quincy\"\n+++\n\n# Quincy\n",
    )
    .expect("person");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    assert_eq!(
        tree.person_display_name(tree.people[0].id),
        Some("Quincy Smith")
    );
    assert_eq!(tree.people[0].names.len(), 2);
    assert_eq!(tree.people[0].names[0].usage.as_deref(), Some("preferred"));
    assert_eq!(tree.people[0].names[0].display, "Quincy Smith");
    assert_eq!(tree.people[0].names[0].given.as_deref(), Some("Quincy"));
    assert_eq!(tree.people[0].names[0].surname.as_deref(), Some("Smith"));
    assert_eq!(
        tree.people[0].names[0]
            .order
            .as_ref()
            .map(NameOrder::as_value),
        Some(NameOrder::GIVEN_MIDDLE_FAMILY)
    );
    assert_eq!(tree.people[0].names[1].usage.as_deref(), Some("legal"));
    assert_eq!(
        tree.people[0].names[1].full.as_deref(),
        Some("John Quincy Smith")
    );
    assert_eq!(tree.people[0].names[1].given.as_deref(), Some("John"));
    assert_eq!(tree.people[0].names[1].middle.as_deref(), Some("Quincy"));
    assert_eq!(tree.people[0].names[1].surname.as_deref(), Some("Smith"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn supports_family_name_first_preferred_name_order() {
    let temp_dir = test_temp_dir("family-name-first");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::write(
        temp_dir.join("entities/people/wang-xiaoming.md"),
        "+++\nid = \"person:wang-xiaoming\"\nkind = \"person\"\n\n[names.preferred]\ngiven = \"Xiaoming\"\nfamily = \"Wang\"\norder = \"family-given\"\n\n[names.legal]\ngiven = \"Xiaoming\"\nfamily = \"Wang\"\norder = \"family-given\"\n+++\n\n# Wang Xiaoming\n",
    )
    .expect("person");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    assert_eq!(
        tree.person_display_name(tree.people[0].id),
        Some("Wang Xiaoming")
    );
    assert_eq!(tree.people[0].names[0].given.as_deref(), Some("Xiaoming"));
    assert_eq!(tree.people[0].names[0].surname.as_deref(), Some("Wang"));
    assert_eq!(
        tree.people[0].names[0]
            .order
            .as_ref()
            .map(NameOrder::as_value),
        Some(NameOrder::FAMILY_GIVEN)
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn married_name_default_does_not_rewrite_family_name_first_display() {
    let temp_dir = test_temp_dir("married-name-family-first");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
        temp_dir.join("entities/people/li-wei.md"),
        "+++\nid = \"person:li-wei\"\nkind = \"person\"\nsex = \"male\"\n\n[names.preferred]\ngiven = \"Wei\"\nfamily = \"Li\"\norder = \"family-given\"\n+++\n\n# Li Wei\n",
    )
    .expect("husband");
    fs::write(
        temp_dir.join("entities/people/wang-xiaoming.md"),
        "+++\nid = \"person:wang-xiaoming\"\nkind = \"person\"\nsex = \"female\"\n\n[names.preferred]\ngiven = \"Xiaoming\"\nfamily = \"Wang\"\norder = \"family-given\"\n+++\n\n# Wang Xiaoming\n",
    )
    .expect("wife");
    fs::write(
        temp_dir.join("relationships/li-wang-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:li-wei\"\ntarget = \"person:wang-xiaoming\"\n",
    )
    .expect("spouse relationship");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    let wife = tree
        .people
        .iter()
        .find(|person| {
            person
                .source_record
                .as_ref()
                .is_some_and(|source| source.0 == "local:person:wang-xiaoming")
        })
        .expect("wife");

    assert_eq!(tree.person_display_name(wife.id), Some("Wang Xiaoming"));
    assert_eq!(wife.names[0].surname.as_deref(), Some("Wang"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn married_woman_defaults_to_husbands_surname_unless_preferred_surname_is_explicit() {
    let temp_dir = test_temp_dir("married-name-default");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
        temp_dir.join("entities/people/john-smith.md"),
        "+++\nid = \"person:john-smith\"\nkind = \"person\"\nsex = \"male\"\n+++\n\n# John\n",
    )
    .expect("husband");
    fs::write(
        temp_dir.join("entities/people/mary-ann-jones.md"),
        "+++\nid = \"person:mary-ann-jones\"\nkind = \"person\"\npreferred_name = \"Ann\"\nsex = \"female\"\n+++\n\n# Ann\n",
    )
    .expect("wife");
    fs::write(
        temp_dir.join("relationships/john-mary-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:john-smith\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("spouse relationship");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    let wife = tree
        .people
        .iter()
        .find(|person| {
            person
                .source_record
                .as_ref()
                .is_some_and(|source| source.0 == "local:person:mary-ann-jones")
        })
        .expect("wife");

    assert_eq!(tree.person_display_name(wife.id), Some("Ann Smith"));
    assert_eq!(wife.names[0].given.as_deref(), Some("Ann"));
    assert_eq!(wife.names[0].surname.as_deref(), Some("Smith"));
    assert_eq!(wife.names[1].surname.as_deref(), Some("Jones"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn explicit_preferred_surname_overrides_married_name_default() {
    let temp_dir = test_temp_dir("married-name-override");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
        temp_dir.join("entities/people/john-smith.md"),
        "+++\nid = \"person:john-smith\"\nkind = \"person\"\nsex = \"male\"\n+++\n\n# John\n",
    )
    .expect("husband");
    fs::write(
        temp_dir.join("entities/people/mary-ann-jones.md"),
        "+++\nid = \"person:mary-ann-jones\"\nkind = \"person\"\nsex = \"female\"\n\n[names.preferred]\nfull = \"Ann Jones\"\ngiven = \"Ann\"\nfamily = \"Jones\"\n+++\n\n# Ann\n",
    )
    .expect("wife");
    fs::write(
        temp_dir.join("relationships/john-mary-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:john-smith\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("spouse relationship");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    let wife = tree
        .people
        .iter()
        .find(|person| {
            person
                .source_record
                .as_ref()
                .is_some_and(|source| source.0 == "local:person:mary-ann-jones")
        })
        .expect("wife");

    assert_eq!(tree.person_display_name(wife.id), Some("Ann Jones"));
    assert_eq!(wife.names[0].surname.as_deref(), Some("Jones"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn parent_roles_can_drive_married_name_default_without_sex_fields() {
    let temp_dir = test_temp_dir("married-name-parent-roles");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
        temp_dir.join("entities/people/john-smith.md"),
        "+++\nid = \"person:john-smith\"\nkind = \"person\"\n+++\n\n# John\n",
    )
    .expect("father");
    fs::write(
        temp_dir.join("entities/people/mary-ann-jones.md"),
        "+++\nid = \"person:mary-ann-jones\"\nkind = \"person\"\npreferred_name = \"Ann\"\n+++\n\n# Ann\n",
    )
    .expect("mother");
    fs::write(
        temp_dir.join("entities/people/riley-smith.md"),
        "+++\nid = \"person:riley-smith\"\nkind = \"person\"\n+++\n\n# Riley\n",
    )
    .expect("child");
    fs::write(
        temp_dir.join("relationships/john-mary-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:john-smith\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("spouse relationship");
    fs::write(
        temp_dir.join("relationships/john-riley-parent.toml"),
        "relationship = \"biological-parent-child\"\nparent_role = \"father\"\nsource = \"person:john-smith\"\ntarget = \"person:riley-smith\"\n",
    )
    .expect("father relationship");
    fs::write(
        temp_dir.join("relationships/mary-riley-parent.toml"),
        "relationship = \"biological-parent-child\"\nparent_role = \"mother\"\nsource = \"person:mary-ann-jones\"\ntarget = \"person:riley-smith\"\n",
    )
    .expect("mother relationship");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    let mother = tree
        .people
        .iter()
        .find(|person| {
            person
                .source_record
                .as_ref()
                .is_some_and(|source| source.0 == "local:person:mary-ann-jones")
        })
        .expect("mother");

    assert_eq!(tree.person_display_name(mother.id), Some("Ann Smith"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn former_spouse_does_not_drive_current_married_name_default() {
    let temp_dir = test_temp_dir("married-name-former-spouse");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
        temp_dir.join("entities/people/alex-brown.md"),
        "+++\nid = \"person:alex-brown\"\nkind = \"person\"\nsex = \"male\"\n+++\n\n# Alex\n",
    )
    .expect("former husband");
    fs::write(
        temp_dir.join("entities/people/john-smith.md"),
        "+++\nid = \"person:john-smith\"\nkind = \"person\"\nsex = \"male\"\n+++\n\n# John\n",
    )
    .expect("current husband");
    fs::write(
        temp_dir.join("entities/people/mary-ann-jones.md"),
        "+++\nid = \"person:mary-ann-jones\"\nkind = \"person\"\npreferred_name = \"Ann\"\nsex = \"female\"\n+++\n\n# Ann\n",
    )
    .expect("wife");
    fs::write(
        temp_dir.join("relationships/alex-mary-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:alex-brown\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("old spouse relationship");
    fs::write(
        temp_dir.join("relationships/alex-mary-former-spouse.toml"),
        "relationship = \"former-spouse\"\nsource = \"person:alex-brown\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("former spouse relationship");
    fs::write(
        temp_dir.join("relationships/john-mary-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:john-smith\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("current spouse relationship");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    let wife = tree
        .people
        .iter()
        .find(|person| {
            person
                .source_record
                .as_ref()
                .is_some_and(|source| source.0 == "local:person:mary-ann-jones")
        })
        .expect("wife");

    assert_eq!(tree.person_display_name(wife.id), Some("Ann Smith"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn ambiguous_multiple_current_spouses_do_not_drive_married_name_default() {
    let temp_dir = test_temp_dir("married-name-ambiguous-spouses");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::write(
        temp_dir.join("entities/people/alex-brown.md"),
        "+++\nid = \"person:alex-brown\"\nkind = \"person\"\nsex = \"male\"\n+++\n\n# Alex\n",
    )
    .expect("first spouse");
    fs::write(
        temp_dir.join("entities/people/john-smith.md"),
        "+++\nid = \"person:john-smith\"\nkind = \"person\"\nsex = \"male\"\n+++\n\n# John\n",
    )
    .expect("second spouse");
    fs::write(
        temp_dir.join("entities/people/mary-ann-jones.md"),
        "+++\nid = \"person:mary-ann-jones\"\nkind = \"person\"\npreferred_name = \"Ann\"\nsex = \"female\"\n+++\n\n# Ann\n",
    )
    .expect("wife");
    fs::write(
        temp_dir.join("relationships/alex-mary-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:alex-brown\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("first spouse relationship");
    fs::write(
        temp_dir.join("relationships/john-mary-spouse.toml"),
        "relationship = \"spouse\"\nsource = \"person:john-smith\"\ntarget = \"person:mary-ann-jones\"\n",
    )
    .expect("second spouse relationship");

    let tree = compile_local_tree(&temp_dir).expect("compile tree");
    let wife = tree
        .people
        .iter()
        .find(|person| {
            person
                .source_record
                .as_ref()
                .is_some_and(|source| source.0 == "local:person:mary-ann-jones")
        })
        .expect("wife");

    assert_eq!(tree.person_display_name(wife.id), Some("Ann Jones"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn compiles_local_event_collections_into_timeline_projection() {
    let temp_dir = test_temp_dir("timeline-collections");
    fs::create_dir_all(temp_dir.join("events/observations")).expect("events dir");
    fs::create_dir_all(temp_dir.join("collections")).expect("collections dir");
    fs::write(
        temp_dir.join("events/observations/first.md"),
        "+++\nid = \"event:first\"\nkind = \"event\"\ntype = \"observation\"\ntitle = \"First event\"\ndate = 2026-01-01\n+++\n\n# First\n",
    )
    .expect("first event");
    fs::write(
        temp_dir.join("events/observations/second.md"),
        "+++\nid = \"event:second\"\nkind = \"event\"\ntype = \"observation\"\ntitle = \"Second event\"\ndate = 2026-01-02\n+++\n\n# Second\n",
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
fn compiles_inline_event_location_into_timeline_projection() {
    let temp_dir = test_temp_dir("inline-location");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::create_dir_all(temp_dir.join("sources")).expect("sources dir");
    fs::write(
        temp_dir.join("entities/people/alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("sources/personal-knowledge.md"),
        "+++\nid = \"source:personal-knowledge\"\nkind = \"note\"\ntitle = \"Personal knowledge\"\n+++\n\n# Source\n",
    )
    .expect("source");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
        "+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\ntime = \"1900-01-01 07:18\"\nlocation = \"Example Town\"\nparticipants = [\"person:alex-example\"]\nassertions = []\n+++\n\n# Birth\n",
    )
    .expect("birth event");

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(timeline.events.len(), 1);
    assert_eq!(
        timeline.events[0].title.as_deref(),
        Some("Alex Example was born")
    );
    assert_eq!(
        timeline.events[0].places,
        vec![serde_json::json!({
            "entity": "place:inline:event-birth-alex-example-location",
            "role": "birthplace",
            "label": "Example Town",
            "generated": true,
        })]
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn filename_hints_fill_birth_datetime_participant_and_location_coordinates() {
    let temp_dir = test_temp_dir("filename-hints-birth");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("events/births/birth--person=alex-example--local=1900-01-01T12-04--lat=40.7128--lng=-74.0060.md"),
        "+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\n\n[location]\nlabel = \"Example Memorial Hospital\"\n+++\n\n# Birth\n",
    )
    .expect("birth event");

    let bundle = compile_local_data(&temp_dir).expect("compile local data");
    let birth_record = bundle
        .markdown_records
        .iter()
        .find(|record| record.id == "event:birth-alex-example")
        .expect("birth record");
    assert_eq!(
        birth_record
            .attributes
            .get("subject")
            .and_then(serde_json::Value::as_str),
        Some("alex-example")
    );
    assert!(
        !birth_record.attributes.contains_key("participants"),
        "filename person hint should infer subject without writing participants"
    );

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(timeline.events.len(), 1);
    assert_eq!(timeline.events[0].event_type, "birth");
    assert_eq!(timeline.events[0].time.as_deref(), Some("1900-01-01 12:04"));
    assert_eq!(
        timeline.events[0].participants,
        vec![serde_json::json!({ "entity": "person:alex-example", "role": "subject" })]
    );
    assert_eq!(
        timeline.events[0].places,
        vec![serde_json::json!({
            "entity": "place:inline:event-birth-alex-example-location",
            "role": "birthplace",
            "label": "Example Memorial Hospital",
            "latitude": 40.7128,
            "longitude": -74.006,
            "generated": true,
        })]
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn compiles_typed_event_with_participant_shorthand() {
    let temp_dir = test_temp_dir("typed-event-participant-shorthand");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
        "+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\ntime = \"1900-01-01\"\nparticipants = [\"alex-example\"]\nassertions = []\n+++\n\n# Birth\n",
    )
    .expect("birth event");

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(timeline.events.len(), 1);
    assert_eq!(timeline.events[0].event_type, "birth");
    assert_eq!(
        timeline.events[0].participants,
        vec![serde_json::json!({ "entity": "person:alex-example", "role": "subject" })]
    );
    assert_eq!(
        timeline.events[0].title.as_deref(),
        Some("Alex Example was born")
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn compiles_place_shorthand_with_default_event_role() {
    let temp_dir = test_temp_dir("place-shorthand-default-role");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("entities/places")).expect("places dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("entities/places/example-town.md"),
        "+++\nid = \"place:example-town\"\nkind = \"place\"\npreferred_name = \"Example Town\"\n+++\n\n# Example Town\n",
    )
    .expect("place");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
        "+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\nparticipants = [\"alex-example\"]\nplaces = [\"example-town\"]\nassertions = []\n+++\n\n# Birth\n",
    )
    .expect("birth event");

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(
        timeline.events[0].places,
        vec![serde_json::json!({ "entity": "place:example-town", "role": "birthplace" })]
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn explicit_event_title_overrides_default_label() {
    let temp_dir = test_temp_dir("explicit-event-title");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
"+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\ntitle = \"Custom birth label\"\nparticipants = [\"person:alex-example\"]\nassertions = []\n+++\n\n# Birth\n"
    )
    .expect("birth event");

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(
        timeline.events[0].title.as_deref(),
        Some("Custom birth label")
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn uses_subject_as_default_event_participant() {
    let temp_dir = test_temp_dir("event-subject");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/residences")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("events/residences/alex-residence.md"),
        "+++\nid = \"event:alex-residence\"\nkind = \"event\"\ntype = \"residence\"\nsubject = \"alex-example\"\ntime = \"1900-01-01\"\nassertions = []\n+++\n\n# Residence\n",
    )
    .expect("residence event");

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(
        timeline.events[0].participants,
        vec![serde_json::json!({ "entity": "person:alex-example", "role": "subject" })]
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn missing_participant_error_suggests_local_lookup_and_subject() {
    let temp_dir = test_temp_dir("missing-participant-help");
    fs::create_dir_all(temp_dir.join("events/observations")).expect("events dir");
    fs::write(
        temp_dir.join("events/observations/missing.md"),
        "+++\nid = \"event:missing\"\nkind = \"event\"\ntype = \"observation\"\nparticipants = [\"missing-person\"]\nassertions = []\n+++\n\n# Missing\n",
    )
    .expect("event");

    let err = compile_local_data(&temp_dir).expect_err("missing participant should fail");
    let message = err.to_string();

    assert!(
        message.contains("list-people"),
        "unexpected error: {message}"
    );
    assert!(message.contains("subject"), "unexpected error: {message}");

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn infers_birth_participant_from_birth_event_id_when_omitted() {
    let temp_dir = test_temp_dir("birth-participant-inferred");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
        "+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\ntime = \"1900-01-01\"\nassertions = []\n+++\n\n# Birth\n",
    )
    .expect("birth event");

    let timeline = compile_local_timeline(&temp_dir, None).expect("compile timeline");

    assert_eq!(
        timeline.events[0].participants,
        vec![serde_json::json!({ "entity": "person:alex-example", "role": "subject" })]
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_empty_inline_event_location() {
    let temp_dir = test_temp_dir("empty-inline-location");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
"+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\nlocation = \"\"\nparticipants = []\nassertions = []\n+++\n\n# Birth\n"
    )
    .expect("birth event");

    let err = compile_local_data(&temp_dir).expect_err("empty location should fail");
    assert!(
        err.to_string().contains("location"),
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
fn rejects_event_type_with_whitespace() {
    let temp_dir = test_temp_dir("bad-event-type");
    fs::create_dir_all(temp_dir.join("events/other")).expect("events dir");
    fs::write(
        temp_dir.join("events/other/bad.md"),
        "+++\nid = \"event:bad\"\nkind = \"event\"\ntype = \"bad type\"\nparticipants = []\nassertions = []\n+++\n\n# Bad\n",
    )
    .expect("event");

    let err = compile_local_data(&temp_dir).expect_err("bad event type should fail");
    assert!(
        err.to_string().contains("bad type"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn rejects_missing_event_assertion_reference() {
    let temp_dir = test_temp_dir("missing-assertion");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
    fs::write(
        temp_dir.join("entities/people/person-alex-example.md"),
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Note\n",
    )
    .expect("person");
    fs::write(
        temp_dir.join("events/births/birth-alex-example.md"),
        "+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\nparticipants = [\"alex-example\"]\nassertions = [\"assertion:missing\"]\nsources = [\"personal-knowledge\"]\n+++\n\n# Note\n",
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
        "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Note\n",
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
                "+++\nid = \"person:{slug}\"\nkind = \"person\"\npreferred_name = \"{slug}\"\n+++\n\n# Note\n"
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
        "schema_version = 1\nid = \"tree:root-tree\"\nkind = \"tree-view\"\ntitle = \"Root tree\"\n\n[root]\nentity = \"person:root\"\n\n[projection]\nrelationship_kinds = [\"biological-parent-child\"]\ngenerations_up = 1\ngenerations_down = 1\ninclude_partners = false\ninclude_siblings = false\ninclude_unconnected = false\n",
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
fn tree_view_projection_can_include_siblings_without_unconnected_people() {
    let temp_dir = test_temp_dir("tree-projection-siblings");
    fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
    fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
    fs::create_dir_all(temp_dir.join("views/trees")).expect("tree views dir");
    for slug in ["parent", "root", "sibling", "unrelated"] {
        fs::write(
            temp_dir.join(format!("entities/people/{slug}.md")),
            format!(
                "+++\nid = \"person:{slug}\"\nkind = \"person\"\npreferred_name = \"{slug}\"\n+++\n\n# Note\n"
            ),
        )
        .expect("person");
    }
    for (slug, source, target) in [
        ("parent-root", "person:parent", "person:root"),
        ("parent-sibling", "person:parent", "person:sibling"),
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
        "schema_version = 1\nid = \"tree:root-tree\"\nkind = \"tree-view\"\ntitle = \"Root tree\"\n\n[root]\nentity = \"person:root\"\n\n[projection]\nrelationship_kinds = [\"biological-parent-child\"]\ngenerations_up = 1\ngenerations_down = 0\ninclude_partners = false\ninclude_siblings = true\ninclude_unconnected = false\n",
    )
    .expect("tree view");

    let tree = compile_local_tree_with_view(&temp_dir, Some("root-tree")).expect("compile tree");
    let names = tree
        .people
        .iter()
        .filter_map(|person| tree.person_display_name(person.id))
        .collect::<Vec<_>>();

    assert!(names.contains(&"root"));
    assert!(names.contains(&"parent"));
    assert!(names.contains(&"sibling"));
    assert!(!names.contains(&"unrelated"));

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
"+++\nid = \"event:example\"\nkind = \"event\"\ntype = \"observation\"\ntitle = \"Example\"\nassertions = [\"assertion:example-support\"]\n+++\n\n# Example\n"
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
        "+++\nid = \"person:alex\"\nkind = \"person\"\npreferred_name = \"Alex\"\n+++\n",
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
