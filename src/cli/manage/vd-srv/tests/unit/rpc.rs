//! JSON-RPC wire + transport kind tests.

use vd_srv::api::rpc::{Id, Request, Response, JSONRPC_VERSION};
use vd_srv::api::TransportKind;

#[test]
fn transport_kind_parse() {
    assert_eq!(TransportKind::parse("auto"), Some(TransportKind::Auto));
    assert_eq!(TransportKind::parse("uds"), Some(TransportKind::Uds));
    assert_eq!(TransportKind::parse("unix"), Some(TransportKind::Uds));
    assert_eq!(TransportKind::parse("tcp"), Some(TransportKind::Tcp));
    assert_eq!(TransportKind::parse("pipe"), Some(TransportKind::Pipe));
    assert!(TransportKind::parse("nope").is_none());
}

#[test]
fn request_roundtrip() {
    let req = Request::call(Id::number(7), "server.ping", Some(serde_json::json!({})));
    let s = serde_json::to_string(&req).unwrap();
    let back: Request = serde_json::from_str(&s).unwrap();
    assert_eq!(back.jsonrpc, JSONRPC_VERSION);
    assert_eq!(back.method, "server.ping");
    assert!(matches!(back.id, Some(Id::Number(7))));
}

#[test]
fn response_error_shape() {
    let resp = Response::failure(
        Some(Id::number(1)),
        vd_srv::api::ErrorObject::method_not_found("nope"),
    );
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["jsonrpc"], "2.0");
    assert_eq!(v["error"]["code"], -32601);
    assert!(v.get("result").is_none());
}
