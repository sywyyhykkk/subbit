# Contributing services

Each service is one JSON file in `catalog/services/`; categories live in `catalog/categories.json` and bundled icons in `catalog/icons/`.

1. Copy a nearby service file. Use a permanent lowercase kebab-case `id` and matching filename.
2. Provide the official service name, category ID, search aliases and official pricing/source URL.
3. Add region information and the date you reviewed the source. Plan names must match that source; do not guess price, currency, or region availability. Poe uses a custom points entry because point allowances change. Mark old plans `retired: true` rather than removing them.
4. Give each plan a stable ID and a `kind` of `recurring` or `lifetime`. Optionally set `cycleUnit` (`day`, `week`, `month`, or `year`) when the official billing period is fixed; it prefills the editable cycle. Never turn a lifetime license into a recurring plan.
5. For verified fixed USD list prices, add `prices`, e.g. `[{"amountUsd":"20.00","cycleUnit":"month"},{"amountUsd":"200.00","cycleUnit":"year"}]`, plus `priceSource` and `priceReviewedAt`. The first price is the default billing cycle. Annual amounts must be the full yearly charge, not a monthly equivalent; lifetime prices omit the cycle. Optional `cycleCount` defaults to 1. Mark seat prices with `priceBasis: "per-seat"`. Omit prices for custom quotes, usage billing, promotions, or unverified prices. Prices are editable references, converted using the record's fixed rate, the global fixed rate, or the cached rate, in that order. Missing rates or unmatched billing cycles require manual input. Catalog updates never overwrite saved bills.
6. Bundle an SVG or PNG and record `iconSource` and `iconLicense`. Icons should work offline. Do not add tracking URLs or remote image loading.
7. Run `npm run catalog:check`, `npm run test:pricing` and `npm run check`.

The app includes an offline catalog assembled from these files at build time. After catalog changes merge into `main`, the **Publish service catalog** workflow validates and packs categories, services and icons into one `catalog.json` on the dedicated `catalog` branch. This does not create an app release. To publish the initial catalog or retry a failed publication, run that workflow manually. `npm run catalog:bundle` generates the same bundle locally in `artifacts/catalog.json`.

Desktop and Rust MCP share a local catalog cache. The desktop loads cached data immediately, checks for updates on opening/focus and every 15 minutes while focused, and offers **Settings → Data → Update catalog**. Automatic attempts are at least 15 minutes apart; manual refresh bypasses that interval. MCP checks when `get_catalog` is called. An unavailable network, invalid bundle or failed write preserves the last complete cache; without a usable cache, the built-in catalog is used. Icons are embedded in the bundle and need no network once cached.

The cache lives in `catalog-cache/` under the OS application configuration directory, separate from the asset JSON and its backups. User records retain name, category, plan, original price and custom icon snapshots. New desktop assets also snapshot the chosen catalog icon as PNG. Catalog changes never rewrite saved subscriptions, settings, exchange rates or their revision. Open asset forms keep the catalog they started with until closed.

Install an app version with remote catalog support once. Later category, service, plan and icon changes require only a catalog publication. Incompatible catalog schema changes still require an app update; keep `schemaVersion: 1` for compatible directory changes.

VPN intentionally has no provider list. Add it through Custom service and use the VPN category.

Brand names and icons identify their respective products. The Subbit MIT license does not grant rights to third-party trademarks. Simple Icons assets retain their indicated upstream license; official favicons remain owned by their brands. See each JSON file for provenance. Letter fallbacks are original MIT assets.
