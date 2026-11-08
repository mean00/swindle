//! Probe-side request parameter parser.
//!
//! The decoding itself lives in [`crate::rpc_wire::RpcParams`] — shared with the
//! host so the two halves are tested against the same codec — and this module
//! keeps the names the generated dispatch code uses.

pub use crate::rpc_wire::RpcParams as RpcParameterParser;
pub use crate::rpc_wire::RpcParams as rpc_parameter_parser;
