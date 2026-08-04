# gRPC client (codegen)

Like Apollo codegen: **schema → one command → typed client file**.

| | |
|--|--|
| Schema | [`vd-srv/proto`](../../cli/manage/vd-srv/proto) (`runtime.proto`) |
| Command | from `src/desktop`: **`npm run generate`** |
| Output | `src/gen/` (**gitignored** — regenerated locally / on `npm install`) |

```bash
# once — postinstall runs generate
cd src/desktop/grpc-client && npm install

# after any proto change (from src/desktop)
cd src/desktop
npm run generate
```

Import:

```ts
import {
  ExecutionService,
  OperatorService,
  EventService,
  type JobView,
} from "@voxdecoder/grpc-client";
```

Optional: `npm run generate:verify`  
Live Health: `SMOKE_LIVE=1 npm run generate:verify`  
URL: `.env` ← copy `.env.example` (`VD_SRV_GRPC`).

UI not wired yet.
