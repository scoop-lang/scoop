//! Internal, synchronous text operations for Scoop's ordinary C FFI.
//!
//! Managed input is borrowed only for the current call. Each owned result uses
//! the matching release function declared in the C header. Upstream
//! allocation failure and internal panic abort; format errors return statuses.

#![no_std]
extern crate alloc;
mod allocation;

mod bytes;
#[cfg(not(target_os = "macos"))]
mod domain;
mod grapheme;
#[cfg(not(target_os = "macos"))]
mod normalize;
mod punycode;
#[cfg(not(target_os = "macos"))]
mod unicode;
mod width;

mod charset;
