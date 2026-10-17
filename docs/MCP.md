# Local Agent / MCP

Enable **Settings → Agent / MCP**, save, and copy the generated configuration. The executable path comes from the running app, so move the app into its final installation location before copying.

```json
{
  "mcpServers": {
    "subbit": {
      "command": "/Applications/Subbit.app/Contents/MacOS/subbit",
      "args": ["--mcp"]
    }
  }
}
```

On Windows use the installed `subbit.exe` path. MCP uses stdio JSON-RPC and the same executable, without a window, port, Node or Python runtime. Protocol versions 2024-11-05, 2025-03-26, 2025-06-18 and 2025-11-25 are supported. Tool results are JSON encoded in a text content block.

| Tool | Purpose |
| --- | --- |
| `get_catalog` | Categories, services, plans with verified USD reference prices, currencies and full record schema |
| `list_subscriptions` | Current original records and revision |
| `preview_import` | Validate records; report added, skipped and possible duplicate IDs |
| `import_subscriptions` | Atomically append a batch with a stable `importId` |
| `get_summary` | `currency` and `period` (`hour`, `day`, `month`, `year`) cost summary |

Read the catalog/schema first. Generate stable record IDs, preserve original currencies and decimal strings, and do not guess missing amounts. Always preview. Import retries must reuse the same `importId` and identical records. Identical record IDs are skipped; conflicting IDs reject the whole batch. Matching service names with different IDs require `allowDuplicates: true`, for intentional separate accounts.

Catalog `prices` are optional reference prices, not saved bills. Each quote has `amountUsd` and, for recurring purchases, a `cycleUnit` and optional `cycleCount` (default 1). Yearly quotes contain the full annual charge. Check `priceSource`, `priceReviewedAt` and `priceBasis` before using a quote; per-seat amounts cover one seat. Only use a reference price when the user chooses it. MCP imports still require complete records and do not automatically apply catalog prices or currency conversion.

`get_catalog` shares the desktop's cached directory, including embedded icon data URLs in `catalog.icons` keyed by the service's `icon` path. When offline mode is off, it checks for a remote update at most once per 15 minutes, with a 15-second request timeout. On failure it returns the last complete cache, or the built-in catalog if no usable cache exists. Catalog refresh does not modify the asset database or revision and needs no app release for compatible directory changes.

All tools check the enabled setting on every call. Desktop and MCP use the same file lock, validation, atomic replacement and backup. Foreground windows refresh within 15 seconds or when focused. No polling occurs when the interface is closed.

`get_summary` returns null totals and `missingRates` when conversion is unavailable. It does not fetch the network or assume missing rates are 1:1. Update rates in the app or configure fixed rates.

For development and isolated tests, `SUBBIT_CONFIG_DIR` overrides both the configuration and initial data directories. Never point test runs at personal data.

Run `python3 scripts/smoke-mcp.py` against the debug executable, or pass a built executable path, for example `python3 scripts/smoke-mcp.py src-tauri/target/release/bundle/macos/Subbit.app/Contents/MacOS/subbit`. The script creates temporary isolated data and checks that the executable's catalog matches the repository.

Data returned to a cloud Agent is processed by that Agent, even though Subbit itself stores everything locally.

## Example import record

Use the required fields listed in `catalog/subscription.schema.json`; `registrar` is optional, and nullable values may use `null`. For example:

```json
{
  "id": "my-apple-music-account",
  "name": "Apple Music",
  "serviceId": "apple-music",
  "category": "media",
  "plan": "Individual",
  "icon": null,
  "amount": "11",
  "currency": "CNY",
  "kind": "recurring",
  "cycleUnit": "month",
  "cycleCount": 1,
  "purchasedAt": "2026-10-16",
  "expiresAt": null,
  "autoRenew": true,
  "active": true,
  "reimbursement": "none",
  "reimbursementValue": "0",
  "paymentMethod": "apple",
  "notes": "Illustrative price; replace with the user's actual bill.",
  "fixedUsdRate": null
}
```

Pass it as `{"records":[...]}` to `preview_import`, then `{"importId":"stable-batch-id","records":[...]}` to `import_subscriptions`.


## Offline rate and catalog imports

These tools receive data directly and never fetch a supplied URL. Enable MCP in the GUI first. The application offline policy is shared with MCP and survives asset-file replacement. `get_catalog` reads only local data while offline. Agents manage their own network access independently.

| Tool | Arguments and result |
| --- | --- |
| `get_exchange_rates` | No arguments. Returns effective local `rates`, `fixedRates`, database `revision`, `bundledSource` and `offline`. |
| `preview_exchange_rates` | `{ "rates": { "CNY": { "usd": "0.14", "asOf": "2026-10-18T00:00:00Z" } } }`; returns the database revision. Values are illustrative; use verified quotes. |
| `import_exchange_rates` | The same `rates`, a stable `importId`, and the preview's `revision`. Updates cache quotes while preserving fixed rates and assets. |
| `preview_catalog` | `{ "catalog": ... }`, using the complete catalog shape returned by `get_catalog`, including embedded icons. Returns catalog revision and counts. |
| `import_catalog` | The same `catalog`, a stable `importId`, and the preview's `revision`. Replaces the local catalog cache, not saved assets. |

Rate imports require supported currency codes, positive decimal-string USD-per-unit quotes, RFC3339 timestamps, and USD = 1 if supplied. The bundled fallback has source dates only and is not a live quote. Catalog and database revisions are separate: always use the corresponding preview result. Stale revisions fail; retrying an identical import ID and payload is safe, while reusing an ID with different data fails. Imports keep write locks, atomic replacement and previous-file backups. Catalogs also retain import receipts and their own `.bak`.

Domain imports use `category: "domain"`, the hostname in `name`, optional `registrar`, and `kind: "recurring"`. Use one record per domain. MCP does not fetch favicons; a supplied raster data URL can be stored in `icon`, otherwise the GUI shows the globe fallback.
