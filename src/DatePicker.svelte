<script lang="ts">
  import { tick } from "svelte";
  import { isoDate, parseDate } from "./dates";
  import Icon from "./Icon.svelte";
  let { value, onchange, en = false, required = false, min = "1900-01-01", label }:
    { value: string; onchange: (value: string) => void; en?: boolean; required?: boolean; min?: string; label: string } = $props();
  let expanded = $state(false);
  let year = $state(new Date().getFullYear()), month = $state(new Date().getMonth());
  let field: HTMLDivElement, input: HTMLInputElement;
  let calendar = $state<HTMLDivElement>();
  let left = $state(0), top = $state(0);
  const locale = $derived(en ? "en-US" : "zh-CN");
  const formatted = $derived(value ? (en ? `${value.slice(5, 7)}/${value.slice(8, 10)}/${value.slice(0, 4)}` : value.replaceAll("-", "/")) : "");
  const cells = $derived.by(() => {
    const first = new Date(year, month, 1, 12).getDay();
    return Array.from({ length: 42 }, (_, i) => new Date(year, month, i - first + 1, 12));
  });
  const months = $derived(Array.from({ length: 12 }, (_, i) => new Intl.DateTimeFormat(locale, { month: "long" }).format(new Date(2024, i, 1))));
  const weekdays = $derived(Array.from({ length: 7 }, (_, i) => new Intl.DateTimeFormat(locale, { weekday: "short" }).format(new Date(2024, 0, 7 + i))));
  async function toggle() {
    if (expanded) { expanded = false; return; }
    const selected = parseDate(value) ?? new Date();
    year = selected.getFullYear(); month = selected.getMonth();
    const rect = field.getBoundingClientRect();
    left = Math.max(12, Math.min(rect.left, window.innerWidth - 302));
    top = Math.max(12, Math.min(rect.bottom + 6, window.innerHeight - 350));
    expanded = true;
    await tick();
    (calendar?.querySelector<HTMLButtonElement>("button[aria-pressed='true']") ?? calendar)?.focus();
  }
  function move(delta: number) {
    const date = new Date(year, month + delta, 1);
    if (date.getFullYear() < 1900 || date.getFullYear() > 9999) return;
    year = date.getFullYear(); month = date.getMonth();
  }
  function choose(date: string) {
    onchange(date); expanded = false; input.setCustomValidity(""); input.focus();
  }
  function typed(event: Event) {
    const el = event.currentTarget as HTMLInputElement;
    const parts = el.value.trim().split(/[\/.-]/);
    const iso = /^\d{4}-\d{2}-\d{2}$/.test(el.value) ? el.value : parts.length === 3
      ? (en ? `${parts[2]}-${parts[0].padStart(2, "0")}-${parts[1].padStart(2, "0")}` : `${parts[0]}-${parts[1].padStart(2, "0")}-${parts[2].padStart(2, "0")}`) : "";
    const valid = (!el.value && !required) || (!!parseDate(iso) && iso >= min);
    el.setCustomValidity(valid ? "" : en ? "Enter a valid date (MM/DD/YYYY)." : "请输入有效日期（年/月/日）。");
    if (valid) onchange(el.value ? iso : "");
  }
</script>

<svelte:window onclick={(event) => { if (expanded && !field.contains(event.target as Node) && !calendar?.contains(event.target as Node)) expanded = false; }} onresize={() => expanded = false}/>
<div class="date-field" bind:this={field}>
  <input bind:this={input} type="text" value={formatted} {required} aria-label={label} placeholder={en ? "MM/DD/YYYY" : "YYYY/MM/DD"} onchange={typed} oninput={typed} />
  <button type="button" class="date-toggle" aria-label={en ? `Choose ${label}` : `选择${label}`} aria-expanded={expanded} aria-haspopup="dialog" onclick={toggle}><Icon name="calendar" size={17}/></button>
</div>
{#if expanded}
  <div class="calendar" bind:this={calendar} style:left="{left}px" style:top="{top}px" role="dialog" aria-modal="false" tabindex="-1" aria-label={label} onkeydown={(event) => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); expanded = false; input.focus(); } }}>
    <div class="calendar-heading">
      <button type="button" aria-label={en ? "Previous month" : "上个月"} onclick={() => move(-1)}>‹</button>
      <select aria-label={en ? "Month" : "月份"} bind:value={month}>{#each months as name, i}<option value={i}>{name}</option>{/each}</select>
      <input aria-label={en ? "Year" : "年份"} type="number" min="1900" max="9999" value={year} onchange={(e) => { const y = Number(e.currentTarget.value); if (Number.isInteger(y) && y >= 1900 && y <= 9999) year = y; }}/>
      <button type="button" aria-label={en ? "Next month" : "下个月"} onclick={() => move(1)}>›</button>
    </div>
    <div class="calendar-grid">
      {#each weekdays as day}<span class="weekday">{day}</span>{/each}
      {#each cells as day}{@const iso = isoDate(day)}<button type="button" class:outside={day.getMonth() !== month} class:today={iso === isoDate(new Date())} aria-label={new Intl.DateTimeFormat(locale, { dateStyle: "full" }).format(day)} aria-pressed={iso === value} disabled={iso < min || day.getFullYear() > 9999} onclick={() => choose(iso)}>{day.getDate()}</button>{/each}
    </div>
    <div class="calendar-footer"><button type="button" disabled={isoDate(new Date()) < min} onclick={() => choose(isoDate(new Date()))}>{en ? "Today" : "今天"}</button>{#if !required}<button type="button" onclick={() => choose("")}>{en ? "Clear" : "清除"}</button>{/if}<button type="button" onclick={() => { expanded = false; input.focus(); }}>{en ? "Done" : "完成"}</button></div>
  </div>
{/if}
