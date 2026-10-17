export interface Subscription {
  id: string;
  name: string;
  serviceId: string | null;
  category: string;
  plan: string;
  icon: string | null;
  amount: string;
  currency: string;
  kind: "recurring" | "lifetime";
  cycleUnit: string;
  cycleCount: number;
  purchasedAt: string;
  expiresAt: string | null;
  autoRenew: boolean;
  active: boolean;
  reimbursement: string;
  reimbursementValue: string;
  paymentMethod: string;
  notes: string;
  fixedUsdRate: string | null;
}
export interface Settings {
  currency: string;
  period: string;
  language: string;
  theme: string;
  currencySymbol: boolean;
  showLevel: boolean;
  levelBasis: string;
  mcpEnabled: boolean;
  autoRates: boolean;
  fixedRates: Record<string, string>;
  levels: { above: string; name: string }[];
}
export interface Database {
  schemaVersion: number;
  revision: number;
  subscriptions: Subscription[];
  settings: Settings;
  rates: Record<string, { usd: string; asOf: string }>;
  imports: Record<string, unknown>;
}
export interface CostRow {
  id: string;
  current: boolean;
  dueDate: string | null;
  bill: string | null;
  actual: string | null;
  elapsedDays: number;
}
export interface Snapshot {
  database: Omit<Database, "imports">;
  key: {
    file: { path: string; length: number | null; modifiedAt: string | null };
    currency: string;
    period: string;
    day: string;
  } | null;
  path: string;
  executable: string;
  level: string | null;
  summary: {
    bill: string | null;
    actual: string | null;
    reimbursed: string | null;
    currency: string;
    period: string;
    missingRates: string[];
    rows: CostRow[];
    activeCount: number;
  };
}
export interface Category {
  id: string;
  zh: string;
  en: string;
  icon: string;
}
export interface Catalog {
  schemaVersion: number;
  categories: Category[];
  services: Service[];
  icons: Record<string, string>;
}
export interface CatalogState {
  catalog: Catalog;
  updatedAt: number | null;
}
export interface Service {
  id: string;
  name: string;
  category: string;
  aliases: string[];
  icon: string;
  source: string;
  reviewedAt: string;
  plans: Plan[];
}
export interface PlanPrice {
  amountUsd: string;
  cycleUnit?: "day" | "week" | "month" | "year";
  cycleCount?: number;
}
export interface Plan {
  id: string;
  name: string;
  kind: "recurring" | "lifetime";
  cycleUnit?: "day" | "week" | "month" | "year";
  retired: boolean;
  legacy?: boolean;
  prices?: PlanPrice[];
  priceSource?: string;
  priceReviewedAt?: string;
  priceBasis?: "per-seat";
}
