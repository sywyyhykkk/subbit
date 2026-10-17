# Contributing services

Each service is one JSON file in `catalog/services/`; categories live in `catalog/categories.json` and bundled icons in `catalog/icons/`.

1. Copy a nearby service file. Use a permanent lowercase kebab-case `id` and matching filename.
2. Provide the official service name, category ID, search aliases and official pricing/source URL.
3. Add region information and the date you reviewed the source. Plan names must match that source; do not guess price, currency, or region availability. Poe uses a custom points entry because point allowances change. Mark old plans `retired: true` rather than removing them.
4. Give each plan a stable ID and a `kind` of `recurring` or `lifetime`. Optionally set `cycleUnit` (`day`, `week`, `month`, or `year`) when the official billing period is fixed; it prefills the editable cycle. Prices are always entered by the user. Never turn a lifetime license into a recurring plan.
5. Bundle an SVG or PNG and record `iconSource` and `iconLicense`. Icons should work offline. Do not add tracking URLs or remote image loading.
6. Run `npm run catalog:check` and `npm run check`.

The app and Rust MCP catalog are assembled from these files at build time. User records retain name, category, plan, original price and custom icon snapshots. Catalog changes do not rewrite saved subscriptions.

VPN intentionally has no provider list. Add it through Custom service and use the VPN category.

Brand names and icons identify their respective products. The Subbit MIT license does not grant rights to third-party trademarks. Simple Icons assets retain their indicated upstream license; official favicons remain owned by their brands. See each JSON file for provenance. Letter fallbacks are original MIT assets.
