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
| `get_catalog` | Categories, services, plans, currencies and full record schema |
| `list_subscriptions` | Current original records and revision |
| `preview_import` | Validate records; report added, skipped and possible duplicate IDs |
| `import_subscriptions` | Atomically append a batch with a stable `importId` |
| `get_summary` | `currency` and `period` (`hour`, `month`, `year`) cost summary |

Read the catalog/schema first. Generate stable record IDs, preserve original currencies and decimal strings, and do not guess missing amounts. Always preview. Import retries must reuse the same `importId` and identical records. Identical record IDs are skipped; conflicting IDs reject the whole batch. Matching service names with different IDs require `allowDuplicates: true`, for intentional separate accounts.

All tools check the enabled setting on every call. Desktop and MCP use the same file lock, validation, atomic replacement and backup. Foreground windows refresh within 15 seconds or when focused. No polling occurs when the interface is closed.

`get_summary` returns null totals and `missingRates` when conversion is unavailable. It does not fetch the network or assume missing rates are 1:1. Update rates in the app or configure fixed rates.

For development and isolated tests, `SUBBIT_CONFIG_DIR` overrides both the configuration and initial data directories. Never point test runs at personal data.

Data returned to a cloud Agent is processed by that Agent, even though Subbit itself stores everything locally.
