#![no_std]

// Code shared by the two keyboards, each binary (split.rs, mono.rs) uses what it needs.
pub mod hardware;
pub mod layouts;
pub mod options;
pub mod software;
