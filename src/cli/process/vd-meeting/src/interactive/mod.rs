//! Interactive wizard for meeting input selection (ADR 0017 Decision D).
//!
//! **STUB — implementation tracked separately.** Signature and interface here; body is
//! `todo!()` pending the full P1-D wizard implementation (classify + menu + confirm).

use std::path::PathBuf;

use crate::model::{InputRole, InputSource};

/// Run interactive meeting input wizard.
///
/// Called when `vd-meeting run --interactive` or auto-detected TTY. Proposes meeting inputs
/// from files in working_dir (using [`vd_classify`] heuristics), shows a numbered menu
/// (using [`vd_pipeline::interactive::run`]) for accept/edit/drop, detects context folder
/// (using [`crate::paths::resolve_context_dir`]), then returns confirmed inputs + optional
/// context path.
///
/// **STUB:** No implementation yet — returns error or empty inputs. Full implementation pending.
pub fn show_wizard(
    working_dir: Option<&std::path::Path>,
) -> Result<(Vec<InputSource>, Option<PathBuf>), String> {
    let _ = working_dir;
    todo!("ADR 0017 P1-D: implement interactive wizard (classify + menu + confirm)")
}
