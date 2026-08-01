//! Input role parsing.

use vd_meeting::InputRole;

#[test]
fn known_roles() {
    assert_eq!(InputRole::parse("merged"), Some(InputRole::Merged));
    assert_eq!(InputRole::parse("participant"), Some(InputRole::Participant));
    assert_eq!(InputRole::parse("context"), Some(InputRole::Context));
    assert_eq!(InputRole::parse("tracks"), None);
}
