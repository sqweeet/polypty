use super::{transition, CloseTransition};

#[test]
fn final_workspace_requests_quit_without_empty_state() {
    assert_eq!(transition(1, 0, 0), CloseTransition::Quit);
    assert_eq!(transition(0, 0, 0), CloseTransition::Ignore);
}

#[test]
fn close_preserves_a_valid_active_workspace() {
    assert_eq!(transition(4, 2, 0), CloseTransition::Active(1));
    assert_eq!(transition(4, 1, 3), CloseTransition::Active(1));
    assert_eq!(transition(4, 3, 3), CloseTransition::Active(2));
    assert_eq!(transition(4, 1, 1), CloseTransition::Active(1));
}
