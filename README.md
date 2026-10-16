# Subbit

**数字生活，心中有数。** A small, local-first companion for your subscriptions and lifetime software purchases.

Rust + Tauri 2 + Svelte 5. macOS and Windows. No account, server, or bundled browser.

## What it does

- Menu bar / system tray overview and a standalone management window. Closing windows keeps the tray alive and releases their WebViews.
- Hourly, monthly and yearly amortized **billed cost** and **personal cost** after reimbursement.
- Recurring purchases with an optional expiry date and calendar-aware automatic renewal. Turn off renewal to stop counting a subscription after its expiry day.
- Lifetime purchases amortized over elapsed calendar days. As ownership time grows, average cost decreases.
- Full, fixed-amount and percentage reimbursement; Apple App Store, Google Play, bank card, Alipay, WeChat Pay, PayPal, crypto and other payment methods.
- 36 bundled services across media, productivity, AI, cloud storage, developer tools and design. VPN is a generic category; custom names, plans and local icons are supported.
- 26 fiat currencies plus USDT, USDC, BTC, ETH and SOL. Automatic cached exchange rates, global fixed rates and a fixed acquisition rate per asset.
- Chinese / English, system / light / dark appearance, configurable spending levels and optional launch at login.
- Local JSON, backup before writes, selectable storage directory, import preview and full export including custom icons.
- Five local stdio MCP tools for Agent-assisted import and querying.

Prices are entered from your own bill. Preset plan names are editable and region-dependent. Subbit records costs; it never charges a card or renews a real service.

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

## Agent access and contributions

See [MCP setup](docs/MCP.md) and the [subscription JSON schema](catalog/subscription.schema.json). Enable MCP in settings before connecting an Agent. The executable supports `subbit --mcp` without creating a desktop window.

Add services and categories through [catalog contributions](catalog/CONTRIBUTING.md). Plans have stable IDs and an explicit recurring/lifetime type. Catalog updates preserve the snapshots in existing records. User-defined services can be reused from the picker while their saved records exist.

Application artwork is temporary. Brand icons retain their respective licenses and trademark rights; source metadata lives beside every service entry. Code and original assets are [MIT licensed](LICENSE).
