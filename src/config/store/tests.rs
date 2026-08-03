use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

use super::save_sidebar_shortcuts;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn shortcut_setting_preserves_comments_and_other_values() {
    let root = temporary_root();
    let path = root.join("config.toml");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        &path,
        "# keep this\n[sidebar]\nvisible = true # and this\n\n[bindings]\nnew-tab = 'ctrl+n'\n",
    )
    .unwrap();

    save_sidebar_shortcuts(&path, false).unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("# keep this"));
    assert!(saved.contains("visible = true # and this"));
    assert!(saved.contains("new-tab = 'ctrl+n'"));
    assert!(saved.contains("shortcuts = false"));

    save_sidebar_shortcuts(&path, true).unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("shortcuts = true"));
    assert_eq!(saved.matches("shortcuts =").count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn shortcut_setting_creates_a_missing_config_tree() {
    let root = temporary_root();
    let path = root.join("nested/config.toml");

    save_sidebar_shortcuts(&path, false).unwrap();

    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("[sidebar]"));
    assert!(saved.contains("shortcuts = false"));
    fs::remove_dir_all(root).unwrap();
}

fn temporary_root() -> std::path::PathBuf {
    let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "polypty-config-test-{}-{sequence}",
        std::process::id()
    ))
}
