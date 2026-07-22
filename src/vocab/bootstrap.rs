//! Runtime vocabulary bootstrap: load base YAML then merge optional distilled dir.

use super::loader::{SENSES, Vocab};

/// Summary of what was loaded when building the runtime vocab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabLoadReport {
    pub base_entries: usize,
    pub distilled_files: usize,
    pub total_entries: usize,
}

/// Count total vocabulary entries across all sense categories.
pub fn count_entries(v: &Vocab) -> usize {
    SENSES
        .iter()
        .map(|s| v.entries(s).map(|m| m.len()).unwrap_or(0))
        .sum()
}

/// Load base vocab from `base`, optionally merge all `*.yaml` under `distilled_dir`.
///
/// When `distilled_dir` is `None` or not a directory, only the base file is loaded.
pub fn load_runtime_vocab(
    base: &std::path::Path,
    distilled_dir: Option<&std::path::Path>,
) -> Result<(Vocab, VocabLoadReport), crate::models::StoryError> {
    let mut vocab = Vocab::load_from_path(base)?;
    let base_entries = count_entries(&vocab);
    let mut distilled_files = 0;
    if let Some(dir) = distilled_dir
        && dir.is_dir()
    {
        distilled_files = vocab.load_dir_merged(dir)?;
    }
    let total_entries = count_entries(&vocab);
    Ok((
        vocab,
        VocabLoadReport {
            base_entries,
            distilled_files,
            total_entries,
        },
    ))
}
