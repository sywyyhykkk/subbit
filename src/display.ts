export function localTimestamp(value: string | number | null | undefined): string {
  if (value == null) return "—";
  // Date-only source quotes have no time of day; do not invent midnight.
  if (typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value)) return value;
  const date = new Date(typeof value === "number" ? value * 1000 : value);
  if (!Number.isFinite(date.getTime())) return "—";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
export function bytesLabel(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

export function reimbursementStatus(item: { reimbursement: string; reimbursementValue: string; amount: string }): "full" | "partial" | "none" {
  const normalized = (value: string) => {
    const [whole, fraction = ""] = value.split(".");
    const integer = whole.replace(/^0+(?=\d)/, "") || "0";
    const decimals = fraction.replace(/0+$/, "");
    return decimals ? `${integer}.${decimals}` : integer;
  };
  const value = normalized(item.reimbursementValue), amount = normalized(item.amount);
  if (item.reimbursement === "full") return "full";
  if (!["fixed", "percent"].includes(item.reimbursement) || value === "0") return "none";
  if ((item.reimbursement === "percent" && value === "100") || (item.reimbursement === "fixed" && amount !== "0" && value === amount)) return "full";
  return "partial";
}
