//! Input role / purpose parsing.

use vd_meeting::{InputPurpose, InputRole};

#[test]
fn known_roles() {
    assert_eq!(InputRole::parse("room"), Some(InputRole::Room));
    assert_eq!(InputRole::parse("merged"), Some(InputRole::Room));
    assert_eq!(
        InputRole::parse("participant"),
        Some(InputRole::Participant)
    );
    assert_eq!(InputRole::parse("context"), Some(InputRole::Context));
    assert_eq!(InputRole::parse("tracks"), None);
    assert_eq!(InputRole::Room.as_str(), "room");
}

#[test]
fn known_purposes() {
    assert_eq!(
        InputPurpose::parse("transcript"),
        Some(InputPurpose::Transcript)
    );
    assert_eq!(
        InputPurpose::parse("timeline"),
        Some(InputPurpose::Timeline)
    );
    assert_eq!(InputPurpose::parse("mix"), None);
}
