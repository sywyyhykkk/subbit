<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { presetQuote } from "./pricing";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open, save, confirm } from "@tauri-apps/plugin-dialog";
  import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
  import {
    categories,
    services,
    serviceIcon,
    currencies,
    currencyNames,
    paymentMethods,
    defaultSettings,
  } from "./catalog";
  import type {
    Subscription,
    Settings,
    Snapshot,
    Service,
    Database,
  } from "./types";

  const panel = new URLSearchParams(location.search).has("panel");
  let snap: Snapshot | null = $state(null);
  let loading = $state(true),
    busy = $state(false),
    error = $state(""),
    notice = $state("");
  let currency = $state("CNY"),
    period = $state("month"),
    category = $state("all"),
    search = $state(""),
    filter = $state("all");
  let modal = $state<"" | "picker" | "editor" | "settings">("");
  let pickerSearch = $state(""),
    pickerCategory = $state("all");
  let record: Subscription = $state(blank());
  let planPresetId = $state("");
  let autoPrice = $state(false);
  let editRevision = 0;
  let prefs: Settings = $state(structuredClone(defaultSettings));
  let settingsRevision = 0,
    settingsTab = $state("general"),
    autoStart = $state(false),
    rateCode = $state("USD"),
    rateValue = $state("");
  let preview: Database | null = $state(null);
  let inputDialog: HTMLDialogElement;
  const en = $derived.by(
    () =>
      snap?.database.settings.language === "en" ||
      (snap?.database.settings.language !== "zh" &&
        navigator.language.startsWith("en")),
  );
  const tr = (zh: string, english: string) => (en ? english : zh);
  const entries = $derived.by(() => snap?.database.subscriptions ?? []);
  const filtered = $derived(
    entries.filter(
      (s) =>
        (category === "all" || s.category === category) &&
        (filter === "all" || s.kind === filter) &&
        `${s.name} ${s.plan} ${s.notes}`
          .toLowerCase()
          .includes(search.toLowerCase()),
    ),
  );
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
    entries.filter(
      (s, i, all) =>
        !s.serviceId && all.findIndex((x) => x.name === s.name) === i,
    ),
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
  async function reload(initial = false) {
    try {
      if (!isTauri()) {
        snap = {
          database: {
            schemaVersion: 1,
            revision: 0,
            subscriptions: [],
            settings: structuredClone(defaultSettings),
            rates: {},
            imports: {},
          },
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
      const next = await call<Snapshot>(
        "snapshot",
        initial ? {} : { currency, period },
      );
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
          "汇率更新失败，继续使用已有缓存。",
          "Rate update failed. Cached rates are still in use.",
        );
      else if (force) notice = tr("汇率已更新", "Exchange rates updated");
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
    record = blank();
    record.currency = currency;
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
    modal = "editor";
  }
  function edit(s: Subscription) {
    if (panel) {
      void call("open_main");
      return;
    }
    record = structuredClone($state.snapshot(s));
    planPresetId = services.find((service) => service.id === s.serviceId)
      ?.plans.find((p) => p.name === s.plan)?.id ?? "";
    autoPrice = false;
    editRevision = snap!.database.revision;
    modal = "editor";
  }
  function setKind() {
    if (record.kind === "lifetime") {
      record.autoRenew = false;
      record.expiresAt = null;
    } else {
      record.autoRenew = true;
    }
  }
  function applyPlan(plan: Service["plans"][number] | undefined) {
    autoPrice = !!plan;
    record.plan = plan?.name ?? "";
    record.amount = "";
    if (plan) {
      record.kind = plan.kind;
      record.cycleUnit = plan.prices?.[0]?.cycleUnit ?? plan.cycleUnit ?? "month";
      record.cycleCount = plan.prices?.[0]?.cycleCount ?? 1;
      setKind();
    }
  }
  async function submit() {
    await action(async () => {
      const payload = structuredClone($state.snapshot(record));
      payload.name = payload.name.trim();
      payload.expiresAt = payload.expiresAt || null;
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
  async function uploadIcon(file?: File) {
    if (!file) return;
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
    modal = "settings";
    settingsTab = "general";
    if (isTauri())
      try {
        autoStart = await isEnabled();
      } catch (e) {
        error = String(e);
      }
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
  $effect(() => {
    document.documentElement.dataset.theme =
      snap?.database.settings.theme ?? "system";
    document.documentElement.lang = en ? "en" : "zh-CN";
  });
  $effect(() => {
    if (modal && inputDialog && !inputDialog.open) inputDialog.showModal();
    if (!modal && inputDialog?.open) inputDialog.close();
  });
  onMount(() => {
    let cleanup: (() => void)[] = [];
    let disposed = false;
    void (async () => {
      await reload(true);
      if (new URLSearchParams(location.search).has("settings"))
        await settings();
      if (isTauri()) {
        if (new URLSearchParams(location.search).has("add")) await picker();
        for (const name of ["data-changed", "open-settings", "open-add"]) {
          const unlisten = await listen(name, () => {
            if (name === "open-settings") void settings();
            else if (name === "open-add") void picker();
            else if (!modal) void reload();
          });
          if (disposed) unlisten();
          else cleanup.push(unlisten);
        }
        if (!modal) void rates();
      }
    })();
    const focus = () => {
      if (!modal) {
        void reload();
      }
    };
    window.addEventListener("focus", focus);
    const interval = setInterval(() => {
      if (document.hasFocus() && !modal && !busy) void reload();
    }, 15000);
    return () => {
      disposed = true;
      cleanup.forEach((f) => f());
      clearInterval(interval);
      window.removeEventListener("focus", focus);
    };
  });
</script>

<div class:panel class="shell">
  <main>
    <header class="page-header">
      <div class="page-title"><h1>Subbit</h1><span>{tr("数字资产", "Digital assets")}</span>{#if snap?.level}<span class="level">{snap.level}</span>{/if}</div>
      <div class="header-actions">
        {#if !panel}<button class="icon-button" aria-label={tr("设置", "Settings")} title={tr("设置", "Settings")} onclick={settings}><Icon name="settings"/></button>{/if}
        <button class="primary" onclick={picker}><Icon name="plus"/>{tr("新增资产", "Add asset")}</button>
      </div>
    </header>
    <section class="overview" aria-label={tr("成本总览", "Cost overview")}>
      <div class="overview-heading"><span>{tr("全部资产的均摊成本", "Average cost of all assets")}</span>
        <div class="controls"><select aria-label={tr("显示币种", "Display currency")} bind:value={currency} onchange={() => reload()}>{#each currencies.slice(0, 26) as c}<option>{c}</option>{/each}</select>
          <div class="segmented">{#each ["hour", "month", "year"] as unit}<button class:active={period === unit} aria-pressed={period === unit} onclick={() => {period = unit; void reload();}}>{tr("每", "/")}{unitLabel(unit)}</button>{/each}</div>
        </div>
      </div>
      <div class="summary-grid">
        <div class="personal"><div class="card-label">{tr("实际花销", "Personal cost")}</div><div class="amount">{money(snap?.summary.actual)}<small>/ {unitLabel(period)}</small></div><div class="card-foot">{tr("报销后个人承担", "After reimbursement")}</div></div>
        <div class="secondary-costs"><div class="bill"><div class="card-label">{tr("账单花销", "Billed cost")}</div><div class="secondary-amount">{money(snap?.summary.bill)}</div></div><div class="reimbursed"><div class="card-label">{tr("报销抵扣", "Reimbursed")}</div><div class="secondary-amount">{money(snap?.summary.reimbursed)}</div></div></div>
      </div>
    </section>
    {#if snap?.summary.missingRates.length}<div class="banner warning">{tr("部分币种缺少汇率，总额暂不可用。", "Some exchange rates are missing.")}<button onclick={settings}>{tr("设置汇率", "Set rates")}</button></div>{/if}
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
            {#each categories as c}<option value={c.id}>{en ? c.en : c.zh}</option>{/each}
          </select>
          <label class="search-field"><Icon name="search" size={16}/><input class="search" aria-label={tr("搜索资产", "Search assets")} placeholder={tr("搜索资产", "Search assets")} bind:value={search}/></label>
        </div>
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
          <p>
            {entries.length
              ? tr("试试其他分类或关键词。", "Try another category or search.")
              : tr(
                  "从第一份订阅开始，看看每一笔花销去了哪里。",
                  "Start with a subscription and see where it all goes.",
                )}
          </p>
          {#if entries.length}<button onclick={() => { category = "all"; filter = "all"; search = ""; }}>{tr("清除筛选", "Clear filters")}</button>
          {:else}<button class="primary" onclick={picker}><Icon name="plus"/>{tr("添加资产", "Add an asset")}</button>{/if}
        </div>
      {:else}<div class="list-head">
          <span>{tr("服务 / 套餐", "SERVICE / PLAN")}</span><span
            >{tr("个人承担", "YOUR COST")} / {unitLabel(period)}</span
          >
        </div>
        <div class="asset-list">
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
                  />{:else}<span
                    >{item.category === "vpn"
                      ? "⌁"
                      : item.name.slice(0, 1)}</span
                  >{/if}</span
              >
              <span class="asset-info"
                ><strong
                  >{item.name}<span class="tag"
                    >{item.kind === "lifetime"
                      ? tr("买断", "Lifetime")
                      : categoryName(item.category)}</span
                  ></strong
                ><span class="asset-meta"
                  >{item.plan || tr("自定义套餐", "Custom plan")} · {item.currency}
                  {item.amount}{item.kind === "recurring"
                    ? ` / ${item.cycleCount > 1 ? item.cycleCount : ""}${unitLabel(item.cycleUnit)}`
                    : ""}</span
                ><span class="asset-detail"
                  >{#if !item.active}{tr(
                      "已停用",
                      "Paused",
                    )}{:else if !cost?.current}{item.purchasedAt > today()
                      ? tr("尚未开始", "Not started")
                      : tr(
                          "已到期",
                          "Expired",
                        )}{:else if item.kind === "lifetime"}{tr(
                      `已拥有 ${cost.elapsedDays} 天 · 成本逐日降低`,
                      `Owned ${cost.elapsedDays} days · cost decreases daily`,
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
                ><strong>{money(cost?.actual)}</strong><span
                  >{item.reimbursement !== "none"
                    ? tr("含报销抵扣", "After reimbursement")
                    : tr("均摊成本", "Average cost")}</span
                ></span
              ><span class="chevron"><Icon name="chevron" size={16}/></span>
            </button>{/each}
        </div>{/if}
    </section>
    <footer><span>{filtered.length} / {entries.length} {tr("项资产", "assets")}<span class="footer-separator">·</span>{tr("本地保存", "Saved locally")}</span>{#if panel}<button onclick={() => call("open_main")}>{tr("打开窗口", "Open window")}<Icon name="arrow" size={16}/></button>{:else}<span>{tr("当前成本折算，非扣款流水", "Amortized cost, not transactions")}</span>{/if}</footer>
  </main>
</div>

<dialog
  bind:this={inputDialog}
  class:settings-modal={modal === "settings"}
  oncancel={() => {
    modal = "";
    preview = null;
  }}
  onclose={() => (modal = "")}
>
  {#if modal === "picker"}<div class="dialog-header">
      <div>
        <h2>{tr("新增数字资产", "Add a digital asset")}</h2>
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
              : tr("添加资产", "Add asset")}
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
                />{:else}{record.category === "vpn"
                  ? "⌁"
                  : record.name.slice(0, 1) || "＋"}{/if}</span
            ><input
              type="file"
              accept="image/png,image/jpeg,image/webp"
              aria-label={tr("上传图标", "Upload icon")}
              onchange={(e) => uploadIcon(e.currentTarget.files?.[0])}
            /><small>{tr("更换图标", "Change icon")}</small></label
          ><label class="grow"
            >{tr("名称", "Name")}<input
              required
              maxlength="100"
              bind:value={record.name}
              placeholder={tr("服务或软件名称", "Service or software name")}
            /></label
          >
        </div>
        <div class="form-grid">
          <label
            >{tr("分类", "Category")}<select bind:value={record.category}
              >{#each categories as c}<option value={c.id}
                  >{en ? c.en : c.zh}</option
                >{/each}</select
            ></label
          ><div class="plan-field">
            {#if selectedService}<label>
              {tr("套餐 / 档位", "Plan")}
              <select bind:value={planPresetId} onchange={() => applyPlan(selectedService?.plans.find((p) => p.id === planPresetId))}>
                {#each selectedService.plans.filter((p) => !p.retired || p.id === planPresetId) as p}
                  <option value={p.id}>{p.name}{p.retired ? tr("（旧套餐）", " (legacy)") : ""}</option>
                {/each}
                <option value="">{tr("自定义套餐", "Custom plan")}</option>
              </select>
            </label>{/if}
            {#if !selectedService || !planPresetId}<label>
              {tr("自定义套餐", "Custom plan")}
              <input bind:value={record.plan} placeholder={tr("输入套餐名称", "Enter plan name")} />
            </label>{/if}
          </div>
        </div>
        <fieldset class="kind-choice">
          <legend>{tr("购买方式", "Purchase type")}</legend><label
            class:active={record.kind === "recurring"}
            ><input
              type="radio"
              value="recurring"
              bind:group={record.kind}
              onchange={() => {
                record.kind = "recurring";
                setKind();
              }}
            /><span><Icon name="refresh" size={16}/>{tr("周期订阅", "Subscription")}</span></label
          ><label class:active={record.kind === "lifetime"}
            ><input
              type="radio"
              value="lifetime"
              bind:group={record.kind}
              onchange={() => {
                record.kind = "lifetime";
                setKind();
              }}
            /><span><Icon name="infinity" size={16}/>{tr("一次买断", "Lifetime purchase")}</span></label
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
              <span>{tr("按美元标价换算，税费及地区定价以实际账单为准，可直接修改。", "Converted from USD list price. Regional prices and taxes may differ; edit to match your bill.")}</span>
              {#if !autoPrice}<button type="button" onclick={() => (autoPrice = true)}>{tr("使用参考价", "Use reference price")}</button>{/if}
            {/if}
          {/if}
          <a href={selectedPlan.priceSource ?? selectedService?.source} target="_blank" rel="noreferrer">{tr("官方价格", "Official pricing")}</a>
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
              /></label
            ><label
              >{tr("付款周期", "Billing cycle")}<select
                bind:value={record.cycleUnit}
                >{#each ["day", "week", "month", "year"] as unit}<option
                    value={unit}>{unitLabel(unit)}</option
                  >{/each}</select
              ></label
            >
          </div>{/if}
        <div class="form-grid">
          <label
            >{record.kind === "lifetime"
              ? tr("购买日期", "Purchase date")
              : tr("开始日期", "Start date")}<input
              required
              type="date"
              min="1900-01-01"
              max="9999-12-31"
              bind:value={record.purchasedAt}
            /></label
          >{#if record.kind === "recurring"}<label
              >{tr("到期日期（可选）", "Expiry date (optional)")}<input
                type="date"
                min={record.purchasedAt}
                max="9999-12-31"
                required={!record.autoRenew}
                value={record.expiresAt ?? ""}
                onchange={(e) =>
                  (record.expiresAt = e.currentTarget.value || null)}
              /></label
            >{/if}
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
                "留空使用最新汇率",
                "Leave blank for latest rates",
              )}
            /></label
          ><label
            >{tr("备注", "Notes")}<textarea
              rows="2"
              maxlength="4000"
              bind:value={record.notes}></textarea></label
          ><label class="switch-row"
            ><span>{tr("计入当前资产", "Active asset")}</span><input
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
      {#each [["general", "通用", "General"], ["rates", "汇率", "Exchange rates"], ["data", "数据", "Data"], ["mcp", "Agent / MCP", "Agent / MCP"]] as tab}<button
          class:active={settingsTab === tab[0]}
          onclick={() => (settingsTab = tab[0])}>{tab[en ? 2 : 1]}</button
        >{/each}
    </div>
    <div class="dialog-content settings-content">
      {#if settingsTab === "general"}<div class="form-grid">
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
            >{tr("默认币种", "Default currency")}<select
              bind:value={prefs.currency}
              >{#each currencies.slice(0, 26) as c}<option>{c}</option
                >{/each}</select
            ></label
          ><label
            >{tr("默认周期", "Default period")}<select bind:value={prefs.period}
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
              ><option value="actual">{tr("实际花销", "Personal cost")}</option
              ><option value="bill">{tr("账单花销", "Billed cost")}</option
              ></select
            ></label
          >{#each prefs.levels as level, i}<div class="level-rule">
              <input
                aria-label={tr("超过金额", "Above amount")}
                bind:value={level.above}
              /><input
                aria-label={tr("等级名称", "Level name")}
                bind:value={level.name}
              /><button onclick={() => prefs.levels.splice(i, 1)}><Icon name="close" size={16}/></button>
            </div>{/each}<button
            onclick={() => prefs.levels.push({ above: "100", name: "" })}
            ><Icon name="plus" size={16}/>{tr("添加等级", "Add level")}</button
          >{/if}
        <div class="switch-row">
          <span
            >{tr("登录时启动", "Launch at login")}<small
              >{tr(
                "启动后安静驻留菜单栏",
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
            "关闭窗口后仍驻留菜单栏。退出请使用托盘菜单或下方按钮。",
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
          disabled={busy}
          onclick={() =>
            action(async () => {
              const warnings = await call<string[]>("refresh_rates", {
                force: true,
              });
              await reload();
              settingsRevision = snap!.database.revision;
              notice = warnings.length
                ? tr(
                    "部分更新失败，保留已有缓存",
                    "Some rates failed; cache retained",
                  )
                : tr("汇率已更新", "Rates updated");
            })}><Icon name="refresh" size={16}/>{tr("立即更新", "Update now")}</button
        >
        <div class="rate-input">
          <select bind:value={rateCode}
            >{#each currencies as c}<option>{c}</option>{/each}</select
          ><input
            aria-label={tr("固定美元汇率", "Fixed USD rate")}
            placeholder="1 USD = 1"
            bind:value={rateValue}
          /><button onclick={addRate}>{tr("固定", "Set")}</button>
        </div>
        <div class="rates-list">
          {#each currencies as c}{#if prefs.fixedRates[c] || snap?.database.rates[c]}<div
              >
                <strong>{c}</strong><span
                  >{prefs.fixedRates[c] ?? snap?.database.rates[c]?.usd} USD<small
                    >{prefs.fixedRates[c]
                      ? tr("手动固定", "Fixed")
                      : snap?.database.rates[c]?.asOf.slice(0, 10)}</small
                  ></span
                >{#if prefs.fixedRates[c]}<button
                    aria-label={tr("移除固定汇率", "Remove fixed rate")}
                    onclick={() => delete prefs.fixedRates[c]}><Icon name="close" size={16}/></button
                  >{/if}
              </div>{/if}{/each}
        </div>
      {:else if settingsTab === "data"}<h3>
          {tr("你的数据，随你带走", "Your data goes with you")}
        </h3>
        <p class="hint">
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
        <div class="button-row">
          <button disabled={busy} onclick={moveData}
            >{tr("更换存储文件夹", "Change folder")}</button
          ><button disabled={busy} onclick={locateData}
            >{tr("定位已有数据", "Locate existing data")}</button
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
      {:else}<label class="switch-row"
          ><span
            ><strong>{tr("启用本地 MCP", "Enable local MCP")}</strong><small
              >{tr(
                "让 Agent 查询或导入你的订阅",
                "Let your Agent read or import subscriptions",
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
        <button onclick={copyMcp}>{tr("复制连接配置", "Copy config")}</button>
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
      {error}<button onclick={() => (error = "")}><Icon name="close" size={16}/></button>
    </div>{/if}
</dialog>
{#if error && !modal}<div class="toast error" role="alert">
    <span>{error}</span><button onclick={locateData}
      >{tr("定位数据", "Locate data")}</button
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
