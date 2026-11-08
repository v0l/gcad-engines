use std::path::PathBuf;

pub fn fixtures() -> PathBuf {
    let repo = std::env::var_os("GCAD_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gcad"));
    let dir = repo.join("examples/game-assets");
    assert!(
        dir.is_dir(),
        "no gcad fixtures at {}: set GCAD_REPO to a gcad checkout at GCAD_REV",
        dir.display()
    );
    dir
}
