use super::super::compose_info;

#[test]
fn process_is_preferred() {
    let info = compose_info("", Some("/home/u/code"), Some("nvim"), false);
    assert_eq!(info.primary, "nvim");
    assert!(info.secondary.contains('~') || info.secondary.contains("code"));
}
