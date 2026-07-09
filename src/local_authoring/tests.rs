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
