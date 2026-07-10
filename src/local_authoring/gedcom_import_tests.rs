use std::fs;
use std::path::Path;

use crate::local_authoring::{
    LocalGedcomIngestOptions, LocalSkeletonOptions, PrimaryGedcomImportOptions,
    create_workspace_skeleton, ingest_primary_gedcom_to_world, set_primary_gedcom_import,
};

#[test]
fn links_primary_gedcom_in_world_config() {
    let temp_dir = std::env::temp_dir().join(format!(
        "kleio-gedcom-link-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
    let world_root = temp_dir.join("worlds/default");
    fs::write(
        world_root.join("imports/gedcom/family.ged"),
        "0 HEAD\n0 TRLR\n",
    )
    .expect("gedcom");

    set_primary_gedcom_import(
        &world_root,
        &PrimaryGedcomImportOptions {
            path: "imports/gedcom/family.ged".to_string(),
            strategy: "link".to_string(),
            allow_missing: false,
        },
    )
    .expect("set primary gedcom");

    let updated = fs::read_to_string(world_root.join("world.toml")).expect("config");
    assert!(updated.contains("path = \"imports/gedcom/family.ged\""));
    assert!(updated.contains("strategy = \"link\""));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn ingests_minimal_gedcom_into_world_records() {
    let temp_dir = std::env::temp_dir().join(format!(
        "kleio-gedcom-ingest-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
    let world_root = temp_dir.join("worlds/default");
    fs::write(
        world_root.join("imports/gedcom/family.ged"),
            "0 HEAD\n1 SOUR kleio-test\n0 @I1@ INDI\n1 NAME Alex /Example/\n1 SEX M\n1 NOTE Person note from GEDCOM\n2 CONT continued person note\n2 CONC appended\n1 BIRT\n2 DATE 1 JAN 1900\n2 PLAC Example Town\n2 NOTE Birth note from GEDCOM\n3 CONT continued birth note\n0 @I2@ INDI\n1 NAME Morgan /Example/\n1 SEX F\n0 @I3@ INDI\n1 NAME Riley /Example/\n0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 CHIL @I3@\n0 TRLR\n",
    )
    .expect("gedcom");

    let report = ingest_primary_gedcom_to_world(
        &world_root,
        &LocalGedcomIngestOptions {
            path: "imports/gedcom/family.ged".to_string(),
            force: true,
        },
    )
    .expect("ingest GEDCOM");

    assert_eq!(report.people, 3);
    assert_eq!(report.places, 1);
    assert_eq!(report.events, 1);
    assert_eq!(report.assertions, 2);
    assert_eq!(report.relationships, 3);
    assert_eq!(report.parser, expected_parser());
    assert_eq!(report.warnings.len(), 0);
    assert_eq!(
        report.import_report_path.as_deref(),
        Some(Path::new("imports/gedcom/family-report.toml"))
    );
    assert!(world_root.join("entities/people/i1.md").exists());
    let person =
        fs::read_to_string(world_root.join("entities/people/i1.md")).expect("person record");
    assert!(person.contains("Person note from GEDCOM\ncontinued person noteappended"));
    assert!(world_root.join("entities/people/i2.md").exists());
    assert!(world_root.join("entities/people/i3.md").exists());
    assert!(
        world_root
            .join("events/births/1-jan-1900-birth-i1-0.md")
            .exists()
    );
    let event = fs::read_to_string(world_root.join("events/births/1-jan-1900-birth-i1-0.md"))
        .expect("birth event");
    assert!(event.contains("assertion:gedcom-1-jan-1900-birth-i1-date"));
    assert!(event.contains("assertion:gedcom-1-jan-1900-birth-i1-place"));
    assert!(event.contains("Birth note from GEDCOM\ncontinued birth note"));
    assert!(
        world_root
            .join("assertions/gedcom-1-jan-1900-birth-i1-date.md")
            .exists()
    );
    assert!(
        world_root
            .join("assertions/gedcom-1-jan-1900-birth-i1-place.md")
            .exists()
    );
    assert!(world_root.join("relationships/spouse-i1-i2.toml").exists());
    assert!(
        world_root
            .join("relationships/biological-parent-child-i1-i3.toml")
            .exists()
    );
    assert!(
        world_root
            .join("relationships/biological-parent-child-i2-i3.toml")
            .exists()
    );
    let import_report = fs::read_to_string(world_root.join("imports/gedcom/family-report.toml"))
        .expect("import report");
    assert!(import_report.contains(&format!("parser = \"{}\"", expected_parser())));
    assert!(import_report.contains("warnings = []"));
    assert!(import_report.contains("kind = \"gedcom-import-report\""));
    assert!(import_report.contains("people = 3"));
    assert!(import_report.contains("assertions = 2"));
    assert!(import_report.contains("relationships = 3"));
    assert!(import_report.contains("skipped_existing = 0"));

    let repeat_report = ingest_primary_gedcom_to_world(
        &world_root,
        &LocalGedcomIngestOptions {
            path: "imports/gedcom/family.ged".to_string(),
            force: false,
        },
    )
    .expect("repeat ingest GEDCOM");
    assert_eq!(repeat_report.people, 0);
    assert_eq!(repeat_report.places, 0);
    assert_eq!(repeat_report.events, 0);
    assert_eq!(repeat_report.assertions, 0);
    assert_eq!(repeat_report.relationships, 0);
    assert_eq!(repeat_report.sources, 0);
    assert_eq!(repeat_report.parser, expected_parser());
    assert_eq!(repeat_report.skipped_existing, 11);

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

#[test]
fn gedcom_ingest_reports_missing_family_references_as_warnings() {
    let temp_dir = std::env::temp_dir().join(format!(
        "kleio-gedcom-ingest-warnings-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
    let world_root = temp_dir.join("worlds/default");
    fs::write(
        world_root.join("imports/gedcom/family.ged"),
        "0 HEAD\n1 SOUR kleio-test\n0 @I1@ INDI\n1 NAME Alex /Example/\n0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 CHIL @I3@\n0 TRLR\n",
    )
    .expect("gedcom");

    let report = ingest_primary_gedcom_to_world(
        &world_root,
        &LocalGedcomIngestOptions {
            path: "imports/gedcom/family.ged".to_string(),
            force: true,
        },
    )
    .expect("ingest GEDCOM");

    assert_eq!(report.relationships, 0);
    assert_eq!(report.warnings.len(), 2);
    assert!(report.warnings.iter().any(|warning| warning.contains("i2")));
    assert!(report.warnings.iter().any(|warning| warning.contains("i3")));
    let import_report = fs::read_to_string(world_root.join("imports/gedcom/family-report.toml"))
        .expect("import report");
    assert!(import_report.contains("warnings = ["));
    assert!(import_report.contains("i2"));
    assert!(import_report.contains("i3"));

    fs::remove_dir_all(temp_dir).expect("remove temp dir");
}

fn expected_parser() -> &'static str {
    #[cfg(feature = "ged-io")]
    {
        "ged-io"
    }
    #[cfg(not(feature = "ged-io"))]
    {
        "minimal"
    }
}
