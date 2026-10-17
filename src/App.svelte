<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import ReleaseNotes from "./ReleaseNotes.svelte";
  import { localTimestamp, bytesLabel } from "./display";
  import DatePicker from "./DatePicker.svelte";
  import { nextCycle } from "./dates";
  import { createReloadQueue } from "./reload";
  import { localizeLevelName } from "./levels";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import appInfo from "../package.json";
  import appLight from "../src-tauri/icons/about-light.png";
  import appDark from "../src-tauri/icons/about-dark.png";
  import { presetQuote } from "./pricing";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open, save, confirm } from "@tauri-apps/plugin-dialog";
  import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
  import {
    categories as bundledCategories,
    services as bundledServices,
    serviceIcon as bundledServiceIcon,
    currencies,
    currencyNames,
    paymentMethods,
    defaultSettings,
    defaultLevels,
  } from "./catalog";
  import type {
    Subscription,
    Settings,
    Snapshot,
    Service,
    Database,
    CatalogState,
  } from "./types";

  const panel = new URLSearchParams(location.search).has("panel");
  let snap: Snapshot | null = $state(null);
  let directory: CatalogState = $state({ catalog: { schemaVersion: 1, categories: bundledCategories, services: bundledServices, icons: {} }, updatedAt: null });
  let pendingDirectory: CatalogState | null = $state(null);
  let catalogBusy = $state(false), catalogError = $state(false);
  const categories = $derived(directory.catalog.categories.some((c) => c.id === "domain") ? directory.catalog.categories : [...directory.catalog.categories, bundledCategories.find((c) => c.id === "domain")!]);
  const services = $derived(directory.catalog.services);
  const serviceIcon = (id: string | null) => {
    const service = services.find((s) => s.id === id);
    return (service && directory.catalog.icons[service.icon]) || bundledServiceIcon(id);
  };
  let loading = $state(true),
    busy = $state(false),
    error = $state(""),
    notice = $state("");
  let currency = $state("CNY"),
    period = $state("month"),
    category = $state("all"),
    search = $state(""),
    filter = $state("all");
  let sortBy = $state("added"), sortOrder = $state("desc"), paymentFilter = $state("all");
  const viewKey = "subbit.asset-view";
  function restoreView() {
    try {
      const saved = JSON.parse(localStorage.getItem(viewKey) ?? "{}");
      sortBy = saved.sortBy === "price" ? "price" : "added";
      sortOrder = saved.sortOrder === "asc" ? "asc" : "desc";
      paymentFilter = typeof saved.payment === "string" ? saved.payment : "all";
      category = typeof saved.category === "string" ? saved.category : "all";
      filter = ["recurring", "lifetime"].includes(saved.kind) ? saved.kind : "all";
    } catch { /* Invalid saved preferences do not prevent opening assets. */ }
  }
  restoreView();
  $effect(() => {
    const saved = { sortBy, sortOrder, payment: paymentFilter, category, kind: filter };
    try { localStorage.setItem(viewKey, JSON.stringify(saved)); } catch { /* Storage can be unavailable in browser previews. */ }
  });
  let offline = $state(false);
  let version = $state(appInfo.version);
  let update: Update | null = $state(null);
  let showUpdate = $state(false);
  let updateDialog: HTMLDialogElement;
  let updateState = $state<"idle" | "checking" | "available" | "current" | "downloading" | "installed" | "error">("idle");
  let updateError = $state(""), downloaded = $state(0), downloadTotal = $state(0);
  let systemDark = $state(window.matchMedia("(prefers-color-scheme: dark)").matches);
  let modal = $state<"" | "picker" | "editor" | "settings">("");
  let pickerSearch = $state(""),
    pickerCategory = $state("all");
  let record: Subscription = $state(blank());
  let planPresetId = $state("");
  let autoPrice = $state(false);
  let iconRequest = 0;
  let fetchedIcon: string | null = null;
  let fetchedDomain = "";
  let editRevision = 0;
  let prefs: Settings = $state(structuredClone(defaultSettings));
  let settingsRevision = 0,
    settingsTab = $state("general"),
    autoStart = $state(false),
    rateCode = $state("USD"),
    rateValue = $state("");
  let preview: Database | null = $state(null);
  let fileInfo: {data: number | null; backup: number | null} | null = $state(null);
  let fileInfoError = $state(false);
  let inputDialog: HTMLDialogElement;
  const activeLanguage = $derived.by(() => modal === "settings" ? prefs.language : snap?.database.settings.language ?? "system");
  const en = $derived(activeLanguage === "en" || (activeLanguage === "system" && !navigator.language.toLowerCase().startsWith("zh")));
  const activeTheme = $derived.by(() => modal === "settings" ? prefs.theme : snap?.database.settings.theme ?? "system");
  const dark = $derived(activeTheme === "dark" || (activeTheme === "system" && systemDark));
  const tr = (zh: string, english: string) => (en ? english : zh);
  const spendingLevelName = $derived.by(() => snap?.level ? localizeLevelName(snap.level, en) : tr("尚未达到等级", "No level yet"));
  const labels = $derived({
    addAsset: tr("新增资产", "Add asset"),
    personalCost: tr("实际花销", "Personal cost"),
    billedCost: tr("账单花销", "Billed cost"),
    locateData: tr("选择已有数据文件", "Choose existing data file"),
    changeIcon: tr("更换图标", "Change icon"),
    openApp: tr("打开 Subbit", "Open Subbit"),
    ratesUpdated: tr("汇率已更新", "Exchange rates updated"),
  });
  const entries = $derived.by(() => snap?.database.subscriptions ?? []);
  const assetCategories = $derived([...categories, ...Array.from(new Set(entries.map((s) => s.category)))
    .filter((id) => !categories.some((c) => c.id === id)).map((id) => ({ id, zh: id, en: id, icon: "" }))]);
  const filtered = $derived.by(() => {
    const rows = new Map(snap?.summary.rows.map((r) => [r.id, r.actual]));
    return entries.filter(
      (s) =>
        (category === "all" || s.category === category) &&
        (filter === "all" || s.kind === filter) &&
        (paymentFilter === "all" || s.paymentMethod === paymentFilter) &&
        `${s.name} ${s.plan} ${s.registrar ?? ""} ${s.notes}`
          .toLowerCase()
          .includes(search.toLowerCase()),
    ).sort((a, b) => {
      if (sortBy === "price") {
        const av = rows.get(a.id), bv = rows.get(b.id);
        if (av == null || bv == null) return av == null ? (bv == null ? 0 : 1) : -1;
        const diff = Number(av) - Number(bv);
        if (diff) return sortOrder === "asc" ? diff : -diff;
      }
      const diff = entries.indexOf(a) - entries.indexOf(b);
      return sortOrder === "asc" ? diff : -diff;
    });
  });
  const selectedService = $derived(
    services.find((s) => s.id === record.serviceId),
  );
  const selectedPlan = $derived(selectedService?.plans.find((p) => p.id === planPresetId));
  const quote = $derived.by(() => presetQuote(selectedPlan, record, snap?.database));
  const pickerServices = $derived(
    services.filter(
      (s) =>
        (pickerCategory === "all" || s.category === pickerCategory) &&
        `${s.name} ${s.aliases.join(" ")}`
          .toLowerCase()
          .includes(pickerSearch.toLowerCase()),
    ),
  );
  const customServices = $derived(
    entries.filter((s) =>
      !s.serviceId && s.category !== "domain" &&
      (pickerCategory === "all" || s.category === pickerCategory) &&
      s.name.toLowerCase().includes(pickerSearch.toLowerCase()),
    ).filter((s, i, all) => all.findIndex((x) => x.name === s.name) === i),
  );
  const unitLabel = (unit: string) =>
    ({
      hour: tr("小时", "hour"),
      month: tr("月", "month"),
      year: tr("年", "year"),
      day: tr("天", "day"),
      week: tr("周", "week"),
    })[unit] ?? unit;
  const categoryName = (id: string) => {
    const c = categories.find((c) => c.id === id);
    return c ? (en ? c.en : c.zh) : id;
  };
  const paymentName = (id: string) => {
    const p = paymentMethods.find((p) => p[0] === id);
    return p ? p[en ? 2 : 1] : id;
  };
  function today() {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }
  function blank(): Subscription {
    return {
      id: crypto.randomUUID(),
      name: "",
      serviceId: null,
      category: "other",
      plan: "",
      icon: null,
      amount: "",
      currency: "CNY",
      kind: "recurring",
      cycleUnit: "month",
      cycleCount: 1,
      purchasedAt: today(),
      expiresAt: null,
      autoRenew: true,
      active: true,
      reimbursement: "none",
      reimbursementValue: "0",
      paymentMethod: "",
      notes: "",
      fixedUsdRate: null,
    };
  }
  function money(value: string | null | undefined, code = currency) {
    if (value == null) return "—";
    const n = Number(value);
    if (
      snap?.database.settings.currencySymbol !== false &&
      currencies.indexOf(code) < 26
    )
      return new Intl.NumberFormat(en ? "en-US" : "zh-CN", {
        style: "currency",
        currency: code,
        minimumFractionDigits: period === "hour" ? 4 : 2,
        maximumFractionDigits: period === "hour" ? 4 : 2,
      }).format(n);
    return `${code} ${n.toLocaleString(en ? "en-US" : "zh-CN", { minimumFractionDigits: period === "hour" ? 4 : 2, maximumFractionDigits: period === "hour" ? 4 : 2 })}`;
  }
  async function call<T>(
    command: string,
    args?: Record<string, unknown>,
  ): Promise<T> {
    if (!isTauri())
      throw new Error(
        tr(
          "请在桌面应用中使用此功能（npm run tauri dev）",
          "Open the desktop app to use this feature (npm run tauri dev)",
        ),
      );
    return invoke<T>(command, args);
  }
  function applyDirectory(next: CatalogState) {
    if (modal === "editor" || modal === "picker") pendingDirectory = next;
    else directory = next;
  }
  async function loadDirectory() {
    if (isTauri()) applyDirectory(await call<CatalogState>("load_catalog"));
  }
  async function refreshDirectory(force = false) {
    if (!isTauri() || catalogBusy || offline) return;
    catalogBusy = true;
    catalogError = false;
    try {
      applyDirectory(await call<CatalogState>("refresh_catalog", { force }));
      if (force) notice = tr("目录已更新", "Catalog updated");
    } catch {
      if (force) catalogError = true;
    } finally { catalogBusy = false; }
  }
  $effect(() => {
    if (pendingDirectory && modal !== "editor" && modal !== "picker") {
      directory = pendingDirectory;
      pendingDirectory = null;
    }
  });
  const reload = createReloadQueue(loadSnapshot);
  async function loadSnapshot(initial: boolean) {
    try {
      if (!isTauri()) {
        snap = {
          database: {
            schemaVersion: 1,
            revision: 0,
            subscriptions: [],
            settings: structuredClone(defaultSettings),
            rates: {},
          },
          key: null,
          path: "Documents/Subbit/subscriptions.json",
          executable: "subbit",
          level: null,
          summary: {
            bill: "0",
            actual: "0",
            reimbursed: "0",
            currency,
            period,
            missingRates: [],
            rows: [],
            activeCount: 0,
          },
        };
        return;
      }
      offline = await call<boolean>("offline_mode");
      const requestedCurrency = currency, requestedPeriod = period;
      const next = await call<Snapshot | null>(
        "snapshot",
        initial ? {} : { currency: requestedCurrency, period: requestedPeriod, known: snap?.key },
      );
      if (!next || (!initial && (currency !== requestedCurrency || period !== requestedPeriod))) return;
      snap = next;
      if (initial) {
        currency = next.database.settings.currency;
        period = next.database.settings.period;
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  async function action(fn: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = "";
    notice = "";
    try {
      await fn();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function rates(force = false) {
    await action(async () => {
      const warnings = await call<string[]>("refresh_rates", { force });
      await reload();
      if (warnings.length)
        notice = tr(
          "部分汇率更新失败，保留已有缓存。",
          "Some exchange rates could not be updated. Cached rates were retained.",
        );
      else if (force) notice = labels.ratesUpdated;
    });
  }
  async function picker() {
    if (panel) {
      await call("open_main", {action:"add"});
      return;
    }
    await reload();
    editRevision = snap?.database.revision ?? 0;
    pickerSearch = "";
    pickerCategory = category === "all" ? "all" : category;
    modal = "picker";
  }
  function choose(service?: Service, custom?: Subscription) {
    iconRequest++; fetchedIcon = null; fetchedDomain = "";
    record = blank();
    record.currency = en ? "USD" : "CNY";
    if (service) {
      record.name = service.name;
      record.serviceId = service.id;
      record.category = service.category;
      planPresetId = service.plans.find((p) => !p.retired)?.id ?? "";
      applyPlan(service.plans.find((p) => p.id === planPresetId));
    } else if (custom) {
      planPresetId = "";
      autoPrice = false;
      record.name = custom.name;
      record.icon = custom.icon;
      record.category = custom.category;
      record.plan = custom.plan;
    } else {
      planPresetId = "";
      autoPrice = false;
      record.name = pickerSearch;
      record.category = pickerCategory === "all" ? "other" : pickerCategory;
    }
    if (record.category === "domain") { record.cycleUnit = "year"; record.registrar = ""; }
    updateExpiry();
    modal = "editor";
  }
  function edit(s: Subscription) {
    iconRequest++; fetchedIcon = null; fetchedDomain = "";
    if (panel) {
      void call("open_main");
      return;
    }
    record = structuredClone($state.snapshot(s));
    planPresetId = services.find((service) => service.id === s.serviceId)
      ?.plans.find((p) => p.name === s.plan)?.id ?? "";
    autoPrice = false;
    if (record.kind === "recurring" && !record.expiresAt) updateExpiry();
    editRevision = snap!.database.revision;
    modal = "editor";
  }
  function changeCategory() {
    if (record.category === "domain") {
      record.kind = "recurring"; record.cycleUnit = "year"; record.cycleCount = 1;
      record.serviceId = null; planPresetId = ""; autoPrice = false;
      record.registrar ??= "";
      setKind();
    }
  }
  function setKind() {
    if (record.kind === "lifetime") {
      record.autoRenew = false;
      record.expiresAt = null;
    } else {
      record.autoRenew = true;
      updateExpiry();
    }
  }
  function updateExpiry() {
    record.expiresAt = record.kind === "recurring" ? nextCycle(record.purchasedAt, record.cycleUnit, record.cycleCount) : null;
  }
  function applyPlan(plan: Service["plans"][number] | undefined) {
    autoPrice = !!plan;
    record.plan = plan?.name ?? "";
    if (plan) {
      record.kind = plan.kind;
      record.cycleUnit = plan.prices?.[0]?.cycleUnit ?? plan.cycleUnit ?? "month";
      record.cycleCount = plan.prices?.[0]?.cycleCount ?? 1;
      setKind();
    }
    const nextQuote = presetQuote(plan, record, snap?.database);
    record.amount = nextQuote.status === "ready" ? nextQuote.amount : "";
  }
  async function submit() {
    await action(async () => {
      const payload = structuredClone($state.snapshot(record));
      if (!payload.icon && payload.serviceId && !entries.some((s) => s.id === payload.id)) {
        const url = serviceIcon(payload.serviceId);
        if (url) {
          const image = new Image();
          image.src = url;
          await image.decode();
          const canvas = document.createElement("canvas");
          canvas.width = canvas.height = 96;
          const size = Math.max(image.naturalWidth, image.naturalHeight);
          canvas.getContext("2d")!.drawImage(image, (96 - image.naturalWidth / size * 96) / 2,
            (96 - image.naturalHeight / size * 96) / 2, image.naturalWidth / size * 96, image.naturalHeight / size * 96);
          payload.icon = canvas.toDataURL("image/png");
        }
      }
      payload.name = payload.name.trim();
      payload.registrar = payload.registrar?.trim() || undefined;
      payload.expiresAt = payload.expiresAt || null;
      // Keep the original start-date anchor for month ends and leap years.
      if (payload.autoRenew && payload.expiresAt === nextCycle(payload.purchasedAt, payload.cycleUnit, payload.cycleCount)) payload.expiresAt = null;
      payload.fixedUsdRate = payload.fixedUsdRate || null;
      if (payload.kind === "lifetime") {
        payload.autoRenew = false;
        payload.expiresAt = null;
      }
      await call("save_subscription", {
        record: payload,
        revision: editRevision,
      });
      modal = "";
      await reload();
      notice = tr("已保存到本地", "Saved locally");
    });
  }
  async function remove() {
    await action(async () => {
      if (
        await confirm(
          tr(`删除「${record.name}」？`, `Delete “${record.name}”?`),
          { title: "Subbit", kind: "warning" },
        )
      ) {
        await call("delete_subscription", {
          id: record.id,
          revision: editRevision,
        });
        modal = "";
        await reload();
      }
    });
  }
  async function fetchDomainIcon() {
    if (record.category !== "domain" || offline || !isTauri()) return;
    const domain = record.name.trim();
    if (fetchedIcon && fetchedDomain !== domain && record.icon === fetchedIcon) record.icon = null;
    if (record.icon || !domain.includes(".") || /[/:@?#\s]/.test(domain)) return;
    const request = ++iconRequest, id = record.id;
    const current = () => request === iconRequest && modal === "editor" && record.id === id && record.category === "domain" && record.name.trim() === domain && !record.icon && !offline;
    let links: string[] = [];
    try {
      const page = await call<{url: string; html: string}>("website_icon", { domain, iconUrl: null });
      // Parse an inert document; never insert website HTML into the application.
      const template = document.createElement("template");
      template.innerHTML = page.html;
      const doc = template.content;
      links = Array.from(doc.querySelectorAll<HTMLLinkElement>('link[rel]'))
        .filter((link) => link.rel.toLowerCase().split(/\s+/).some((rel) => rel === "icon" || rel === "apple-touch-icon"))
        .flatMap((link) => { try { const url = new URL(link.getAttribute("href") ?? "", page.url); return url.protocol === "https:" ? [url.href] : []; } catch { return []; } }).slice(0, 3);
    } catch { /* Root favicon remains a fallback when the page cannot be parsed. */ }
    for (const iconUrl of [...new Set([...links, `https://${domain}/favicon.ico`])]) {
      if (!current()) return;
      try {
        const { data } = await call<{data: string}>("website_icon", { domain, iconUrl });
        if (!current()) return;
        const image = new Image(); image.src = data; await image.decode();
        if (!current()) return;
        const canvas = document.createElement("canvas"); canvas.width = canvas.height = 96;
        const size = Math.max(image.naturalWidth, image.naturalHeight);
        if (!size) continue;
        canvas.getContext("2d")!.drawImage(image, (96 - image.naturalWidth / size * 96) / 2, (96 - image.naturalHeight / size * 96) / 2, image.naturalWidth / size * 96, image.naturalHeight / size * 96);
        fetchedIcon = canvas.toDataURL("image/png"); fetchedDomain = domain; record.icon = fetchedIcon;
        return;
      } catch { /* Keep the globe when a site does not expose a supported favicon. */ }
    }
  }
  async function uploadIcon(file?: File) {
    if (!file) return;
    iconRequest++; fetchedIcon = null; fetchedDomain = "";
    await action(async () => {
      if (!["image/png", "image/jpeg", "image/webp"].includes(file.type))
        throw new Error(
          tr("请选择 PNG、JPG 或 WebP", "Choose PNG, JPG or WebP"),
        );
      if (file.size > 10_000_000)
        throw new Error(tr("图片请小于 10 MB", "Choose an image under 10 MB"));
      const bitmap = await createImageBitmap(file);
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 96;
      const ctx = canvas.getContext("2d")!;
      const size = Math.max(bitmap.width, bitmap.height);
      ctx.drawImage(
        bitmap,
        (96 - (bitmap.width / size) * 96) / 2,
        (96 - (bitmap.height / size) * 96) / 2,
        (bitmap.width / size) * 96,
        (bitmap.height / size) * 96,
      );
      record.icon = canvas.toDataURL("image/png");
      bitmap.close();
    });
  }
  async function settings() {
    if (panel) {
      await call("open_main", {action:"settings"});
      return;
    }
    await reload();
    if (!snap) return;
    prefs = structuredClone($state.snapshot(snap.database.settings));
    settingsRevision = snap.database.revision;
    if (prefs.levels.length === 1 && prefs.levels[0].above === "100" && prefs.levels[0].name === "数字土豪") prefs.levels = defaultLevels(en);
    modal = "settings";
    settingsTab = "general";
    fileInfo = null; fileInfoError = false;
    if (isTauri()) try { fileInfo = await call("data_file_info"); } catch { fileInfoError = true; }
    if (isTauri())
      try {
        autoStart = await isEnabled();
      } catch (e) {
        error = String(e);
      }
  }
  async function toggleOffline() {
    if (updateState === "downloading") { error = tr("请等待应用更新完成", "Wait for the application update to finish"); return; }
    const enabled = !offline;
    if (enabled && !await confirm(tr(
      "开启后将停止联网更新汇率、服务目录、网站图标和应用版本。\n\n本地汇率可能已过时，换算结果可能与实时价格有差异；服务目录可能不包含最新服务、套餐和参考价。已保存的资产不受影响。\n\n你仍可在设置中填写固定汇率，或通过 Agent 使用 MCP 导入汇率和服务目录。",
      "Subbit will stop updating exchange rates, the service catalog, website icons and the application online.\n\nLocal rates may be outdated and conversions may differ from current prices. The catalog may lack recent services, plans and reference prices. Saved assets are unaffected.\n\nYou can still set fixed rates in Settings or use an Agent to import rates and catalogs through MCP."
    ), { title: tr("开启离线模式", "Enable offline mode"), kind: "warning", okLabel: tr("开启离线模式", "Enable offline mode"), cancelLabel: tr("取消", "Cancel") })) return;
    await action(async () => {
      await call("set_offline_mode", { enabled });
      offline = enabled;
      if (enabled) { await update?.close(); update = null; updateState = "idle"; showUpdate = false; }
    });
  }
  async function onlineAllowed() {
    if (offline) { error = tr("请先关闭离线模式", "Turn off offline mode first"); return false; }
    try { if (isTauri()) await call("require_online"); return true; }
    catch (e) { error = String(e); return false; }
  }
  async function savePrefs() {
    await action(async () => {
      await call("save_settings", {
        settings: $state.snapshot(prefs),
        revision: settingsRevision,
      });
      modal = "";
      await reload(true);
    });
  }
  async function toggleStartup() {
    await action(async () => {
      if (autoStart) await disable();
      else await enable();
      autoStart = await isEnabled();
    });
  }
  function addRate() {
    const n = Number(rateValue);
    if (!Number.isFinite(n) || n <= 0) {
      error = tr("请输入正数汇率", "Enter a positive exchange rate");
      return;
    }
    prefs.fixedRates[rateCode] = rateValue;
    rateValue = "";
  }
  async function exportData() {
    await action(async () => {
      const path = await save({
        defaultPath: "subbit-backup.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (path) {
        await call("export_data", { path });
        notice = tr("已导出完整备份", "Full backup exported");
      }
    });
  }
  async function importData() {
    await action(async () => {
      const path = await open({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (path) {
        preview = await call<Database>("preview_file", { path });
      }
    });
  }
  async function replaceData() {
    await action(async () => {
      await call("replace_data", {
        database: $state.snapshot(preview),
        revision: settingsRevision,
      });
      preview = null;
      modal = "";
      await reload(true);
      notice = tr(
        "已导入，原数据已备份",
        "Imported. Previous data was backed up.",
      );
    });
  }
  async function moveData() {
    await action(async () => {
      const directory = await open({ directory: true, multiple: false });
      if (directory) {
        await call("move_data", { directory });
        await reload();
        notice = tr(
          "数据位置已切换，原文件保留",
          "Data location changed. Original file retained.",
        );
      }
    });
  }
  async function locateData() {
    await action(async () => {
      const path = await open({
        multiple: false,
        defaultPath: snap?.path,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (path) {
        await call("locate_data", { path });
        modal = "";
        await reload(true);
      }
    });
  }
  const mcpConfig = $derived.by(() =>
    JSON.stringify(
      {
        mcpServers: {
          subbit: { command: snap?.executable ?? "subbit", args: ["--mcp"] },
        },
      },
      null,
      2,
    ),
  );
  async function copyMcp() {
    await action(async () => {
      await navigator.clipboard.writeText(mcpConfig);
      notice = tr("连接配置已复制", "Connection config copied");
    });
  }
  $effect(() => {
    if (autoPrice) record.amount = quote.status === "ready" ? quote.amount : "";
  });
  async function external(url?: string) {
    if (!url || !await onlineAllowed()) return;
    try { if (isTauri()) await openUrl(url); else window.open(url, "_blank", "noopener,noreferrer"); }
    catch (e) { error = String(e); }
  }
  async function checkUpdate() {
    if (["checking", "downloading"].includes(updateState) || !await onlineAllowed()) return;
    updateState = "checking"; updateError = "";
    try {
      if (!isTauri()) throw new Error(tr("请在桌面应用中检查更新", "Check for updates in the desktop app"));
      await update?.close(); update = null;
      update = await check({ timeout: 30000 });
      if (!await onlineAllowed()) { await update?.close(); update = null; updateState = "idle"; return; }
      updateState = update ? "available" : "current";
      showUpdate = !!update;
    } catch (e) { updateState = "error"; updateError = String(e); }
  }
  async function installUpdate() {
    if (!update || updateState !== "available" || !await onlineAllowed()) return;
    updateState = "downloading"; downloaded = 0; downloadTotal = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") downloadTotal = event.data.contentLength ?? 0;
        if (event.event === "Progress") downloaded += event.data.chunkLength;
      });
      updateState = "installed";
    } catch (e) { updateState = "error"; updateError = String(e); }
  }
  $effect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    document.documentElement.lang = en ? "en" : "zh-CN";
    if (isTauri()) void call("apply_appearance", { theme: activeTheme, dark }).catch((e) => error = String(e));
  });
  $effect(() => {
    if (modal !== "settings") preview = null;
  });
  $effect(() => {
    if (showUpdate && updateDialog && !updateDialog.open) updateDialog.showModal();
    if (!showUpdate && updateDialog?.open) updateDialog.close();
  });
  $effect(() => {
    if (modal && inputDialog && !inputDialog.open) inputDialog.showModal();
    if (!modal && inputDialog?.open) inputDialog.close();
  });
  onMount(() => {
    let cleanup: (() => void)[] = [];
    let disposed = false;
    const storageChanged = (event: StorageEvent) => { if (event.key === viewKey) restoreView(); };
    window.addEventListener("storage", storageChanged);
    cleanup.push(() => window.removeEventListener("storage", storageChanged));
    const appearance = window.matchMedia("(prefers-color-scheme: dark)");
    const changedAppearance = () => systemDark = appearance.matches;
    appearance.addEventListener("change", changedAppearance);
    if (isTauri()) void getVersion().then((v) => version = v);
    void (async () => {
      await loadDirectory().catch(() => {});
      await reload(true);
      if (new URLSearchParams(location.search).has("settings")) await settings();
      if (new URLSearchParams(location.search).has("about")) { await settings(); settingsTab = "about"; }
      if (isTauri()) {
        if (new URLSearchParams(location.search).has("add")) await picker();
        for (const name of ["network-changed", "data-changed", "open-settings", "open-add", "open-about"]) {
          const unlisten = await listen(name, () => {
            if (name === "network-changed") void call<boolean>("offline_mode").then((value) => offline = value);
            else if (name === "open-about") void settings().then(() => settingsTab = "about");
            else if (name === "open-settings") void settings();
            else if (name === "open-add") void picker();
            else if (!modal && !busy) void reload();
          });
          if (disposed) unlisten();
          else cleanup.push(unlisten);
        }
        if (!modal && !offline) void rates();
        void refreshDirectory();
      }
    })();
    const focus = () => {
      if (!modal && !busy) {
        void reload();
        void refreshDirectory();
      }
    };
    window.addEventListener("focus", focus);
    const interval = setInterval(() => {
      if (document.hasFocus() && !modal && !busy) void reload();
    }, 15000);
    const catalogInterval = setInterval(() => { if (document.hasFocus()) void refreshDirectory(); }, 15 * 60 * 1000);
    return () => {
      disposed = true;
      cleanup.forEach((f) => f());
      appearance.removeEventListener("change", changedAppearance);
      clearInterval(interval);
      clearInterval(catalogInterval);
      window.removeEventListener("focus", focus);
    };
  });
</script>

<div class:panel class="shell">
  <main>
    <header class="page-header">
      <div class="page-title"><h1>Subbit</h1>{#if snap?.database.settings.showLevel}<span class="level" title={spendingLevelName}>{spendingLevelName}</span>{/if}</div>
      <div class="header-actions">
        {#if !panel}<button class="icon-button" aria-label={tr("设置", "Settings")} title={tr("设置", "Settings")} onclick={settings}><Icon name="settings"/></button>{/if}
        <button class="primary" onclick={picker}><Icon name="plus"/>{labels.addAsset}</button>
      </div>
    </header>
    <section class="overview" aria-label={tr("成本总览", "Cost overview")}>
      <div class="overview-heading">
        <div class="controls"><select aria-label={tr("显示币种", "Display currency")} bind:value={currency} onchange={() => reload()}>{#each currencies.slice(0, 26) as c}<option>{c}</option>{/each}</select>
          <div class="segmented">{#each ["hour", "month", "year"] as unit}<button class:active={period === unit} aria-pressed={period === unit} onclick={() => {period = unit; void reload();}}>{tr("每", "/")}{unitLabel(unit)}</button>{/each}</div>
        </div>
      </div>
      <div class="summary-grid">
        <div class="personal"><div class="card-label">{labels.personalCost}</div><div class="amount">{money(snap?.summary.actual)}<small>/ {unitLabel(period)}</small></div></div>
        <div class="secondary-costs"><div class="bill"><div class="card-label">{labels.billedCost}</div><div class="secondary-amount">{money(snap?.summary.bill)}</div></div><div class="reimbursed"><div class="card-label">{tr("报销抵扣", "Reimbursed")}</div><div class="secondary-amount">{money(snap?.summary.reimbursed)}</div></div></div>
      </div>
    </section>
    {#if snap?.summary.missingRates.length}<div class="banner warning">{tr("部分币种缺少汇率，总额暂不可用。", "Some exchange rates are missing.")}<button onclick={settings}>{tr("设置", "Settings")}</button></div>{/if}
    {#if !isTauri()}<div class="banner">{tr("界面预览 · 本地数据功能请在桌面应用中使用", "Interface preview · Local data features require the desktop app")}</div>{/if}
    <section class="assets-section">
      <div class="list-toolbar">
        <div class="filter-tabs" aria-label={tr("资产类型", "Asset type")}>
          {#each [["all", "全部", "All"], ["recurring", "订阅", "Subscriptions"], ["lifetime", "买断", "Lifetime"]] as item}
            <button class:active={filter === item[0]} aria-pressed={filter === item[0]} onclick={() => filter = item[0]}>{item[en ? 2 : 1]}</button>
          {/each}
        </div>
        <div class="list-filters">
          <select class="category-filter" aria-label={tr("筛选分类", "Filter category")} bind:value={category}>
            <option value="all">{tr("全部分类", "All categories")}</option>
            {#each assetCategories as c}<option value={c.id}>{en ? c.en : c.zh}</option>{/each}
          </select>
          <label class="search-field"><Icon name="search" size={16}/><input class="search" aria-label={tr("搜索资产", "Search assets")} placeholder={tr("搜索资产", "Search assets")} bind:value={search}/></label>
        </div>
      </div>
      <div class="list-head">
        <div class="sort-controls"><span>{tr("服务 / 套餐", "Service / Plan")}</span>
          <select aria-label={tr("支付方式", "Payment method")} bind:value={paymentFilter}><option value="all">{tr("全部支付方式", "All payments")}</option><option value="">{tr("未设置", "Not set")}</option>{#each paymentMethods as method}<option value={method[0]}>{method[en ? 2 : 1]}</option>{/each}</select>
          <select aria-label={tr("排序依据", "Sort by")} bind:value={sortBy}><option value="added">{tr("添加时间", "Date added")}</option><option value="price">{tr("价格", "Price")}</option></select>
          <select aria-label={tr("排序方向", "Sort order")} bind:value={sortOrder}><option value="desc">{tr("降序", "Descending")}</option><option value="asc">{tr("升序", "Ascending")}</option></select>
        </div><span>{labels.personalCost} / {unitLabel(period)}</span>
      </div>
      {#if loading}<div class="empty-state">
          {tr("正在读取本地数据…", "Loading your local data…")}
        </div>
      {:else if !filtered.length}<div class="empty-state">

          <h2>
            {entries.length
              ? tr("没有匹配的资产", "No matching assets")
              : tr("还没有数字资产", "No assets yet")}
          </h2>
          {#if entries.length}<button onclick={() => { category = "all"; filter = "all"; paymentFilter = "all"; search = ""; }}>{tr("清除筛选", "Clear filters")}</button>
          {:else}<button class="primary" onclick={picker}><Icon name="plus"/>{labels.addAsset}</button>{/if}
        </div>
      {:else}<div class="asset-list">
          {#each filtered as item (item.id)}{@const cost =
              snap?.summary.rows.find((r) => r.id === item.id)}<button
              class:inactive={!cost?.current}
              class="asset-row"
              onclick={() => edit(item)}
            >
              <span class="service-icon"
                >{#if item.icon || serviceIcon(item.serviceId)}<img
                    src={item.icon || serviceIcon(item.serviceId)}
                    alt=""
                  />{:else if item.category === "domain"}<Icon name="globe" size={28}/>{:else}<span
                    >{item.category === "vpn"
                      ? "⌁"
                      : item.name.slice(0, 1)}</span
                  >{/if}</span
              >
              <span class="asset-info"
                ><strong
                  ><span class="asset-name" title={item.name}>{item.name}</span><span class="tag"
                    >{item.kind === "lifetime"
                      ? tr("买断", "Lifetime")
                      : categoryName(item.category)}</span
                  ></strong
                ><span class="asset-meta" title={`${item.category === "domain" ? item.registrar ?? "" : item.plan || tr("自定义套餐", "Custom plan")} · ${item.currency} ${item.amount}${item.kind === "recurring" ? ` / ${item.cycleCount} ${unitLabel(item.cycleUnit)}` : ""}`}
                  >{item.category === "domain" ? (item.registrar ? `${item.registrar} · ` : "") : `${item.plan || tr("自定义套餐", "Custom plan")} · `}{item.currency}
                  {item.amount}{item.kind === "recurring"
                    ? ` / ${item.cycleCount > 1 ? item.cycleCount : ""}${unitLabel(item.cycleUnit)}`
                    : ""}</span
                ><span class="asset-detail"
                  >{#if !item.active}{tr(
                      "已停用",
                      "Inactive",
                    )}{:else if !cost?.current}{item.purchasedAt > today()
                      ? tr("尚未开始", "Not started")
                      : tr(
                          "已到期",
                          "Expired",
                        )}{:else if item.kind === "lifetime"}{tr(
                      `已拥有 ${cost.elapsedDays} 天`,
                      `Owned ${cost.elapsedDays} days`,
                    )}{:else}{cost?.dueDate}
                    {item.autoRenew
                      ? tr("续期", "renews")
                      : tr(
                          "到期 · 不续期",
                          "expires · no renewal",
                        )}{/if}{#if item.paymentMethod}
                    · {paymentName(item.paymentMethod)}{/if}</span
                ></span
              >
              <span class="asset-cost"
                ><strong>{money(cost?.actual)}</strong>{#if item.reimbursement === "full" || (item.reimbursement === "percent" && Number(item.reimbursementValue) === 100) || (item.reimbursement === "fixed" && Number(item.amount) > 0 && Number(item.reimbursementValue) === Number(item.amount))}<small>{tr("全额报销", "Fully reimbursed")}</small>{:else if ["fixed", "percent"].includes(item.reimbursement) && Number(item.reimbursementValue) > 0}<small>{tr("部分报销", "Partly reimbursed")}</small>{/if}</span
              ><span class="chevron"><Icon name="chevron" size={16}/></span>
            </button>{/each}
        </div>{/if}
    </section>
    <footer><span>{filtered.length} / {entries.length} {tr("项资产", "assets")}</span>{#if panel}<button onclick={() => call("open_main")}>{labels.openApp}<Icon name="arrow" size={16}/></button>{/if}</footer>
  </main>
</div>

<dialog
  bind:this={inputDialog}
  class:settings-modal={modal === "settings"}
  aria-label={modal === "settings" ? tr("设置", "Settings") : labels.addAsset}
  oncancel={() => {
    modal = "";
    preview = null;
  }}
  onclose={() => (modal = "")}
>
  {#if modal === "picker"}<div class="dialog-header">
      <div>
        <h2>{labels.addAsset}</h2>
      </div>
      <button
        class="icon-button"
        aria-label={tr("关闭", "Close")}
        onclick={() => (modal = "")}><Icon name="close" size={16}/></button
      >
    </div>
    <div class="dialog-content">
      <input
        class="picker-search"
        placeholder={tr(
          "搜索服务，比如 Apple Music、OpenAI…",
          "Search Apple Music, OpenAI…",
        )}
        aria-label={tr("搜索服务", "Search services")}
        bind:value={pickerSearch}
      />
      <div class="category-pills">
        <button
          class:active={pickerCategory === "all"}
          onclick={() => (pickerCategory = "all")}>{tr("全部", "All")}</button
        >{#each categories as c}<button
            class:active={pickerCategory === c.id}
            onclick={() => (pickerCategory = c.id)}>{en ? c.en : c.zh}</button
          >{/each}
      </div>
      <div class="service-grid">
        <button class="service-choice custom" onclick={() => choose()}
          ><span class="service-icon"><Icon name="plus" size={22}/></span><strong
            >{pickerSearch
              ? tr(`创建「${pickerSearch}」`, `Create “${pickerSearch}”`)
              : tr("自定义服务", "Custom service")}</strong
          ><small
            >{tr(
              "任何订阅或买断软件",
              "Any subscription or lifetime app",
            )}</small
          ></button
        >{#each pickerServices as s}<button
            class="service-choice"
            onclick={() => choose(s)}
            ><span class="service-icon"
              ><img src={serviceIcon(s.id)} alt="" /></span
            ><strong>{s.name}</strong><small>{categoryName(s.category)}</small
            ></button
          >{/each}{#each customServices as s}<button
            class="service-choice"
            onclick={() => choose(undefined, s)}
            ><span class="service-icon"
              >{#if s.icon}<img src={s.icon} alt="" />{:else}{s.name.slice(
                  0,
                  1,
                )}{/if}</span
            ><strong>{s.name}</strong><small
              >{tr("我的自定义服务", "My custom service")}</small
            ></button
          >{/each}
      </div>
    </div>
  {:else if modal === "editor"}<form
      onsubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <div class="dialog-header">
        <div>
          <h2>
            {entries.some((s) => s.id === record.id)
              ? tr("编辑资产", "Edit asset")
              : labels.addAsset}
          </h2>
        </div>
        <button
          type="button"
          class="icon-button"
          aria-label={tr("关闭", "Close")}
          onclick={() => (modal = "")}><Icon name="close" size={16}/></button
        >
      </div>
      <div class="dialog-content form-body">
        <div class="identity">
          <label
            class="upload-icon"
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => {
              e.preventDefault();
              void uploadIcon(e.dataTransfer?.files[0]);
            }}
            ><span class="service-icon"
              >{#if record.icon || serviceIcon(record.serviceId)}<img
                  src={record.icon || serviceIcon(record.serviceId)}
                  alt=""
                />{:else if record.category === "domain"}<Icon name="globe" size={28}/>{:else}{record.category === "vpn"
                  ? "⌁"
                  : record.name.slice(0, 1) || "＋"}{/if}</span
            ><input
              type="file"
              accept="image/png,image/jpeg,image/webp"
              aria-label={labels.changeIcon}
              onchange={(e) => uploadIcon(e.currentTarget.files?.[0])}
            /><small>{labels.changeIcon}</small></label
          ><label class="grow"
            >{record.category === "domain" ? tr("域名", "Domain") : tr("名称", "Name")}<input
              required
              maxlength="100"
              bind:value={record.name}
              onblur={() => void fetchDomainIcon()}
              placeholder={record.category === "domain" ? "example.com" : tr("服务或软件名称", "Service or software name")}
            /></label
          >
        </div>
        <div class="form-grid">
          <label
            >{tr("分类", "Category")}<select bind:value={record.category} onchange={changeCategory}
              >{#if !categories.some((c) => c.id === record.category)}<option value={record.category}>{record.category}</option>{/if}{#each categories as c}<option value={c.id}
                  >{en ? c.en : c.zh}</option
                >{/each}</select
            ></label
          ><div class="plan-field">
            {#if record.category === "domain"}<label>{tr("域名管理商", "Registrar")}<input bind:value={record.registrar} maxlength="200" list="registrars" placeholder={tr("输入管理商名称", "Enter registrar name")}/><datalist id="registrars">{#each [...new Set(entries.filter((s) => s.category === "domain" && s.registrar).map((s) => s.registrar!))] as registrar}<option value={registrar}></option>{/each}</datalist></label>{:else}
            {#if selectedService}<label>
              {tr("套餐", "Plan")}
              <select bind:value={planPresetId} onchange={(e) => { planPresetId = e.currentTarget.value; applyPlan(selectedService?.plans.find((p) => p.id === planPresetId)); }}>
                {#each selectedService.plans.filter((p) => !p.retired || p.id === planPresetId) as p}
                  <option value={p.id}>{p.name}{p.retired || p.legacy ? tr("（旧订阅）", " (legacy subscription)") : ""}</option>
                {/each}
                <option value="">{tr("自定义套餐", "Custom plan")}</option>
              </select>
            </label>{/if}
            {#if !selectedService || !planPresetId}<label>
              {tr("自定义套餐", "Custom plan")}
              <input bind:value={record.plan} placeholder={tr("输入套餐名称", "Enter plan name")} />
            </label>{/if}
            {/if}
          </div>
        </div>
        <fieldset class="kind-choice">
          <legend>{tr("资产类型", "Asset type")}</legend><label
            class:active={record.kind === "recurring"}
            ><input
              type="radio"
              value="recurring"
              bind:group={record.kind}
              onchange={() => {
                record.kind = "recurring";
                setKind();
              }}
            /><span><Icon name="refresh" size={16}/>{tr("订阅", "Subscription")}</span></label
          ><label class:active={record.kind === "lifetime"}
            ><input
              type="radio"
              value="lifetime"
              disabled={record.category === "domain"}
              bind:group={record.kind}
              onchange={() => {
                record.kind = "lifetime";
                setKind();
              }}
            /><span><Icon name="infinity" size={16}/>{tr("买断", "Lifetime")}</span></label
          >
        </fieldset>
        <div class="form-grid">
          <label
            >{record.kind === "lifetime"
              ? tr("买断金额", "Purchase price")
              : tr("每期金额", "Price per cycle")}<input
              required
              inputmode="decimal"
              pattern={"[0-9]+([.][0-9]{1,12})?"}
              bind:value={record.amount}
              oninput={() => (autoPrice = false)}
              placeholder="0.00"
            /></label
          ><label
            >{tr("计价币种", "Billing currency")}<input
              required
              list="currencies"
              bind:value={record.currency}
              placeholder="CNY"
            /><datalist id="currencies"
              >{#each currencies as c, i}<option value={c}
                  >{en ? c : currencyNames[i]}</option
                >{/each}</datalist
            ></label
          >
        </div>
        {#if selectedPlan}<div class="hint price-hint" aria-live="polite">
          {#if quote.status === "unpriced"}
            {tr("此套餐或付款周期暂无固定参考价，请填写实际账单金额。", "No fixed reference price for this plan or billing cycle. Enter your bill amount.")}
          {:else}
            <span>{tr("美元参考价", "USD reference price")}: ${quote.price.amountUsd}
              {#if quote.price.cycleUnit} / {quote.price.cycleCount ?? 1} {unitLabel(quote.price.cycleUnit)}{/if}
              {selectedPlan.priceBasis === "per-seat" ? tr(" / 人", " / seat") : ""}
              · {selectedPlan.priceReviewedAt ?? selectedService?.reviewedAt}
            </span>
            {#if quote.status === "missing-rate"}
              <span>{tr("缺少有效汇率，请在更多选项填写固定汇率，或手动输入金额。", "Exchange rate unavailable. Set a fixed rate under More options or enter the amount manually.")}</span>
            {:else}
              {#if !autoPrice}<button type="button" onclick={() => (autoPrice = true)}>{tr("使用参考价", "Use reference price")}</button>{/if}
            {/if}
          {/if}
          <a href={selectedPlan.priceSource ?? selectedService?.source} onclick={(event) => { event.preventDefault(); void external(selectedPlan?.priceSource ?? selectedService?.source); }}>{tr("官方价格", "Official pricing")}</a>
        </div>{/if}
        {#if record.kind === "recurring"}<div class="form-grid">
            <label
              >{tr("每隔", "Every")}<input
                required
                type="number"
                min="1"
                max="120"
                step="1"
                bind:value={record.cycleCount}
                oninput={(e) => { record.cycleCount = Number(e.currentTarget.value); updateExpiry(); }}
              /></label
            ><label
              >{tr("付款周期", "Billing cycle")}<select
                bind:value={record.cycleUnit}
                onchange={(e) => { record.cycleUnit = e.currentTarget.value; updateExpiry(); }}
                >{#each ["day", "week", "month", "year"] as unit}<option
                    value={unit}>{unitLabel(unit)}</option
                  >{/each}</select
              ></label
            >
          </div>{/if}
        <div class="form-grid">
          <label>{record.kind === "lifetime" ? tr("购买日期", "Purchase date") : tr("开始日期", "Start date")}
            <DatePicker value={record.purchasedAt} {en} required label={record.kind === "lifetime" ? tr("购买日期", "Purchase date") : tr("开始日期", "Start date")} onchange={(value) => { record.purchasedAt = value; updateExpiry(); }}/>
          </label>
          {#if record.kind === "recurring"}<label>{tr("到期日期", "Expiry date")}
            <DatePicker value={record.expiresAt ?? ""} {en} required={!record.autoRenew} min={record.purchasedAt} label={tr("到期日期", "Expiry date")} onchange={(value) => record.expiresAt = value || null}/>
          </label>{/if}
        </div>
        {#if record.kind === "recurring"}<label class="switch-row"
            ><span
              ><strong>{tr("自动续期", "Auto-renew")}</strong><small
                >{tr(
                  "按付款周期推进记录中的到期日，不会执行扣款",
                  "Advance the recorded due date; no payments are made",
                )}</small
              ></span
            ><input
              type="checkbox"
              role="switch"
              bind:checked={record.autoRenew}
            /></label
          >{:else}<div class="hint">
            {tr(
              "从购买当天起按经过的天数均摊，一次付费的平均成本会逐日降低。",
              "Amortized over calendar days since purchase, including the first day. Average cost decreases daily.",
            )}
          </div>{/if}
        <div class="divider"></div>
        <div class="form-grid">
          <label
            >{tr("报销", "Reimbursement")}<select
              bind:value={record.reimbursement}
              ><option value="none">{tr("无报销", "None")}</option><option
                value="full">{tr("全额报销", "Fully reimbursed")}</option
              ><option value="fixed">{tr("固定金额", "Fixed amount")}</option
              ><option value="percent">{tr("百分比", "Percentage")}</option
              ></select
            ></label
          >{#if ["fixed", "percent"].includes(record.reimbursement)}<label
              >{record.reimbursement === "percent"
                ? tr("报销比例 %", "Reimbursement %")
                : tr(
                    `报销金额 (${record.currency} / 同期)`,
                    `Amount (${record.currency} / same cycle)`,
                  )}<input
                required
                inputmode="decimal"
                pattern={"[0-9]+([.][0-9]{1,12})?"}
                bind:value={record.reimbursementValue}
              /></label
            >{:else}<label
              >{tr("支付方式", "Payment method")}<select
                bind:value={record.paymentMethod}
                ><option value="">{tr("未指定", "Unspecified")}</option
                >{#each paymentMethods as p}<option value={p[0]}
                    >{p[en ? 2 : 1]}</option
                  >{/each}</select
              ></label
            >{/if}
        </div>
        {#if ["fixed", "percent"].includes(record.reimbursement)}<label
            >{tr("支付方式", "Payment method")}<select
              bind:value={record.paymentMethod}
              ><option value="">{tr("未指定", "Unspecified")}</option
              >{#each paymentMethods as p}<option value={p[0]}
                  >{p[en ? 2 : 1]}</option
                >{/each}</select
            ></label
          >{/if}
        <details>
          <summary>{tr("更多选项", "More options")}</summary><label
            >{tr(
              "固定汇率（1 原币 = 多少 USD，可选）",
              "Fixed rate (USD per 1 original unit, optional)",
            )}<input
              inputmode="decimal"
              value={record.fixedUsdRate ?? ""}
              oninput={(e) =>
                (record.fixedUsdRate = e.currentTarget.value || null)}
              placeholder={tr(
                "留空使用全局固定汇率或缓存汇率",
                "Leave blank for global fixed or cached rates",
              )}
            /></label
          ><label
            >{tr("备注", "Notes")}<textarea
              rows="2"
              maxlength="4000"
              bind:value={record.notes}></textarea></label
          ><label class="switch-row"
            ><span>{tr("计入成本统计", "Include in cost summary")}</span><input
              type="checkbox"
              role="switch"
              bind:checked={record.active}
            /></label
          >
        </details>
      </div>
      <div class="dialog-footer">
        {#if entries.some((s) => s.id === record.id)}<button
            type="button"
            class="danger"
            disabled={busy}
            onclick={remove}>{tr("删除", "Delete")}</button
          >{:else}<button
            type="button"
            class="text-button"
            onclick={() => (modal = "picker")}
            ><Icon name="back" size={16}/>{tr("选择服务", "Choose service")}</button
          >{/if}
        <div>
          <button type="button" onclick={() => (modal = "")}
            >{tr("取消", "Cancel")}</button
          ><button class="primary" disabled={busy} type="submit"
            >{busy
              ? tr("保存中…", "Saving…")
              : tr("保存资产", "Save asset")}</button
          >
        </div>
      </div>
    </form>
  {:else if modal === "settings"}<div class="dialog-header">
      <div>
        <h2>{tr("设置", "Settings")}</h2>
      </div>
      <button
        class="icon-button"
        aria-label={tr("关闭", "Close")}
        onclick={() => {
          modal = "";
          preview = null;
        }}><Icon name="close" size={16}/></button
      >
    </div>
    <div class="settings-tabs">
      {#each [["general", "通用", "General"], ["rates", "汇率", "Exchange rates"], ["data", "数据", "Data"], ["mcp", "Agent / MCP", "Agent / MCP"], ["about", "关于", "About"]] as tab}<button
          class:active={settingsTab === tab[0]}
          onclick={() => (settingsTab = tab[0])}>{tab[en ? 2 : 1]}</button
        >{/each}
    </div>
    <div class="dialog-content settings-content">
      {#if settingsTab === "general"}
        <div class="switch-row"><span>{tr("离线模式", "Offline mode")}<small>{tr("禁止 Subbit 主动联网，立即生效", "Stop Subbit from connecting online; applies immediately")}</small></span><button role="switch" aria-checked={offline} disabled={busy || updateState === "downloading"} onclick={toggleOffline}>{offline ? tr("已开启", "On") : tr("已关闭", "Off")}</button></div>
        <div class="form-grid">
          <label
            >{tr("语言", "Language")}<select bind:value={prefs.language}
              ><option value="system">{tr("跟随系统", "System")}</option><option
                value="zh">中文</option
              ><option value="en">English</option></select
            ></label
          ><label
            >{tr("外观", "Appearance")}<select bind:value={prefs.theme}
              ><option value="system">{tr("跟随系统", "System")}</option><option
                value="light">{tr("浅色", "Light")}</option
              ><option value="dark">{tr("深色", "Dark")}</option></select
            ></label
          ><label
            >{tr("默认显示币种", "Default display currency")}<select
              bind:value={prefs.currency}
              >{#each currencies.slice(0, 26) as c}<option>{c}</option
                >{/each}</select
            ></label
          ><label
            >{tr("默认统计周期", "Default summary period")}<select bind:value={prefs.period}
              >{#each ["hour", "month", "year"] as unit}<option value={unit}
                  >{unitLabel(unit)}</option
                >{/each}</select
            ></label
          >
        </div>
        <label class="switch-row"
          ><span
            >{tr("显示货币符号", "Show currency symbols")}<small
              >¥ / $ / €</small
            ></span
          ><input
            type="checkbox"
            role="switch"
            bind:checked={prefs.currencySymbol}
          /></label
        ><label class="switch-row"
          ><span
            >{tr("显示花销等级", "Show spending level")}<small
              >{tr(
                "按人民币月均成本计算",
                "Based on monthly cost in CNY",
              )}</small
            ></span
          ><input
            type="checkbox"
            role="switch"
            bind:checked={prefs.showLevel}
          /></label
        >{#if prefs.showLevel}<label
            >{tr("等级依据", "Level basis")}<select
              bind:value={prefs.levelBasis}
              ><option value="actual">{labels.personalCost}</option
              ><option value="bill">{labels.billedCost}</option
              ></select
            ></label
          >{#each prefs.levels as level, i}<div class="level-rule">
              <input
                aria-label={tr("起始金额", "From amount")}
                bind:value={level.above}
              /><input
                aria-label={tr("等级名称", "Level name")}
                value={localizeLevelName(level.name, en)}
                oninput={(event) => level.name = event.currentTarget.value}
              /><button aria-label={tr("删除等级", "Delete level")} onclick={() => prefs.levels.splice(i, 1)}><Icon name="close" size={16}/></button>
            </div>{/each}<button
            onclick={() => prefs.levels.push({ above: "100", name: "" })}
            ><Icon name="plus" size={16}/>{tr("新增等级", "Add level")}</button
          ><button class="text-button" onclick={() => prefs.levels = defaultLevels(en)}>{tr("恢复默认等级", "Restore default levels")}</button>{/if}
        <div class="switch-row">
          <span
            >{tr("登录时启动", "Launch at login")}<small
              >{tr(
                "启动后驻留菜单栏 / 系统托盘",
                "Start quietly in the system tray",
              )}</small
            ></span
          ><button
            role="switch"
            aria-checked={autoStart}
            class:enabled={autoStart}
            disabled={busy}
            onclick={toggleStartup}
            >{autoStart ? tr("已开启", "On") : tr("已关闭", "Off")}</button
          >
        </div>
        <div class="hint">
          {tr(
            "关闭窗口后仍驻留菜单栏 / 系统托盘。退出请使用托盘菜单或下方按钮。",
            "Closing the window keeps Subbit in the tray. Use the tray menu or the button below to quit.",
          )}
        </div>
      {:else if settingsTab === "rates"}<label class="switch-row"
          ><span
            >{tr("自动更新汇率", "Automatic exchange rates")}<small
              >{tr(
                "法币每天；有加密货币记录时每 15 分钟检查缓存",
                "Fiat daily; crypto cache checked after 15 minutes when used",
              )}</small
            ></span
          ><input
            type="checkbox"
            role="switch"
            bind:checked={prefs.autoRates}
          /></label
        >
        <p class="hint">
          {tr(
            "汇率单位统一为「1 原币 = X USD」。手动固定汇率优先；稳定币也使用实际报价。",
            "Rates are USD per original unit. Fixed rates override fetched rates; stablecoins use market quotes.",
          )}
        </p>
        <button
          disabled={busy || offline}
          onclick={() =>
            action(async () => {
              const warnings = await call<string[]>("refresh_rates", {
                force: true,
              });
              await reload();
              settingsRevision = snap!.database.revision;
              notice = warnings.length
                ? tr(
                    "部分汇率更新失败，保留已有缓存。",
                    "Some exchange rates could not be updated. Cached rates were retained.",
                  )
                : labels.ratesUpdated;
            })}><Icon name="refresh" size={16}/>{tr("更新汇率", "Update exchange rates")}</button
        >
        <div class="rate-input">
          <select bind:value={rateCode}
            >{#each currencies as c}<option>{c}</option>{/each}</select
          ><input
            aria-label={tr("固定汇率", "Fixed exchange rate")}
            placeholder={`1 ${rateCode} = ? USD`}
            bind:value={rateValue}
          /><button onclick={addRate}>{tr("设置固定汇率", "Set fixed rate")}</button>
        </div>
        <p class="hint">{tr("缺失的法币汇率使用安装包内置快照（Frankfurter，2026-10-16 / 17）；已有缓存与固定汇率优先。", "Missing fiat rates use the bundled Frankfurter snapshot (2026-10-16 / 17); cached and fixed rates take priority.")}</p>
        <div class="rates-list">
          {#each currencies as c}{#if prefs.fixedRates[c] || snap?.database.rates[c]}<div
              >
                <strong>{c}</strong><span
                  >{prefs.fixedRates[c] ?? snap?.database.rates[c]?.usd} USD<small
                    >{prefs.fixedRates[c]
                      ? tr("固定汇率", "Fixed exchange rate")
                      : localTimestamp(snap?.database.rates[c]?.asOf)}</small
                  ></span
                >{#if prefs.fixedRates[c]}<button
                    aria-label={tr("移除固定汇率", "Remove fixed rate")}
                    onclick={() => delete prefs.fixedRates[c]}><Icon name="close" size={16}/></button
                  >{/if}
              </div>{/if}{/each}
        </div>
      {:else if settingsTab === "data"}<p class="hint">
          {tr(
            "所有资产、设置、自定义图标与汇率都保存在一个 JSON 文件中。每次修改保留上一份 .bak 备份。",
            "Assets, settings, custom icons and exchange rates live in one JSON file. Every write keeps the previous .bak backup.",
          )}
        </p>
        <div class="data-path">
          {tr("当前数据文件", "Current data file")}<code class="path"
            >{snap?.path}</code
          >
        </div>
        <p class="file-sizes">{#if fileInfo}{tr("数据文件", "Data file")} {fileInfo.data == null ? tr("尚未创建", "Not created") : bytesLabel(fileInfo.data)} · {tr("备份", "Backup")} {fileInfo.backup == null ? tr("尚无备份", "No backup yet") : bytesLabel(fileInfo.backup)} · {tr("合计", "Total")} {bytesLabel((fileInfo.data ?? 0) + (fileInfo.backup ?? 0))}{:else}{fileInfoError ? tr("无法读取文件大小", "File sizes unavailable") : "—"}{/if}</p>
        <div class="button-row">
          <button disabled={busy} onclick={() => action(async () => { await call("reveal_data"); })}>{tr("打开所在文件夹", "Show in folder")}</button>
          <button disabled={busy} onclick={moveData}
            >{tr("更换存储文件夹", "Change folder")}</button
          ><button disabled={busy} onclick={locateData}
            >{labels.locateData}</button
          >
        </div>
        <div class="divider"></div>
        <div class="button-row">
          <button disabled={busy} onclick={exportData}
            ><Icon name="upload" size={16}/>{tr("导出备份", "Export backup")}</button
          ><button disabled={busy} onclick={importData}
            ><Icon name="download" size={16}/>{tr("导入 JSON", "Import JSON")}</button
          >
        </div>
        {#if preview}<div class="import-preview">
            <strong
              >{tr(
                `导入 ${preview.subscriptions.length} 项资产`,
                `Import ${preview.subscriptions.length} assets`,
              )}</strong
            >
            <p>
              {tr(
                `将替换当前 ${entries.length} 项资产和设置，原文件会备份。`,
                `Replaces ${entries.length} current assets and settings. The old file will be backed up.`,
              )}
            </p>
            <button class="primary" disabled={busy} onclick={replaceData}
              >{tr("确认替换", "Replace data")}</button
            ><button onclick={() => (preview = null)}
              >{tr("取消", "Cancel")}</button
            >
          </div>{/if}
        <h3>{tr("服务目录", "Service catalog")}</h3>
        <p class="hint">{services.length} {tr("项服务", "services")} · {categories.length} {tr("个分类", "categories")} · {directory.updatedAt ? localTimestamp(directory.updatedAt) : tr("内置目录", "Built-in catalog")}</p>
        <button disabled={catalogBusy || offline} onclick={() => refreshDirectory(true)}><Icon name="refresh" size={16}/>{catalogBusy ? tr("正在更新…", "Updating…") : tr("更新目录", "Update catalog")}</button>
        {#if catalogError}<p class="hint" role="status">{tr("更新失败，继续使用本地目录", "Update failed. Using the local catalog.")}</p>{/if}
      {:else if settingsTab === "about"}<div class="about-app">
        <img class="app-icon" src={dark ? appDark : appLight} alt="Subbit"/>
        <h2>Subbit</h2><p>{tr("版本", "Version")} {version}</p>
        <p>{tr("作者", "Made by")} <a href="https://github.com/sywyyhykkk" onclick={(e) => { e.preventDefault(); void external(e.currentTarget.href); }}>mango</a></p>
        <a href="https://github.com/sywyyhykkk/subbit" onclick={(e) => { e.preventDefault(); void external(e.currentTarget.href); }}>GitHub · sywyyhykkk/subbit <Icon name="arrow" size={14}/></a>
      </div>
      <div class="update-box" aria-live="polite">
        {#if updateState === "available"}<strong>{tr("发现新版本", "New version available")} {update?.version}</strong><button onclick={() => showUpdate = true}>{tr("查看更新内容", "View release notes")}</button>
        {:else if updateState === "downloading"}<span>{tr("正在下载并安装…", "Downloading and installing…")} {downloadTotal ? `${Math.min(100, Math.round(downloaded / downloadTotal * 100))}%` : `${(downloaded / 1048576).toFixed(1)} MB`}</span>
        {:else if updateState === "installed"}<p>{tr("更新已安装，重启即可使用", "Update installed. Restart to finish.")}</p><button class="primary" onclick={() => relaunch()}>{tr("立即重启", "Restart now")}</button>
        {:else}<button disabled={updateState === "checking"} onclick={checkUpdate}><Icon name="refresh" size={16}/>{updateState === "checking" ? tr("正在检查…", "Checking…") : tr("检查更新", "Check for updates")}</button>
          {#if updateState === "current"}<p class="hint">{tr("已是最新版本", "You're up to date")}</p>{/if}
          {#if updateState === "error"}<p class="hint">{tr("更新失败，请重试。", "Update failed. Please retry.")} {updateError}</p>{/if}
        {/if}
      </div>
      {:else}<label class="switch-row"
          ><span
            ><strong>{tr("启用本地 MCP", "Enable local MCP")}</strong><small
              >{tr(
                "让 Agent 查询或导入你的资产",
                "Let your Agent read or import assets",
              )}</small
            ></span
          ><input
            type="checkbox"
            role="switch"
            bind:checked={prefs.mcpEnabled}
          /></label
        >
        <p class="hint">
          {tr(
            "保存设置后，将下面的配置添加到支持 stdio MCP 的客户端。无需端口或额外运行环境。",
            "Save settings, then add this configuration to a client supporting stdio MCP. No port or additional runtime is needed.",
          )}
        </p>
        <pre>{mcpConfig}</pre>
        <button onclick={copyMcp}>{tr("复制连接配置", "Copy connection config")}</button>
        <p class="hint">
          {tr(
            "可用工具：查询目录、列出资产、预览导入、批量导入、费用汇总。使用云端 Agent 时，工具返回的数据会交给该 Agent 处理。",
            "Tools: catalog, subscriptions, import preview, batch import and cost summary. With a cloud Agent, returned data is processed by that Agent.",
          )}
        </p>{/if}
    </div>
    <div class="dialog-footer">
      <button class="text-button" onclick={() => call("quit")}
        >{tr("退出 Subbit", "Quit Subbit")}</button
      >
      <div>
        <button
          onclick={() => {
            modal = "";
            preview = null;
          }}>{tr("取消", "Cancel")}</button
        ><button class="primary" disabled={busy} onclick={savePrefs}
          >{tr("保存设置", "Save settings")}</button
        >
      </div>
    </div>{/if}
  {#if error}<div class="dialog-error" role="alert">
      {error}<button aria-label={tr("关闭", "Close")} onclick={() => (error = "")}><Icon name="close" size={16}/></button>
    </div>{/if}
</dialog>
{#if error && !modal}<div class="toast error" role="alert">
    <span>{error}</span><button onclick={locateData}
      >{labels.locateData}</button
    ><button aria-label={tr("关闭", "Close")} onclick={() => (error = "")}
      ><Icon name="close" size={16}/></button
    >
  </div>{/if}
{#if notice}<div class="toast" role="status">
    <span>{notice}</span><button
      aria-label={tr("关闭", "Close")}
      onclick={() => (notice = "")}><Icon name="close" size={16}/></button
    >
  </div>{/if}

<dialog bind:this={updateDialog} class="update-dialog" aria-label={tr("更新内容", "Release notes")} oncancel={(event) => { if (updateState === "downloading") event.preventDefault(); else showUpdate = false; }}>
  <div class="dialog-header"><h2>{tr("更新内容", "Release notes")} {update?.version ?? ""}</h2><button class="icon-button" disabled={updateState === "downloading"} aria-label={tr("关闭", "Close")} onclick={() => showUpdate = false}><Icon name="close"/></button></div>
  <div class="dialog-content">{#if showUpdate}<ReleaseNotes text={update?.body ?? tr("暂无更新说明", "No release notes available")} openLink={(url) => void external(url)}/>{/if}{#if updateState === "error"}<p role="alert">{updateError}</p>{/if}</div>
  <div class="dialog-footer"><button disabled={updateState === "downloading"} onclick={() => showUpdate = false}>{tr("稍后", "Later")}</button>
    {#if updateState === "installed"}<button class="primary" onclick={() => relaunch()}>{tr("立即重启", "Restart now")}</button>
    {:else}<button class="primary" disabled={offline || updateState !== "available"} onclick={installUpdate}>{updateState === "downloading" ? tr("正在下载并安装…", "Downloading and installing…") : tr("下载并安装", "Download and install")}</button>{/if}
  </div>
</dialog>
