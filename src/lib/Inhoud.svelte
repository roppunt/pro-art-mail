<script lang="ts">
  // Toont de (al opgeschoonde) HTML van een bericht in een eigen shadow root, zodat de opmaak
  // van een nieuwsbrief de app niet beïnvloedt. Links gaan open in de gewone browser.
  import { openUrl } from "@tauri-apps/plugin-opener";

  let { html, tekst }: { html: string; tekst: string } = $props();
  let houder: HTMLDivElement;

  $effect(() => {
    const wortel = houder.shadowRoot ?? houder.attachShadow({ mode: "open" });
    const basis = `<style>:host{display:block;color:#1d2433;font:15px/1.6 system-ui,sans-serif;overflow-wrap:anywhere}
      img{max-width:100%;height:auto} table{max-width:100%} pre.tekst{white-space:pre-wrap;font:inherit;margin:0}</style>`;
    if (html) {
      wortel.innerHTML = basis + html;
    } else {
      wortel.innerHTML = basis;
      const pre = document.createElement("pre");
      pre.className = "tekst";
      pre.textContent = tekst; // als tekst, nooit als HTML
      wortel.appendChild(pre);
    }
  });

  function klik(e: MouseEvent) {
    const a = (e.composedPath() as HTMLElement[]).find((el) => el.tagName === "A") as HTMLAnchorElement | undefined;
    if (!a) return;
    e.preventDefault();
    if (/^(https?|mailto):/i.test(a.href)) openUrl(a.href);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div bind:this={houder} onclick={klik}></div>
