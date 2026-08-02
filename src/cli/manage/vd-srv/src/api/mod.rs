//! Control plane: JSON-RPC 2.0 over a transport abstraction.
//!
//! See [TRANSPORT.md](../../TRANSPORT.md).

mod client;
mod dispatch;
pub mod grpc;
pub mod http;
pub mod openapi;
pub mod rpc;
pub mod transport;

pub use client::{call, JsonRpcClient, RpcError};
pub use rpc::{ErrorObject, Id, Notification, Request, Response, JSONRPC_VERSION};
pub use transport::{resolve_endpoint, serve, Endpoint, TransportKind};
