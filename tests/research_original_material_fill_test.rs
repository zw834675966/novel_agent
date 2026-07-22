//! Structural gate for the original-material-fill research deliverable.
//! Ensures the shipped research note (not a reimplementation) contains required sections.

#[test]
fn original_material_fill_research_deliverable_has_required_sections() {
    let path = std::path::Path::new("docs/superpowers/specs/2026-07-22-original-material-fill-research.md");
    assert!(
        path.is_file(),
        "missing research deliverable at {}",
        path.display()
    );
    let body = std::fs::read_to_string(path).expect("read research deliverable");

    // Criterion 1–4 section anchors
    for needle in [
        "Goal framing",
        "Peer-level paper lines",
        "GitHub / open-source practice lines",
        "Recommended stack",
        "Gap vs current",
    ] {
        assert!(
            body.contains(needle),
            "research deliverable missing section marker: {needle}"
        );
    }

    // Paper / line anchors required by verification plan
    for needle in [
        "RELiC",
        "Literary Evidence Retrieval",
        "LangExtract",
        "AEVS",
        "VeriCite",
        "CAT-LLM",
    ] {
        assert!(
            body.contains(needle),
            "research deliverable missing paper line: {needle}"
        );
    }

    // GitHub / practice anchors
    for needle in [
        "google/langextract",
        "relic-retrieval",
        "Constrained-Text-Generation-Studio",
        "yyz-nbt/AEVS",
    ] {
        assert!(
            body.contains(needle),
            "research deliverable missing GitHub line: {needle}"
        );
    }

    // Gap table must call out aligned vs gap behaviors
    for needle in [
        "已对齐",
        "缺口",
        "字典序",
        "action",
        "substring",
    ] {
        assert!(
            body.contains(needle),
            "research deliverable missing gap-table signal: {needle}"
        );
    }

    // Stack must prefer programmatic inject over style transfer as primary
    assert!(
        body.contains("programmatic") || body.contains("程序") || body.contains("Assemble"),
        "stack must emphasize programmatic assemble/inject"
    );
    assert!(
        body.contains("Don't copy") || body.contains("不要"),
        "each practice line should include don't-copy guidance"
    );
}
