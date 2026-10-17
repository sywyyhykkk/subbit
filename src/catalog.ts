import type { Service } from "./types";
import categories from "../catalog/categories.json";
export { categories };
export const services = Object.values(
  import.meta.glob("../catalog/services/*.json", {
    eager: true,
    import: "default",
  }),
) as Service[];
const icons = import.meta.glob("../catalog/icons/*", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;
export function serviceIcon(id: string | null) {
  return id
    ? (icons[`../catalog/icons/${id}.svg`] ??
        icons[`../catalog/icons/${id}.png`])
    : undefined;
}
export const currencies =
  "CNY USD EUR GBP JPY HKD TWD KRW SGD PHP MYR THB IDR VND TRY INR BRL ARS MXN AUD CAD NZD CHF AED SAR ZAR USDT USDC BTC ETH SOL".split(
    " ",
  );
export const currencyNames = [
  "人民币",
  "美元",
  "欧元",
  "英镑",
  "日元",
  "港币",
  "新台币",
  "韩元",
  "新加坡元",
  "菲律宾比索",
  "马来西亚林吉特",
  "泰铢",
  "印度尼西亚卢比",
  "越南盾",
  "土耳其里拉",
  "印度卢比",
  "巴西雷亚尔",
  "阿根廷比索",
  "墨西哥比索",
  "澳元",
  "加元",
  "新西兰元",
  "瑞士法郎",
  "阿联酋迪拉姆",
  "沙特里亚尔",
  "南非兰特",
  "Tether",
  "USDC",
  "Bitcoin",
  "Ether",
  "Solana",
];
export const paymentMethods = [
  ["apple", "Apple 商店", "Apple App Store"],
  ["google", "谷歌商店", "Google Play"],
  ["card", "银行卡", "Bank card"],
  ["alipay", "支付宝", "Alipay"],
  ["wechat", "微信支付", "WeChat Pay"],
  ["paypal", "PayPal", "PayPal"],
  ["crypto", "加密货币", "Cryptocurrency"],
  ["other", "其他", "Other"],
];
export function defaultLevels(en = false) {
  return [
    { above: "0", name: en ? "Subscription Newcomer" : "订阅萌新" },
    { above: "100", name: en ? "Digital Explorer" : "数字玩家" },
    { above: "300", name: en ? "Productivity Pro" : "效率达人" },
    { above: "600", name: en ? "Cloud Collector" : "云端收藏家" },
    { above: "1000", name: en ? "Digital Tycoon" : "数字土豪" },
  ];
}
export const defaultSettings = {
  currency: "CNY",
  period: "month",
  language: "system",
  theme: "system",
  currencySymbol: true,
  showLevel: true,
  levelBasis: "actual",
  mcpEnabled: false,
  autoRates: true,
  fixedRates: {},
  levels: defaultLevels(),
};
