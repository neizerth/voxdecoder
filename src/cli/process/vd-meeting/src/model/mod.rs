//! Meeting domain model (no Job / Executor knobs).

mod document;
mod input;
mod meeting;
mod options;

pub use document::{load_meeting_file, MeetingDocument};
pub use input::{InputRole, InputSource};
pub use meeting::{
    AlignmentMode, AlignmentOptions, CountBounds, DiarizationEnabled, DiarizationPolicy, Gender,
    GroupConstraints, KnownParticipant, MeetingModel, MeetingOutput, MeetingRequest,
    ParticipantConstraints, Participants,
};
pub use options::{BuildOptions, ExecutorOptions, TranscribeDefaults};
