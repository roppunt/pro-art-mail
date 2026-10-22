<script lang="ts">
  // Kleine lijnpictogrammen; kleur volgt de tekstkleur.
  let {
    naam,
    maat = 16,
    gevuld = false,
  }: {
    naam: "zoek" | "plus" | "bijlage" | "beantwoord" | "instellingen" | "prullenbak" | "archief" | "sluiten" | "omlaag" | "ster";
    maat?: number;
    gevuld?: boolean;
  } = $props();
</script>

<svg width={maat} height={maat} viewBox="0 0 24 24" fill={gevuld ? "currentColor" : "none"} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if naam === "zoek"}
    <circle cx="11" cy="11" r="7"></circle><path d="m20 20-3.5-3.5"></path>
  {:else if naam === "plus"}
    <path d="M12 5v14M5 12h14"></path>
  {:else if naam === "bijlage"}
    <path d="m21 11-8.5 8.5a5 5 0 0 1-7-7L14 4a3.5 3.5 0 0 1 5 5l-8.5 8.5a2 2 0 0 1-3-3L15 7"></path>
  {:else if naam === "beantwoord"}
    <path d="M9 14 4 9l5-5"></path><path d="M4 9h11a5 5 0 0 1 5 5v6"></path>
  {:else if naam === "instellingen"}
    <circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"></path>
  {:else if naam === "prullenbak"}
    <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"></path>
  {:else if naam === "archief"}
    <path d="M3 4h18v4H3z"></path><path d="M5 8v12h14V8M10 12h4"></path>
  {:else if naam === "sluiten"}
    <path d="M18 6 6 18M6 6l12 12"></path>
  {:else if naam === "ster"}
    <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.3l-5.6 2.9 1.1-6.2L3 9.6l6.2-.9z"></path>
  {:else if naam === "omlaag"}
    <path d="m6 9 6 6 6-6"></path>
  {/if}
</svg>
