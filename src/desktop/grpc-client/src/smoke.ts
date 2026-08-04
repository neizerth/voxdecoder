/**
 * Smoke: build + import typed client.
 *
 * Offline: constructs clients (no network).
 * Live Health: `SMOKE_LIVE=1` (+ `vd-srv serve --grpc`, URL from `.env` / `VD_SRV_GRPC`).
 */

import { createRuntimeClients, grpcBaseUrl, OperatorService } from "./index.js";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) {
    throw new Error(msg);
  }
}

async function main(): Promise<void> {
  assert(OperatorService.typeName.includes("OperatorService"), "OperatorService missing");
  assert(
    typeof OperatorService.method?.health === "object",
    "health method descriptor missing",
  );

  const baseUrl = grpcBaseUrl();
  const clients = createRuntimeClients(baseUrl);
  assert(typeof clients.operator.health === "function", "operator.health not a function");
  assert(typeof clients.execution.getJob === "function", "execution.getJob not a function");
  assert(typeof clients.events.watchJob === "function", "events.watchJob not a function");

  console.log("grpc-client smoke: constructs OK");
  console.log(`  OperatorService=${OperatorService.typeName}`);
  console.log(`  baseUrl=${baseUrl}`);

  if (process.env.SMOKE_LIVE !== "1") {
    console.log("grpc-client smoke: skip live Health (set SMOKE_LIVE=1 to hit server)");
    return;
  }

  const health = await clients.operator.health({});
  console.log("grpc-client smoke: Health OK", {
    workers: health.workers,
    workersBusy: health.workersBusy,
    dataDir: health.dataDir,
  });
}

main().catch((err: unknown) => {
  console.error("grpc-client smoke failed:", err);
  process.exit(1);
});
