#![no_std]
#![allow(clippy::disallowed_methods, clippy::single_match)]
#![allow(dead_code, unused_variables, unused_imports, internal_features, static_mut_refs)]
extern crate alloc;

use core::fmt::Debug;

pub mod color_map;
pub mod blitter;
pub mod gametank_bus;
pub mod cartridges;
pub mod emulator;
pub mod inputs;
