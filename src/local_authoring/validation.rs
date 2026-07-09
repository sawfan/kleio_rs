use std::path::Path;

use super::{LocalAuthoringError, compile_local_data};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWorldValidationReport {
    pub markdown_records: usize,
    pub toml_documents: usize,
}

pub fn validate_local_world(
    world_root: impl AsRef<Path>,
) -> Result<LocalWorldValidationReport, LocalAuthoringError> {
    let bundle = compile_local_data(world_root)?;
    Ok(LocalWorldValidationReport {
        markdown_records: bundle.markdown_records.len(),
        toml_documents: bundle.toml_documents.len(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_workspace_skeleton};

    #[test]
    fn validates_starter_world() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-validate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let report = validate_local_world(&world_root).expect("validate world");

        assert!(report.markdown_records > 0);
        assert!(report.toml_documents > 0);

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
