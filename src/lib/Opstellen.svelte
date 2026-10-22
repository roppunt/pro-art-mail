<script lang="ts" module>
  import type { Verwijzing } from "./api";

  /** Waarmee het opstelvenster begint (nieuw, antwoord, doorsturen of een bewaard concept). */
  export type Begin = {
    aan: string;
    cc: string;
    onderwerp: string;
    /** HTML onder de ondertekening (bv. het geciteerde bericht), of de hele tekst van een concept. */
    html: string;
    /** true = `html` is een eerder bewaard concept (met ondertekening al erin). */
    isConcept: boolean;
    bijlagen_van: Verwijzing | null;
    overgenomen: string[];
    antwoord_op: Verwijzing | null;
    in_reply_to: string | null;
    references: string | null;
    message_id: string | null;
  };
</script>

<script lang="ts">
  import { open as kiesBestanden } from "@tauri-apps/plugin-dialog";
  import * as api from "./api";
  import type { Account, Ondertekening } from "./api";
  import Pictogram from "./Pictogram.svelte";

  let {
    accounts,
    account,
    begin,
    sluit,
  }: { accounts: Account[]; account: string; begin: Begin; sluit: (melding?: string) => void } = $props();

  // svelte-ignore state_referenced_locally
  let van = $state(account);
  // svelte-ignore state_referenced_locally
  let aan = $state(begin.aan);
  // svelte-ignore state_referenced_locally
  let cc = $state(begin.cc);
  let bcc = $state("");
  // svelte-ignore state_referenced_locally
  let ccTonen = $state(begin.cc !== "");
  // svelte-ignore state_referenced_locally
  let onderwerp = $state(begin.onderwerp);
  let bestanden = $state<string[]>([]);
  // svelte-ignore state_referenced_locally
  let overgenomen = $state<string[]>([...begin.overgenomen]);
  let ondertekeningen = $state<Ondertekening[]>([]);
  let gekozenOndertekening = $state(-1);
  let gewijzigd = $state(false);
  let bezig = $state<"" | "versturen" | "bewaren">("");
  let fout = $state("");
  let suggesties = $state<string[]>([]);
  let suggestieVeld = $state<"aan" | "cc" | "bcc" | null>(null);
  let editor: HTMLDivElement;
  let dialoog: HTMLDialogElement;
  // svelte-ignore state_referenced_locally
  const messageId = begin.message_id ?? `<${crypto.randomUUID()}@${account.split("@")[1] ?? "pro-art-mail"}>`;

  const ONDERTEKENING_SLEUTEL = "pro-art-mail.ondertekening";

  function alsHtml(tekst: string) {
    const d = document.createElement("div");
    d.textContent = tekst;
    return d.innerHTML.replace(/\n/g, "<br>");
  }

  $effect(() => {
    dialoog.showModal();
    editor.innerHTML = begin.isConcept ? begin.html : `<p><br></p><div data-ondertekening></div>${begin.html}`;
    api.ondertekeningen().then((l) => {
      ondertekeningen = l;
      if (begin.isConcept || !l.length) return;
      let laatste = -1;
      try {
        laatste = l.findIndex((o) => o.naam === localStorage.getItem(ONDERTEKENING_SLEUTEL));
      } catch {
        // geen opslag: geen voorkeur
      }
      kiesOndertekening(laatste >= 0 ? laatste : 0, false);
    });
    // Cursor bovenaan, vóór de ondertekening en het citaat.
    const r = document.createRange();
    r.setStart(editor, 0);
    r.collapse(true);
    getSelection()?.removeAllRanges();
    getSelection()?.addRange(r);
    (begin.aan ? editor : document.getElementById("veld-aan"))?.focus();
  });

  function kiesOndertekening(i: number, wijziging = true) {
    gekozenOndertekening = i;
    let blok = editor.querySelector("[data-ondertekening]");
    if (!blok) {
      blok = document.createElement("div");
      blok.setAttribute("data-ondertekening", "");
      editor.insertBefore(blok, editor.children[1] ?? null);
    }
    blok.innerHTML = i >= 0 && ondertekeningen[i] ? `<br>${alsHtml(ondertekeningen[i].tekst)}` : "";
    if (i >= 0 && ondertekeningen[i]) {
      try {
        localStorage.setItem(ONDERTEKENING_SLEUTEL, ondertekeningen[i].naam);
      } catch {
        // niet erg
      }
    }
    if (wijziging) gewijzigd = true;
  }

  const lijst = (s: string) =>
    s
      .split(/[,;]/)
      .map((x) => x.trim())
      .filter(Boolean);

  function concept(): api.Concept {
    return {
      account: van,
      aan: lijst(aan),
      cc: lijst(cc),
      bcc: lijst(bcc),
      onderwerp,
      html: `<div style="font-family: Arial, Helvetica, sans-serif; font-size: 14px;">${editor.innerHTML}</div>`,
      tekst: editor.innerText,
      bestanden,
      bijlagen_van: begin.bijlagen_van,
      zonder_bijlagen: begin.overgenomen.filter((n) => !overgenomen.includes(n)),
      antwoord_op: begin.antwoord_op,
      in_reply_to: begin.in_reply_to,
      references: begin.references,
      message_id: messageId,
    };
  }

  async function verstuur() {
    if (!lijst(aan).length && !lijst(cc).length && !lijst(bcc).length) {
      fout = "Vul bij Aan minstens één mailadres in.";
      return;
    }
    bezig = "versturen";
    fout = "";
    try {
      await api.verstuur(concept());
      dialoog.close();
      sluit("Mail verstuurd");
    } catch (e) {
      const t = String(e);
      // "De mail is verstuurd, maar: …" → wel sluiten, melding tonen.
      if (t.startsWith("De mail is verstuurd")) {
        dialoog.close();
        sluit(t);
      } else fout = t;
    } finally {
      bezig = "";
    }
  }

  async function bewaar(enSluit: boolean) {
    bezig = "bewaren";
    fout = "";
    try {
      await api.bewaarConcept(concept());
      gewijzigd = false;
      if (enSluit) {
        dialoog.close();
        sluit("Bewaard in Concepten");
      }
    } catch (e) {
      fout = `Bewaren als concept mislukt: ${e}`;
    } finally {
      bezig = "";
    }
  }

  /** Sluiten (kruisje of Esc): wat er geschreven is, gaat naar Concepten. */
  function sluitVenster(e?: Event) {
    e?.preventDefault();
    if (bezig) return;
    if (gewijzigd) bewaar(true);
    else {
      dialoog.close();
      sluit();
    }
  }

  function weggooien() {
    dialoog.close();
    sluit(gewijzigd ? "Niet bewaard" : undefined);
  }

  async function voegBestandenToe() {
    const gekozen = await kiesBestanden({ multiple: true, title: "Bijlagen toevoegen" });
    if (!gekozen) return;
    bestanden = [...bestanden, ...(Array.isArray(gekozen) ? gekozen : [gekozen])];
    gewijzigd = true;
  }

  const bestandsnaam = (p: string) => p.split(/[\\/]/).pop() ?? p;

  function opmaak(commando: string) {
    document.execCommand(commando);
    editor.focus();
    gewijzigd = true;
  }

  // Adressen aanvullen: kijkt naar het laatste adres in het veld.
  let zoekTimer: ReturnType<typeof setTimeout>;
  function typAdres(veld: "aan" | "cc" | "bcc", waarde: string) {
    gewijzigd = true;
    clearTimeout(zoekTimer);
    const laatste = waarde.split(/[,;]/).pop()?.trim() ?? "";
    if (laatste.length < 2) {
      suggesties = [];
      return;
    }
    zoekTimer = setTimeout(async () => {
      suggesties = await api.adressen(van, laatste).catch(() => []);
      suggestieVeld = veld;
    }, 120);
  }

  function kiesSuggestie(s: string) {
    const zet = (w: string) => {
      const delen = w.split(/[,;]/);
      delen[delen.length - 1] = ` ${s}`;
      return delen.join(",").replace(/^\s+/, "") + ", ";
    };
    if (suggestieVeld === "aan") aan = zet(aan);
    if (suggestieVeld === "cc") cc = zet(cc);
    if (suggestieVeld === "bcc") bcc = zet(bcc);
    suggesties = [];
    document.getElementById(`veld-${suggestieVeld}`)?.focus();
  }
</script>

{#snippet adresveld(veld: "aan" | "cc" | "bcc", label: string)}
  <div class="rij">
    <label for="veld-{veld}">{label}</label>
    <div class="adres">
      {#if veld === "aan"}
        <input id="veld-aan" type="text" bind:value={aan} oninput={() => typAdres("aan", aan)} onblur={() => setTimeout(() => (suggesties = []), 150)} autocomplete="off" />
      {:else if veld === "cc"}
        <input id="veld-cc" type="text" bind:value={cc} oninput={() => typAdres("cc", cc)} onblur={() => setTimeout(() => (suggesties = []), 150)} autocomplete="off" />
      {:else}
        <input id="veld-bcc" type="text" bind:value={bcc} oninput={() => typAdres("bcc", bcc)} onblur={() => setTimeout(() => (suggesties = []), 150)} autocomplete="off" />
      {/if}
      {#if suggesties.length && suggestieVeld === veld}
        <ul class="suggesties" role="listbox">
          {#each suggesties as s (s)}
            <li><button type="button" onmousedown={(e) => e.preventDefault()} onclick={() => kiesSuggestie(s)}>{s}</button></li>
          {/each}
        </ul>
      {/if}
    </div>
    {#if veld === "aan" && !ccTonen}
      <button type="button" class="link" onclick={() => (ccTonen = true)}>Cc/Bcc</button>
    {/if}
  </div>
{/snippet}

<dialog bind:this={dialoog} class="opstellen" oncancel={sluitVenster} aria-label="Mail opstellen">
  <div class="kop">
    <button type="button" class="verstuur" onclick={verstuur} disabled={!!bezig}>
      {bezig === "versturen" ? "Versturen…" : "Versturen"}
    </button>
    <button type="button" class="tweede" onclick={() => bewaar(false)} disabled={!!bezig}>{bezig === "bewaren" ? "Bewaren…" : "Bewaar als concept"}</button>
    <div class="ruimte"></div>
    <button type="button" class="tweede" onclick={weggooien} disabled={!!bezig}>Weggooien</button>
    <button type="button" class="sluit" aria-label="Sluiten (bewaart als concept)" title="Sluiten (bewaart als concept)" onclick={() => sluitVenster()}>
      <Pictogram naam="sluiten" maat={18} />
    </button>
  </div>

  {#if fout}<p class="fout" role="alert">{fout}</p>{/if}

  <div class="velden">
    {#if accounts.length > 1}
      <div class="rij">
        <label for="veld-van">Van</label>
        <select id="veld-van" bind:value={van}>
          {#each accounts as a (a.email)}<option value={a.email}>{a.naam ? `${a.naam} <${a.email}>` : a.email}</option>{/each}
        </select>
      </div>
    {/if}
    {@render adresveld("aan", "Aan")}
    {#if ccTonen}
      {@render adresveld("cc", "Cc")}
      {@render adresveld("bcc", "Bcc")}
    {/if}
    <div class="rij">
      <label for="veld-onderwerp">Onderwerp</label>
      <input id="veld-onderwerp" type="text" bind:value={onderwerp} oninput={() => (gewijzigd = true)} />
    </div>
  </div>

  <div class="werkbalk" role="toolbar" aria-label="Opmaak">
    <button type="button" onclick={() => opmaak("bold")} aria-label="Vet" title="Vet"><b>B</b></button>
    <button type="button" onclick={() => opmaak("italic")} aria-label="Cursief" title="Cursief"><i>I</i></button>
    <button type="button" onclick={() => opmaak("underline")} aria-label="Onderstrepen" title="Onderstrepen"><u>O</u></button>
    <button type="button" onclick={() => opmaak("insertUnorderedList")} aria-label="Opsomming" title="Opsomming">•&nbsp;lijst</button>
    <button type="button" onclick={() => opmaak("insertOrderedList")} aria-label="Genummerde lijst" title="Genummerde lijst">1.&nbsp;lijst</button>
    <div class="ruimte"></div>
    {#if ondertekeningen.length}
      <label class="ondertekening">
        Ondertekening
        <select value={gekozenOndertekening} onchange={(e) => kiesOndertekening(Number(e.currentTarget.value))}>
          <option value={-1}>Geen</option>
          {#each ondertekeningen as o, i (i)}<option value={i}>{o.naam}</option>{/each}
        </select>
      </label>
    {/if}
    <button type="button" onclick={voegBestandenToe}><Pictogram naam="bijlage" maat={14} /> Bijlage</button>
  </div>

  {#if bestanden.length || overgenomen.length}
    <ul class="bijlagen">
      {#each overgenomen as n (n)}
        <li>
          <Pictogram naam="bijlage" maat={13} />
          {n}
          <button type="button" aria-label="Bijlage {n} weghalen" onclick={() => ((overgenomen = overgenomen.filter((x) => x !== n)), (gewijzigd = true))}>
            <Pictogram naam="sluiten" maat={12} />
          </button>
        </li>
      {/each}
      {#each bestanden as p, i (p + i)}
        <li>
          <Pictogram naam="bijlage" maat={13} />
          {bestandsnaam(p)}
          <button type="button" aria-label="Bijlage {bestandsnaam(p)} weghalen" onclick={() => ((bestanden = bestanden.filter((_, j) => j !== i)), (gewijzigd = true))}>
            <Pictogram naam="sluiten" maat={12} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="editor" contenteditable="true" role="textbox" aria-multiline="true" aria-label="Tekst" tabindex="0" bind:this={editor} oninput={() => (gewijzigd = true)}></div>
</dialog>

<style>
  .opstellen {
    width: min(920px, calc(100vw - 48px));
    height: min(820px, calc(100vh - 48px));
    padding: 0;
    border: 1px solid var(--rand);
    border-radius: 14px;
    background: var(--vlak);
    color: var(--tekst);
    flex-direction: column;
    overflow: hidden;
  }
  .opstellen[open] {
    display: flex;
  }
  .opstellen::backdrop {
    background: rgb(0 0 0 / 0.35);
  }
  .kop {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--rand);
  }
  .ruimte {
    flex: 1;
  }
  .verstuur {
    height: var(--knop-hoog);
    padding: 0 22px;
    border: 0;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-tekst);
    font-weight: 700;
  }
  .tweede {
    height: var(--knop-hoog);
    padding: 0 14px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--knop);
  }
  .sluit {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--tekst-zacht);
  }
  .fout {
    margin: 12px 18px 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--fout-zacht);
    color: var(--fout);
  }
  .velden {
    padding: 6px 18px;
    border-bottom: 1px solid var(--rand);
  }
  .rij {
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid var(--rand-licht);
  }
  .rij:last-child {
    border-bottom: 0;
  }
  .rij label {
    width: 86px;
    flex-shrink: 0;
    color: var(--tekst-zacht);
  }
  .rij input,
  .rij select {
    flex: 1;
    min-width: 0;
    height: 42px;
    border: 0;
    outline: none;
    background: transparent;
  }
  .adres {
    position: relative;
    flex: 1;
    display: flex;
  }
  .suggesties {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 5;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--vlak);
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.15);
  }
  .suggesties button {
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    text-align: left;
  }
  .suggesties button:hover {
    background: var(--gekozen);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--op-accent-zacht);
    text-decoration: underline;
  }
  .werkbalk {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 8px 18px;
    border-bottom: 1px solid var(--rand);
  }
  .werkbalk button {
    min-width: 34px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 0 10px;
    border: 1px solid var(--rand);
    border-radius: 6px;
    background: var(--knop);
  }
  .ondertekening {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--tekst-zacht);
  }
  .ondertekening select {
    height: 32px;
    border: 1px solid var(--rand);
    border-radius: 6px;
    background: var(--knop);
  }
  .bijlagen {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0;
    padding: 10px 18px;
    list-style: none;
    border-bottom: 1px solid var(--rand);
  }
  .bijlagen li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px 4px 10px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
  }
  .bijlagen button {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--tekst-zacht);
  }
  /* Het schrijfvlak is altijd licht, net als de mail die de ontvanger krijgt. */
  .editor {
    flex: 1;
    overflow: auto;
    padding: 20px 24px;
    background: #ffffff;
    color: #1d2433;
    font: 14px/1.6 Arial, Helvetica, sans-serif;
    outline: none;
  }
  .editor :global(blockquote) {
    margin: 0 0 0 4px;
    padding-left: 12px;
    border-left: 2px solid #c8ccd4;
    color: #4a5263;
  }
</style>
