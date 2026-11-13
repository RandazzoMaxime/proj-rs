//! The PROJ version this crate ports, and PROJ's version macros as functions
//! (`proj.h`: `PROJ_COMPUTE_VERSION`, `PROJ_VERSION_NUMBER`,
//! `PROJ_AT_LEAST_VERSION`).

/// Major version of PROJ this crate ports.
pub const MAJOR: u32 = 9;
/// Minor version of PROJ this crate ports.
pub const MINOR: u32 = 9;
/// Patch version of PROJ this crate ports.
pub const PATCH: u32 = 0;

/// `PROJ_COMPUTE_VERSION(maj, min, patch)`.
#[must_use]
pub const fn compute(major: u32, minor: u32, patch: u32) -> u32 {
    major * 10_000 + minor * 100 + patch
}

/// `PROJ_VERSION_NUMBER`: the version of this crate's PROJ, as one number.
pub const NUMBER: u32 = compute(MAJOR, MINOR, PATCH);

/// `PROJ_AT_LEAST_VERSION(maj, min, patch)`: `true` when this crate's PROJ is
/// at least the given version.
#[must_use]
pub const fn at_least(major: u32, minor: u32, patch: u32) -> bool {
    NUMBER >= compute(major, minor, patch)
}
