//! Template render.

use std::collections::BTreeMap;

use vd_postprocess::postprocess::recipe::render_template;

#[test]
fn replaces_vars() {
    let mut v = BTreeMap::new();
    v.insert("audience".into(), "Execs".into());
    let out = render_template("For {{ audience }}", &v);
    assert_eq!(out, "For Execs");
}

#[test]
fn strips_empty_if() {
    let mut v = BTreeMap::new();
    v.insert("meeting".into(), String::new());
    let out = render_template("A {% if meeting %}M: {{ meeting }}{% endif %}B", &v);
    assert_eq!(out, "A B");
}
