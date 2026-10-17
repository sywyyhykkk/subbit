import type { Database, Plan, Subscription } from "./types";

// Stored exchange rates express USD per one unit of the original currency.
export function presetQuote(
  plan: Plan | undefined,
  record: Pick<Subscription, "kind" | "cycleUnit" | "cycleCount" | "currency" | "fixedUsdRate">,
  database: Pick<Database, "rates" | "settings"> | undefined,
) {
  const price = plan?.kind === record.kind
    ? plan.prices?.find((p) => record.kind === "lifetime" ||
        (p.cycleUnit === record.cycleUnit && (p.cycleCount ?? 1) === record.cycleCount))
    : undefined;
  if (!price) return { status: "unpriced" as const };
  const code = record.currency.trim().toUpperCase();
  const rate = code === "USD" ? 1 : Number(
    record.fixedUsdRate ?? database?.settings.fixedRates[code] ?? database?.rates[code]?.usd,
  );
  if (!Number.isFinite(rate) || rate <= 0)
    return { status: "missing-rate" as const, price };
  const amount = Number(price.amountUsd) / rate;
  if (!Number.isFinite(amount)) return { status: "missing-rate" as const, price };
  const digits = ["BTC", "ETH", "SOL", "USDT", "USDC"].includes(code) ? 8 :
    new Intl.NumberFormat("en", { style: "currency", currency: /^[A-Z]{3}$/.test(code) ? code : "USD" })
      .resolvedOptions().maximumFractionDigits;
  return { status: "ready" as const, price, amount: amount.toFixed(digits) };
}
