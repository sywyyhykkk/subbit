<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import Icon from "./Icon.svelte";
  import type { Snapshot, Category } from "./types";
  import { drawReport, type ReportData } from "./report";
  let { snapshot, categories, en, initialBasis, currencies, onclose, onchange }: {
    snapshot: Snapshot; categories: Category[]; en: boolean; initialBasis: "bill" | "actual";
    currencies: string[]; onclose: () => void; onchange: (currency: string, period: string) => Promise<void>;
  } = $props();
  let dialog: HTMLDialogElement;
  let basis = $state<"bill" | "actual">("bill");
  let pending = $state(false), exporting = $state(false), error = $state(""), saved = $state(false);
  onMount(() => { basis = initialBasis; dialog.showModal(); });
  const tr = (zh: string, english: string) => en ? english : zh;
  const summary = $derived(snapshot.summary);
  const total = $derived(Number(summary[basis] ?? 0));
  const available = $derived(summary.missingRates.length === 0);
  const unit = (p: string) => ({hour: tr("小时", "hour"), day: tr("天", "day"), month: tr("月", "month"), year: tr("年", "year")}[p] ?? p);
  const money = (v: string | null) => v == null ? "—" : new Intl.NumberFormat(en ? "en-US" : "zh-CN", { style: "currency", currency: summary.currency, minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(Number(v));
  const share = (v: string | null) => total > 0 ? Math.max(0, Math.min(100, Number(v ?? 0) / total * 100)) : 0;
  const categoryName = (id: string) => { const c = categories.find(c => c.id === id); return c ? (en ? c.en : c.zh) : id; };
  const rankedCategories = $derived([...summary.categories].sort((a,b) => Number(b[basis]) - Number(a[basis])));
  const top = $derived(summary.rows.filter(r => r.current).map(r => ({...r, name: snapshot.database.subscriptions.find(s => s.id === r.id)?.name ?? r.id})).sort((a,b) => Number(b[basis]) - Number(a[basis])).slice(0,5));
  const max = $derived(Math.max(0, ...top.map(r => Number(r.bill ?? 0))));
  async function change(currency: string, period: string) {
    pending = true; saved = false; error = "";
    try { await onchange(currency, period); } catch(e) { error = String(e); }
    finally { pending = false; }
  }
  async function exportPng() {
    exporting = true; error = ""; saved = false;
    try {
      const report: ReportData = {
        title: tr("花销报告", "Spending report"),
        subtitle: `${snapshot.key?.day ?? new Date().toLocaleDateString()} · ${summary.currency} / ${unit(summary.period)} · ${tr("全部有效资产", "All active assets")}`,
        mainLabel: tr("实际花销", "Actual cost"), main: money(summary.actual),
        secondary: `${tr("账单花销", "Bill cost")} ${money(summary.bill)}    ${tr("报销抵扣", "Reimbursed")} ${money(summary.reimbursed)}`,
        categoryTitle: rankedCategories.length > 10 ? tr("分类花销 TOP 10", "Top 10 categories") : tr("分类花销", "By category"), topTitle: tr("花销最高 TOP 5", "Top 5 assets"),
        basis: basis === "bill" ? tr("账单花销", "Bill cost") : tr("实际花销", "Actual cost"),
        categories: rankedCategories.slice(0,10).map(c => ({name: categoryName(c.id), value: `${money(c[basis])} · ${share(c[basis]).toFixed(1)}%`, fraction: share(c[basis]) / 100})),
        top: top.map(c => ({ name: c.name, value: money(c[basis]), fraction: max ? Number(c.bill) / max : 0, actualFraction: max ? Number(c.actual) / max : 0 })),
        legend: tr("深灰：实际花销 · 浅灰：报销抵扣", "Dark: actual cost · Light: reimbursed"),
        footer: `${summary.activeCount} ${tr("项有效资产 · 当前费用折算，非历史付款流水", "active assets · Current amortized costs, not payment history")}`,
      };
      const canvas = drawReport(report);
      const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob(b => b ? resolve(b) : reject(new Error("PNG export failed")), "image/png"));
      if (isTauri()) {
        const path = await save({ defaultPath: `Subbit-${snapshot.key?.day ?? "report"}.png`, filters: [{name: "PNG", extensions: ["png"]}] });
        if (path) { await invoke("save_report", {path, bytes: Array.from(new Uint8Array(await blob.arrayBuffer()))}); saved = true; }
      } else {
        const url = URL.createObjectURL(blob), link = document.createElement("a"); link.href = url; link.download = "Subbit-report.png"; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000); saved = true;
      }
    } catch(e) { error = String(e); }
    finally { exporting = false; }
  }
</script>

<dialog bind:this={dialog} class="statistics-dialog" onclose={onclose} aria-labelledby="statistics-title">
  <div class="dialog-header"><h2 id="statistics-title">{tr("花销统计", "Spending statistics")}</h2><button class="icon-button" aria-label={tr("关闭", "Close")} onclick={() => dialog.close()}><Icon name="close"/></button></div>
  <div class="dialog-content">
    <div class="statistics-controls">
      <select aria-label={tr("统计币种", "Report currency")} value={summary.currency} disabled={pending || exporting} onchange={e => change(e.currentTarget.value, summary.period)}>{#each currencies.slice(0,26) as c}<option>{c}</option>{/each}</select>
      <select aria-label={tr("统计周期", "Report period")} value={summary.period} disabled={pending || exporting} onchange={e => change(summary.currency, e.currentTarget.value)}>{#each ["hour","day","month","year"] as p}<option value={p}>{tr("每", "Per ")}{unit(p)}</option>{/each}</select>
      <select aria-label={tr("统计口径", "Report basis")} bind:value={basis} disabled={exporting}><option value="bill">{tr("账单花销", "Bill cost")}</option><option value="actual">{tr("实际花销", "Actual cost")}</option></select>
    </div>
    <p class="hint statistics-scope">{tr("全部有效资产", "All active assets")} · {snapshot.key?.day ?? ""}</p>
    <div class="statistics-total"><span>{tr("实际花销", "Actual cost")}</span><strong>{money(summary.actual)}<small> / {unit(summary.period)}</small></strong><p>{tr("账单花销", "Bill cost")} {money(summary.bill)} <span>·</span> {tr("报销抵扣", "Reimbursed")} {money(summary.reimbursed)}</p></div>
    {#if !available}<p class="hint">{tr("部分资产缺少汇率，请补齐后查看图表和导出报告。", "Add missing exchange rates to view charts and export a report.")}</p>
    {:else if !summary.activeCount}<p class="empty-state">{tr("暂无有效资产", "No active assets yet")}</p>
    {:else}
      <section class="statistics-section"><h3>{tr("分类花销", "By category")}</h3>
        {#each rankedCategories as group}<div class="chart-row"><div><span title={categoryName(group.id)}>{categoryName(group.id)}</span><span>{money(group[basis])} <small>{share(group[basis]).toFixed(1)}%</small></span></div><div class="chart-track"><div class="chart-fill" style:width={`${share(group[basis])}%`}></div></div></div>{/each}
      </section>
      <section class="statistics-section"><h3>{tr("花销最高 TOP 5", "Top 5 assets")}</h3><p class="hint chart-legend">{tr("深灰：实际花销 · 浅灰：报销抵扣", "Dark: actual cost · Light: reimbursed")}</p>
        {#each top as row}<div class="chart-row"><div><span title={row.name}>{row.name}</span><span>{money(row[basis])}</span></div><div class="chart-track"><div class="chart-bill" style:width={`${max ? Number(row.bill) / max * 100 : 0}%`}></div><div class="chart-fill" style:width={`${max ? Number(row.actual) / max * 100 : 0}%`}></div></div></div>{/each}
      </section>
      <section class="statistics-section"><h3>{tr("订阅 / 买断构成", "Subscriptions / lifetime")}</h3><div class="kind-track">{#each summary.kinds as kind}<div class:lifetime={kind.id === "lifetime"} style:width={`${share(kind[basis])}%`}></div>{/each}</div><div class="kind-labels">{#each summary.kinds as kind}<span>{kind.id === "lifetime" ? tr("买断", "Lifetime") : tr("订阅", "Subscriptions")} · {kind.count}<strong>{money(kind[basis])} · {share(kind[basis]).toFixed(1)}%</strong></span>{/each}</div></section>
    {/if}
    <p class="hint">{tr("按当前费用折算，非历史付款流水；买断资产按持有天数摊销。", "Current amortized costs, not payment history. Lifetime purchases are averaged over days owned.")}</p>
  </div>
  {#if error}<div class="dialog-error" role="alert">{error}</div>{/if}
  <div class="dialog-footer"><span class="hint" role="status">{saved ? tr("报告已保存", "Report saved") : `${summary.activeCount} ${tr("项有效资产", "active assets")}`}</span><button class="primary" disabled={pending || exporting || !available || !summary.activeCount} onclick={exportPng}><Icon name="download" size={16}/>{exporting ? tr("正在导出…", "Exporting…") : tr("导出 PNG", "Export PNG")}</button></div>
</dialog>
