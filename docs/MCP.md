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
| `get_summary` | `currency` and `period` (`hour`, `month`, `year`) cost summary |

Read the catalog/schema first. Generate stable record IDs, preserve original currencies and decimal strings, and do not guess missing amounts. Always preview. Import retries must reuse the same `importId` and identical records. Identical record IDs are skipped; conflicting IDs reject the whole batch. Matching service names with different IDs require `allowDuplicates: true`, for intentional separate accounts.

Catalog `prices` are optional reference prices, not saved bills. Each quote has `amountUsd` and, for recurring purchases, a `cycleUnit` and optional `cycleCount` (default 1). Yearly quotes contain the full annual charge. Check `priceSource`, `priceReviewedAt` and `priceBasis` before using a quote; per-seat amounts cover one seat. Only use a reference price when the user chooses it. MCP imports still require complete records and do not automatically apply catalog prices or currency conversion.

All tools check the enabled setting on every call. Desktop and MCP use the same file lock, validation, atomic replacement and backup. Foreground windows refresh within 15 seconds or when focused. No polling occurs when the interface is closed.

`get_summary` returns null totals and `missingRates` when conversion is unavailable. It does not fetch the network or assume missing rates are 1:1. Update rates in the app or configure fixed rates.

For development and isolated tests, `SUBBIT_CONFIG_DIR` overrides both the configuration and initial data directories. Never point test runs at personal data.

Run `python3 scripts/smoke-mcp.py` against the debug executable, or pass a built executable path, for example `python3 scripts/smoke-mcp.py src-tauri/target/release/bundle/macos/Subbit.app/Contents/MacOS/subbit`. The script creates temporary isolated data and checks that the executable's catalog matches the repository.

Data returned to a cloud Agent is processed by that Agent, even though Subbit itself stores everything locally.

## Example import record

All fields in `catalog/subscription.schema.json` are required; optional values use `null` or an empty string. For example:

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
