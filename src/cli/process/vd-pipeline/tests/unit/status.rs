//! Status helpers.

use vd_pipeline::status::overall_percent;

#[test]
fn overall_percent_math() {
    assert_eq!(overall_percent(0, 4), 0);
    assert_eq!(overall_percent(1, 4), 25);
    assert_eq!(overall_percent(4, 4), 100);
    assert_eq!(overall_percent(0, 0), 100);
}
