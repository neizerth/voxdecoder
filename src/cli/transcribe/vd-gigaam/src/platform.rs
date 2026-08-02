//! OS / feature gates for CLI surface and runtime.
//!
//! Backend-specific knobs (Metal, CUDA, FlashAttention) must not appear in help
//! or be accepted as flags on platforms where they do not exist.

/// FlashAttention CLI flag exists only off macOS.
pub const FLASH_SUPPORTED: bool = cfg!(not(target_os = "macos"));

/// Whether Metal is a first-class device on this build.
pub const METAL_SUPPORTED: bool = cfg!(all(target_os = "macos", feature = "metal"));

/// Whether CUDA is a first-class device on this build.
pub const CUDA_SUPPORTED: bool = cfg!(not(target_os = "macos"));
