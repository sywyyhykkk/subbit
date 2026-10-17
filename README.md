# Subbit

[English](README.md) | [简体中文](README.zh-CN.md)

**数字生活，心中有数。** A small, local-first companion for your subscriptions and lifetime software purchases.

Rust + Tauri 2 + Svelte 5. macOS and Windows. No account, server, or bundled browser.

## Download and install

Get [Subbit 0.0.1](https://github.com/sywyyhykkk/subbit/releases/tag/v0.0.1):

- [macOS Apple Silicon DMG](https://github.com/sywyyhykkk/subbit/releases/download/v0.0.1/Subbit_0.0.1_macos-arm64.dmg): macOS 12+, Apple M-series chips.
- [macOS Intel DMG](https://github.com/sywyyhykkk/subbit/releases/download/v0.0.1/Subbit_0.0.1_macos-x86_64.dmg): macOS 12+, Intel processors.
- [Windows x64 setup EXE](https://github.com/sywyyhykkk/subbit/releases/download/v0.0.1/Subbit_0.0.1_windows-x64-setup.exe): Windows 10/11 on Intel/AMD. Run the installer, then launch from the Start menu.

On macOS, check Apple menu → About This Mac to identify your chip, choose the matching DMG, then drag Subbit into Applications.

No development tools are required. Windows installs WebView2 over the internet if it is missing. Choose an installer rather than GitHub's “Source code” archives. This release is not publisher-signed or Apple-notarized, so Gatekeeper or SmartScreen may require confirmation at first launch. See the [release notes](docs/releases/0.0.1.md) for installation conditions and validation details.

## What it does

- Menu bar / system tray overview and a standalone management window. Closing windows keeps the tray alive and releases their WebViews.
- Hourly, monthly and yearly amortized **billed cost** and **personal cost** after reimbursement.
- Recurring purchases with an optional expiry date and calendar-aware automatic renewal. Turn off renewal to stop counting a subscription after its expiry day.
- Lifetime purchases amortized over elapsed calendar days. As ownership time grows, average cost decreases.
- Full, fixed-amount and percentage reimbursement; Apple App Store, Google Play, bank card, Alipay, WeChat Pay, PayPal, crypto and other payment methods.
- 47 bundled services in 9 categories, including cloud servers from AWS, Google Cloud, Alibaba Cloud, Vultr, DigitalOcean, Hetzner, Tencent Cloud, Huawei Cloud, Microsoft Azure and OVHcloud. VPN is a generic category; custom names, plans and local icons are supported.
- 26 fiat currencies plus USDT, USDC, BTC, ETH and SOL. Automatic cached exchange rates, global fixed rates and a fixed acquisition rate per asset.
- Chinese / English, system / light / dark appearance, configurable spending levels and optional launch at login.
- Local JSON, backup before writes, selectable storage directory, import preview and full export including custom icons.
- Five local stdio MCP tools for Agent-assisted import and querying.

Verified fixed-price plans prefill a USD reference price and convert it to the selected currency using your fixed or cached exchange rate. Monthly and annual billing use their own published prices. You can edit the amount or choose a custom plan; saved bills remain unchanged. Usage-based cloud servers, custom quotes and plans without verified prices require manual entry. Regional pricing and taxes may differ from the reference. Subbit records costs; it never charges a card or renews a real service.

## Development

Install Node.js 24+, a stable Rust toolchain and [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). On macOS use Xcode Command Line Tools. Windows requires the Microsoft C++ build tools and WebView2.

```sh
npm ci
npm run tauri dev
```

`npm run dev` alone provides a browser UI preview. Persistence, import/export, Agent access and system integration require the desktop application.

```sh
npm run check
npm run catalog:check
npm run test:pricing
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo build --locked --manifest-path src-tauri/Cargo.toml
python3 scripts/smoke-mcp.py
npm run tauri build
```

Build outputs are in `src-tauri/target/release/bundle/`. CI builds native macOS and Windows artifacts on their respective operating systems. Windows uses the installed system WebView2 or downloads the bootstrapper; it does not bundle an offline browser runtime. macOS development bundles are not Apple Developer signed or notarized. The single-platform, single-architecture installer target is under 50 MB; artifact sizes should be measured per build.

## Cost and date rules

| Record | Annualized billed cost |
| --- | --- |
| Every N months | Price × 12 ÷ N |
| Every N years | Price ÷ N |
| Every N weeks / days | Price × 365 ÷ (7 × N) / Price × 365 ÷ N |
| Lifetime purchase | Price ÷ elapsed days × 365 |

Monthly = annual ÷ 12; hourly = annual ÷ 8,760. Lifetime elapsed days include the purchase date and have a one-day minimum. Future purchases, paused records and expired nonrenewing subscriptions do not contribute. These are average costs, not historical debit transactions or measured usage time.

Reimbursements apply to the original price and billing period before currency conversion. Fixed reimbursement cannot exceed the price; percentage must be 0–100. Amounts are decimal strings (up to 12 decimal places); Rust performs decimal arithmetic and display rounding happens only in the UI.

The optional expiry date is an anchor. Without one, the first renewal is one billing cycle after the start date. Renewal always advances from the original anchor, preserving Jan 31 → Feb 28 → Mar 31 and leap-year boundaries. A subscription remains active on its expiry date and expires the next day. The saved anchor stays unchanged while the effective next date is computed on read, including after the app has been closed for months.

For lifetime assets, the settings' hourly/monthly/yearly cost keeps decreasing as elapsed days increase. On the first day the annualized number can be large; this is an extrapolated average, not another upcoming charge.

## Storage and exchange rates

The default file is the operating system's actual **Documents/Subbit/subscriptions.json**. The OS application configuration directory stores only the chosen location pointer and a lock file. Custom icons are resized to 96 × 96 and embedded as PNG data URLs, so backups are portable.

Writes take a process-wide file lock shared by desktop and MCP processes, validate the complete data, preserve `subscriptions.json.bak`, fsync a same-directory temporary file and atomically replace the current file. Stale desktop edits are rejected. If a selected file disappears, Subbit shows its path instead of silently starting an empty library. Use **Locate existing data** to recover the file.

Import previews the record count, then replaces current data after an explicit click; the previous file is backed up. MCP appends batches instead, with duplicate detection and stable import receipts. Importing a backup never turns on MCP implicitly.

Exchange providers: [Frankfurter](https://frankfurter.dev/) for fiat and [Coinbase](https://docs.cdp.coinbase.com/coinbase-app/track-apis/exchange-rates) for crypto. Rates are requested when opening the app: daily for fiat, after 15 minutes for crypto if crypto records exist. There is no background price polling. Fixed rates override cached rates. No available rate means an unavailable total, never a silently omitted amount or assumed 1:1 conversion.

Billing currency is distinct from payment method: a USD-priced subscription paid using USDT remains USD-priced. A subscription priced directly in BTC follows the BTC quote unless you set a fixed acquisition rate. All fixed rates mean **USD per one original currency unit**.

## Use Subbit with an Agent (MCP)

Subbit includes a local MCP server so an MCP-compatible Agent can read your subscriptions, summarize costs and import records from information you provide. It runs as `subbit --mcp` over stdio, using the same local data as the desktop app, without opening a window or network port.

1. Install or move Subbit to its final location, then open it.
2. Open **Settings → Agent / MCP**, enable MCP and save.
3. Copy the generated configuration into your Agent client's MCP configuration. It contains the executable's actual path and `args: ["--mcp"]`; adapt the format if your client requires it. On Windows, the command points to `subbit.exe`.
4. Reconnect the Agent client. Ask it to read the catalog and your current subscriptions before importing anything.

| Tool | What the Agent can do |
| --- | --- |
| `get_catalog` | Read services, plans, verified USD reference prices, currencies and the record schema |
| `list_subscriptions` | Read saved subscriptions and their revision |
| `get_summary` | Calculate hourly, monthly or yearly costs in a chosen currency |
| `preview_import` | Validate a batch and report duplicates before writing |
| `import_subscriptions` | Append a batch with a stable `importId` so retries do not duplicate it |

Example request: “Use Subbit to summarize my monthly subscription costs in CNY.” For imports: “Read this bill, prepare Subbit records and show me the import preview before importing.” Amounts must come from your bill or an explicitly chosen reference price; MCP does not automatically fill missing fields or fetch exchange rates. Subbit stays local, but records returned to a cloud Agent are processed by that Agent.

See [MCP configuration and import examples](docs/MCP.md) and the [subscription JSON schema](catalog/subscription.schema.json).

## Contribute service presets

Presets are shared by the desktop picker and MCP. To add a service or fix its plans:

1. Copy a similar JSON file in `catalog/services/`. Give it a stable lowercase kebab-case ID matching its filename, the official name, search aliases, category, source URL, review date and region information.
2. Add plans with stable IDs and `kind: "recurring"` or `"lifetime"`. Mark discontinued plans `retired: true` instead of deleting them.
3. For verified fixed prices, add USD `prices`, `priceSource` and `priceReviewedAt`. Use the full annual charge for yearly billing, omit the cycle for lifetime purchases, and mark per-seat prices. Leave usage-based, custom-quoted or unverified amounts unset.
4. Bundle an SVG or PNG in `catalog/icons/` and record its source and license. Add a category to `catalog/categories.json` only when needed, with both Chinese and English labels.
5. Run `npm run catalog:check`, `npm run test:pricing` and `npm run check`, then submit a pull request with the catalog files, icon and official source links.

See the [complete catalog contribution guide](catalog/CONTRIBUTING.md). Catalog updates preserve existing saved names, plans and amounts. For personal-only entries, choose **Custom service** in the app; no repository contribution is needed.

Application artwork is temporary. Brand icons retain their respective licenses and trademark rights; source metadata lives beside every service entry. Code and original assets are [MIT licensed](LICENSE).
