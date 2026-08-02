//! TimeMap: processed timeline → original timeline.
//!
//! Produced by `vd-preprocess` when timing filters (`speed`, `trim-silence`, …) rewrite
//! media time. The Job Executor remaps timeline artifacts before registering them as
//! canonical (see `docs/adr/0001-platform-refactoring-plan.md` §5–7).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimeInterval {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeMapSegment {
    pub processed: TimeInterval,
    pub original: TimeInterval,
}

/// Piecewise map from the **processed** clock to the **original** media clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeMap {
    pub version: u32,
    pub segments: Vec<TimeMapSegment>,
}

impl TimeMap {
    pub const CURRENT_VERSION: u32 = 1;

    /// Uniform scale: processed `[0, processed_end]` ↔ original `[0, original_end]`.
    /// Used for constant `speed` (and as a fallback when only durations are known).
    pub fn uniform(processed_end: f64, original_end: f64) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            segments: vec![TimeMapSegment {
                processed: TimeInterval {
                    start: 0.0,
                    end: processed_end.max(0.0),
                },
                original: TimeInterval {
                    start: 0.0,
                    end: original_end.max(0.0),
                },
            }],
        }
    }

    /// Identity map over `[0, duration]` (no time rewrite).
    pub fn identity(duration: f64) -> Self {
        Self::uniform(duration, duration)
    }

    /// Map a point on the processed timeline to the original timeline.
    pub fn to_original(&self, t: f64) -> f64 {
        if self.segments.is_empty() {
            return t;
        }
        for seg in &self.segments {
            let ps = seg.processed.start;
            let pe = seg.processed.end;
            if t + f64::EPSILON < ps {
                continue;
            }
            if t <= pe + f64::EPSILON || (pe - ps).abs() < f64::EPSILON {
                let span_p = pe - ps;
                let span_o = seg.original.end - seg.original.start;
                if span_p.abs() < 1e-12 {
                    return seg.original.start;
                }
                let ratio = ((t - ps) / span_p).clamp(0.0, 1.0);
                return seg.original.start + ratio * span_o;
            }
        }
        // Past last segment: extrapolate with last segment's scale.
        let last = self.segments.last().unwrap();
        let span_p = last.processed.end - last.processed.start;
        let span_o = last.original.end - last.original.start;
        if span_p.abs() < 1e-12 {
            return last.original.end;
        }
        let ratio = (t - last.processed.start) / span_p;
        last.original.start + ratio * span_o
    }

    pub fn remap_interval(&self, start: f64, end: f64) -> (f64, f64) {
        (self.to_original(start), self.to_original(end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_speed_2x() {
        let map = TimeMap::uniform(10.0, 20.0);
        assert!((map.to_original(0.0) - 0.0).abs() < 1e-9);
        assert!((map.to_original(5.0) - 10.0).abs() < 1e-9);
        assert!((map.to_original(10.0) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn piecewise_gap() {
        let map = TimeMap {
            version: 1,
            segments: vec![
                TimeMapSegment {
                    processed: TimeInterval {
                        start: 0.0,
                        end: 20.0,
                    },
                    original: TimeInterval {
                        start: 0.0,
                        end: 25.0,
                    },
                },
                TimeMapSegment {
                    processed: TimeInterval {
                        start: 20.0,
                        end: 40.0,
                    },
                    original: TimeInterval {
                        start: 30.0,
                        end: 50.0,
                    },
                },
            ],
        };
        assert!((map.to_original(10.0) - 12.5).abs() < 1e-9);
        assert!((map.to_original(30.0) - 40.0).abs() < 1e-9);
    }
}
