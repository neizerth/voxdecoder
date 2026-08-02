# Transport Architecture for `vd-srv`

Product: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md). CLI: [cli.md](cli.md).

**Status: implemented (control plane).** JSON-RPC 2.0 over a transport abstraction (framing: newline-delimited JSON). Primary IPC is Unix domain socket on Linux/macOS; TCP is optional. Windows Named Pipe is stubbed — Auto currently selects TCP on Windows until pipe transport ships.

---

## Purpose

`vd-srv` is the local execution server for VoxDecoder.

Desktop applications, `vd-pipeline`, `vd-meeting`, `vd-mcp`, future GUI tools and automation clients must communicate with the server through **one common RPC protocol**, independent of the underlying operating system.

The transport layer must therefore satisfy:

* Windows
* Linux
* macOS

without changing the application protocol.

---

## Design principles

### Protocol and transport are different layers

The RPC protocol is part of the public contract.

The transport is an implementation detail.

```text
Application
        │
 JSON-RPC
        │
 Transport
        │
 OS IPC
```

Every client speaks exactly the same JSON-RPC protocol regardless of how bytes are transported.

### Transport abstraction

The server must expose an internal transport abstraction.

```text
Client
        │
JsonRpcClient
        │
Transport
        │
JsonRpcServer
        │
vd-srv
```

Example implementations:

```text
UnixSocketTransport
NamedPipeTransport
TcpTransport
```

The RPC layer must never depend on a specific transport.

---

## Primary transport

### Local IPC

The primary transport is local IPC.

| Platform | Transport          |
| -------- | ------------------ |
| Windows  | Named Pipe         |
| Linux    | Unix Domain Socket |
| macOS    | Unix Domain Socket |

Advantages:

* no TCP stack
* no localhost port
* no firewall interaction
* lower latency
* lower overhead
* operating system security
* ideal for long-lived connections

Desktop applications, `vd-pipeline`, `vd-meeting` and `vd-mcp` use this transport by default.

---

## Optional transport

The server may additionally expose a TCP endpoint.

```text
HTTP
        │
WebSocket
        │
JSON-RPC
        │
vd-srv
```

This transport is optional and disabled by default.

Typical uses:

* browser UI
* remote automation
* REST gateway
* debugging
* integration with external tools

The Job protocol must remain identical.

---

## JSON-RPC

The application protocol is JSON-RPC 2.0.

Requests:

```json
{
  "id": 42,
  "method": "job.submit",
  "params": {
  }
}
```

Responses:

```json
{
  "id": 42,
  "result": {
  }
}
```

Errors:

```json
{
  "id": 42,
  "error": {
    "code": "...",
    "message": "..."
  }
}
```

---

## Notifications

The server also emits asynchronous notifications.

```json
{
  "method": "job.progress",
  "params": {
  }
}
```

Notifications do not contain an `id`.

---

## Single connection

A client opens one persistent connection.

The same connection carries:

* RPC requests
* RPC responses
* progress notifications
* events
* cancellation
* subscriptions

No additional sockets are required.

```text
Desktop
      │
      ├── job.submit
      ├── worker.list
      ├── job.cancel
      ├── queue.list
      │
      ├── JobQueued
      ├── NodeStarted
      ├── NodeProgress
      ├── ArtifactProduced
      └── JobFinished
```

---

## Connection model

Connections are long-lived.

Clients are expected to reconnect automatically.

Server restart handling should include:

* reconnect
* resubscribe
* continue observing Jobs

Jobs remain in the Job Store independently of active clients.

---

## Client SDK

Every frontend should use the same client library.

```text
Desktop
CLI
vd-mcp
Future GUI

        │

 VoxDecoder Client SDK

        │

 JsonRpcClient

        │

 Transport (auto)

        │

 Windows → Named Pipe
 Linux   → Unix Socket
 macOS   → Unix Socket
```

Applications never implement transports themselves.

---

## Transport selection

Automatic selection:

| Platform | Default            |
| -------- | ------------------ |
| Windows  | Named Pipe         |
| Linux    | Unix Domain Socket |
| macOS    | Unix Domain Socket |

Optional overrides:

```text
--transport pipe
--transport uds
--transport tcp
```

or configuration:

```yaml
transport:
  type: auto
```

---

## Security

The primary IPC transport relies on operating system security.

No authentication is required for local IPC by default.

When TCP is enabled, authentication and authorization become transport-specific and are outside the local IPC contract.

---

## Public API

The transport layer must expose the same RPC surface regardless of implementation.

Core methods include:

```text
server.ping
server.health
server.version

job.submit
job.cancel
job.pause
job.resume
job.status
job.list

queue.status

worker.list

artifact.list
artifact.info

subscribe
unsubscribe
```

Notifications include:

```text
ServerStarted

JobQueued
JobStarted
JobFinished
JobCancelled
JobFailed

NodeQueued
NodeStarted
NodeProgress
NodeFinished

ArtifactProduced

WorkerChanged
QueueChanged
```

---

## Goals

The transport architecture must provide:

* one RPC protocol
* one client SDK
* one server implementation
* zero platform-specific application code
* automatic transport selection
* persistent bidirectional connections
* minimal IPC overhead
* optional HTTP/WebSocket bridge
* identical behavior on Windows, Linux and macOS

---

## Non-goals

The transport layer does **not** define:

* Job schema
* Executor behavior
* capability scheduling
* artifact formats
* Meeting Model
* LLM providers
* HTTP REST API semantics

Those belong to their respective subsystems.

---

## Final architecture

```text
                 Desktop
                    │
            vd-pipeline CLI
                    │
                vd-meeting
                    │
                 vd-mcp
                    │
        ─────────────────────────
          VoxDecoder Client SDK
        ─────────────────────────
                    │
             JSON-RPC 2.0
                    │
          Transport Abstraction
        ┌───────────┼────────────┐
        │           │            │
 Named Pipe      Unix Socket    TCP
 (Windows)      (Linux/macOS) (optional)
        └───────────┼────────────┘
                    │
              JSON-RPC Server
                    │
                 vd-srv
                    │
               Job Scheduler
                    │
                 Executor
                    │
              VoxDecoder DAG
```

This architecture separates business logic from transport, keeps native performance on every supported OS, and allows an HTTP/WebSocket bridge later without changing the RPC protocol.
