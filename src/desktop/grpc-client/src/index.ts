/**
 * Re-exports codegen + small Node helper.
 *
 * Prefer importing services from gen after `npm run generate` (from src/desktop):
 *   grpc-client/src/gen/runtime/v1/runtime_pb.ts
 */

import { config as loadEnv } from "dotenv";
import { createClient, type Client } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

import {
  EventService,
  ExecutionService,
  OperatorService,
  PlanningService,
} from "./gen/runtime/v1/runtime_pb.js";

export * from "./gen/runtime/v1/runtime_pb.js";

const here = dirname(fileURLToPath(import.meta.url));
// dist/ → package root; also works when run from src via tsx
loadEnv({ path: resolve(here, "../.env") });
loadEnv({ path: resolve(here, "../../.env") });

export type RuntimeClients = {
  operator: Client<typeof OperatorService>;
  execution: Client<typeof ExecutionService>;
  events: Client<typeof EventService>;
  planning: Client<typeof PlanningService>;
};

/** Fallback only if `.env` / env unset. Prefer `VD_SRV_GRPC` in `.env`. */
const FALLBACK_GRPC_BASE_URL = "http://127.0.0.1:7702";

/** gRPC base URL from `VD_SRV_GRPC` (`.env` or process env). */
export function grpcBaseUrl(): string {
  const url = process.env.VD_SRV_GRPC?.trim();
  return url && url.length > 0 ? url : FALLBACK_GRPC_BASE_URL;
}

/** Node-only helper (http2). Not for Vite webview. */
export function createRuntimeClients(baseUrl: string = grpcBaseUrl()): RuntimeClients {
  const transport = createGrpcTransport({ baseUrl });
  return {
    operator: createClient(OperatorService, transport),
    execution: createClient(ExecutionService, transport),
    events: createClient(EventService, transport),
    planning: createClient(PlanningService, transport),
  };
}
