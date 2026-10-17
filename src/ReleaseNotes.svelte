<script lang="ts">
  import type { Token } from "marked";
  let { text, openLink }: { text: string; openLink: (url: string) => void } = $props();
  let tokens = $state<Token[]>([]);
  $effect(() => {
    const content = text; let active = true;
    void import("marked").then(({ lexer }) => { if (active) tokens = lexer(content); });
    return () => { active = false; };
  });
</script>

{#snippet render(items: Token[] | undefined)}
  {#each items ?? [] as token}
    {#if token.type === "heading"}
      <svelte:element this={`h${Math.min(token.depth, 4)}`}>{@render render(token.tokens)}</svelte:element>
    {:else if token.type === "paragraph"}<p>{@render render(token.tokens)}</p>
    {:else if token.type === "text"}
      {#if token.tokens}{@render render(token.tokens)}{:else}{token.text}{/if}
    {:else if token.type === "strong"}<strong>{@render render(token.tokens)}</strong>
    {:else if token.type === "em"}<em>{@render render(token.tokens)}</em>
    {:else if token.type === "del"}<del>{@render render(token.tokens)}</del>
    {:else if token.type === "codespan"}<code>{token.text}</code>
    {:else if token.type === "code"}<pre><code>{token.text}</code></pre>
    {:else if token.type === "blockquote"}<blockquote>{@render render(token.tokens)}</blockquote>
    {:else if token.type === "list"}
      {#if token.ordered}<ol start={Number(token.start) || 1}>{#each token.items as item}<li>{@render render(item.tokens)}</li>{/each}</ol>
      {:else}<ul>{#each token.items as item}<li>{@render render(item.tokens)}</li>{/each}</ul>{/if}
    {:else if token.type === "link"}
      {#if /^https:\/\//i.test(token.href)}<a href={token.href} onclick={(event) => { event.preventDefault(); openLink(token.href); }}>{@render render(token.tokens)}</a>
      {:else}{@render render(token.tokens)}{/if}
    {:else if token.type === "image"}{token.text}
    {:else if token.type === "hr"}<hr/>
    {:else if token.type === "br"}<br/>
    {:else if token.type !== "space"}{token.raw}
    {/if}
  {/each}
{/snippet}
<div class="release-markdown">{@render render(tokens)}</div>
