use super::super::shorten_path;

#[test]
fn home_is_shortened() {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let path = format!("{}/projects/mux", home.to_string_lossy());
    assert!(shorten_path(&path).starts_with("~/"));
}
