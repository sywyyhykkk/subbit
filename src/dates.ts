export function parseDate(value: string): Date | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(year, month - 1, day, 12);
  return year >= 1900 && year <= 9999 && date.getFullYear() === year &&
    date.getMonth() === month - 1 && date.getDate() === day ? date : null;
}

export function isoDate(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function nextCycle(start: string, unit: string, count: number): string | null {
  const date = parseDate(start);
  if (!date || !Number.isInteger(count) || count < 1 || count > 120) return null;
  if (unit === "month" || unit === "year") {
    const day = date.getDate();
    date.setDate(1);
    date.setMonth(date.getMonth() + count * (unit === "year" ? 12 : 1));
    const last = new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
    date.setDate(Math.min(day, last));
  } else {
    date.setDate(date.getDate() + count * (unit === "week" ? 7 : 1));
  }
  return date.getFullYear() <= 9999 ? isoDate(date) : null;
}
