//! JSON-RPC control wire: socket paths, framing, peer credentials, and types.
//!
//! This crate does not bind a socket. The owner lives in `vissue-serve`.

pub mod frame;
pub mod path;
pub mod rpc;

#[cfg(unix)]
pub mod client;
#[cfg(unix)]
pub mod peercred;

pub use frame::{
    classify_first_line, is_header_line, read_message, write_message, FrameError, Framing,
    MAX_HEADER_LINES, MAX_MESSAGE_BYTES,
};
pub use path::{
    beside_socket, control_log_path, default_socket_path, hud_log_path, hud_socket_path,
    runtime_dir, socket_lock_path, socket_pid_path, HUD_LOG_ENV, HUD_SOCKET_ENV, SERVE_LOG_ENV,
    SOCKET_ENV,
};
pub use rpc::{
    error_from_core, invalid_params, invalid_request, method_not_found, parse_error,
    parse_initialize_params, Error, Event, InitializeParams, InitializeResult, JsonRpcError,
    JsonRpcId, JsonRpcRequest, JsonRpcResponse, Method, Notification, Request, Response,
    PROTOCOL_VERSION, V1_CAPABILITIES,
};
pub use vissue_core::views::*;
