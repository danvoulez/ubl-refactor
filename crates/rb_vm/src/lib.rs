//! Deprecated compatibility shim for `ubl_vm`.
#![deprecated(note = "rb_vm is deprecated; use ubl_vm")]

pub use ubl_vm::*;
pub use ubl_vm::{canon, disasm, exec, opcode, providers, tlv, types};
