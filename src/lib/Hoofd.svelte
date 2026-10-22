<script lang="ts">
  import * as api from "./api";
  import type { Account, Bericht, Kop, Map } from "./api";
  import Inhoud from "./Inhoud.svelte";
  import { grootte, groep, indeling, initiaal, korteDatum, langeDatum, naamVan, personen } from "./opmaak";

  let { account, afgemeld }: { account: Account; afgemeld: () => void } = $props();

  let lijstMappen = $state<Map[]>([]);
  let huidigeMap = $state("INBOX");
  let koppen = $state<Kop[]>([]);
  let totaal = $state(0);
  let pagina = $state(0);
  let laden = $state(false);
  let gekozen = $state<number | null>(null);
  let open = $state<Bericht | null>(null);
  let ladenBericht = $state(false);
  let fout = $state("");

  const ind = $derived(indeling(lijstMappen));
  const naamHuidig = $derived(lijstMappen.find((m) => m.pad === huidigeMap)?.naam ?? "Postvak IN");
  const ongelezen = $derived(koppen.filter((k) => !k.gelezen).length);
  // Koppen per groep (Vandaag, Gisteren, …), in volgorde.
  const groepen = $derived.by(() => {
    const uit: { titel: string; koppen: Kop[] }[] = [];
    for (const k of koppen) {
      const t = groep(k.datum);
      if (uit.at(-1)?.titel !== t) uit.push({ titel: t, koppen: [] });
      uit.at(-1)!.koppen.push(k);
    }
    return uit;
  });

  async function poging<T>(f: () => Promise<T>): Promise<T | undefined> {
    try {
      fout = "";
      return await f();
    } catch (e) {
      fout = String(e);
    }
  }

  async function kiesMap(pad: string) {
    huidigeMap = pad;
    pagina = 0;
    koppen = [];
    gekozen = null;
    open = null;
    await laadPagina();
  }

  async function laadPagina() {
    laden = true;
    const p = await poging(() => api.berichten(huidigeMap, pagina));
    laden = false;
    if (!p) return;
    totaal = p.totaal;
    koppen = pagina === 0 ? p.berichten : [...koppen, ...p.berichten];
  }

  async function meer() {
    pagina += 1;
    await laadPagina();
  }

  async function kies(k: Kop) {
    gekozen = k.uid;
    ladenBericht = true;
    const b = await poging(() => api.bericht(huidigeMap, k.uid));
    ladenBericht = false;
    if (b && gekozen === k.uid) open = b;
  }

  async function afmelden() {
    await poging(() => api.afmelden());
    afgemeld();
  }

  $effect(() => {
    poging(() => api.mappen()).then((m) => m && (lijstMappen = m));
    laadPagina();
  });
</script>

<div class="app">
  <header>
    <div class="merk"><span class="logo">PA</span> Pro-Art Mail</div>
    <label class="zoek">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="11" cy="11" r="7"></circle><path d="m20 20-3.5-3.5"></path></svg>
      <input type="search" placeholder="Zoeken komt in stap 3" aria-label="Zoeken" disabled />
    </label>
    <div class="ruimte"></div>
    <button class="primair" type="button" disabled title="Komt in stap 4">Nieuwe mail</button>
  </header>

  <div class="kolommen">
    <nav aria-label="Mappen">
      <div class="kopje">{account.email}</div>
      {#each ind.vast as m (m.pad)}
        <button type="button" class="map" class:actief={m.pad === huidigeMap} onclick={() => kiesMap(m.pad)}>
          <span>{m.naam}</span>
          {#if m.pad === huidigeMap && ongelezen}<span class="teller">{ongelezen}</span>{/if}
        </button>
      {/each}
      {#if ind.maanden.length}
        <div class="kopje">Archief per maand</div>
        {#each ind.maanden as m (m.pad)}
          <button type="button" class="map" class:actief={m.pad === huidigeMap} onclick={() => kiesMap(m.pad)}><span>{m.naam}</span></button>
        {/each}
      {/if}
      {#if ind.overig.length}
        <div class="kopje">Andere mappen</div>
        {#each ind.overig as m (m.pad)}
          <button type="button" class="map" class:actief={m.pad === huidigeMap} onclick={() => kiesMap(m.pad)}><span>{m.naam}</span></button>
        {/each}
      {/if}
      <div class="ruimte"></div>
      <button type="button" class="map zacht" onclick={afmelden}>Afmelden op deze pc</button>
    </nav>

    <section class="lijst" aria-label="Berichten">
      <div class="lijstkop">
        <h2>{naamHuidig}</h2>
        <span>{totaal.toLocaleString("nl-NL")} berichten</span>
      </div>
      <div class="rollen">
        {#each groepen as g (g.titel)}
          <div class="groep">{g.titel}</div>
          {#each g.koppen as k (k.uid)}
            <button type="button" class="regel" class:nieuw={!k.gelezen} class:gekozen={k.uid === gekozen} onclick={() => kies(k)}>
              <span class="boven">
                <span class="van">{naamVan(k)}</span>
                <span class="tijd">{korteDatum(k.datum)}</span>
              </span>
              <span class="onderwerp">
                {#if k.beantwoord}<span class="label" title="Beantwoord">↩</span>{/if}
                {#if k.bijlage}<span class="label" title="Bijlage">📎</span>{/if}
                {k.onderwerp || "(geen onderwerp)"}
              </span>
            </button>
          {/each}
        {/each}
        {#if laden}
          <p class="melding">Berichten ophalen…</p>
        {:else if koppen.length < totaal}
          <button type="button" class="meer" onclick={meer}>Oudere berichten laden</button>
        {:else if !koppen.length}
          <p class="melding">Deze map is leeg.</p>
        {/if}
      </div>
    </section>

    <main class="lezen">
      {#if fout}<p class="fout" role="alert">{fout}</p>{/if}
      {#if open}
        <div class="acties">
          <button type="button" disabled title="Komt in stap 4">Beantwoorden</button>
          <button type="button" disabled title="Komt in stap 4">Allen beantwoorden</button>
          <button type="button" disabled title="Komt in stap 4">Doorsturen</button>
          <div class="ruimte"></div>
          <button type="button" class="opbergen" disabled title="Komt in stap 4">Opbergen in maandmap</button>
        </div>
        <article class="bericht">
          <h1>{open.onderwerp || "(geen onderwerp)"}</h1>
          <div class="afzender">
            <span class="bol" aria-hidden="true">{initiaal(naamVan(open.van[0] ?? {}))}</span>
            <div>
              <div class="naam">{personen(open.van)}</div>
              <div class="zacht">aan {personen(open.aan)}{open.cc.length ? ` · cc ${personen(open.cc)}` : ""}</div>
              <div class="zacht">{langeDatum(open.datum)}</div>
            </div>
          </div>
          {#if open.bijlagen.length}
            <ul class="bijlagen">
              {#each open.bijlagen as b, i (i)}
                <li>📎 {b.naam} <span class="zacht">({grootte(b.grootte)})</span></li>
              {/each}
            </ul>
          {/if}
          <Inhoud html={open.html} tekst={open.tekst} />
        </article>
      {:else if ladenBericht}
        <p class="melding">Bericht openen…</p>
      {:else}
        <p class="melding leeg">Kies links een bericht om te lezen.</p>
      {/if}
    </main>
  </div>
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  header {
    height: 60px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 20px;
    background: var(--vlak);
    border-bottom: 1px solid var(--rand);
  }
  .merk {
    width: 228px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 16px;
    font-weight: 600;
  }
  .logo {
    width: 30px;
    height: 30px;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-tekst);
    display: grid;
    place-items: center;
    font-weight: 700;
  }
  .zoek {
    flex: 1;
    max-width: 560px;
    height: 38px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-radius: var(--radius);
    background: var(--invoer);
    color: var(--tekst-zacht);
  }
  .zoek input {
    flex: 1;
    border: 0;
    outline: none;
    background: transparent;
  }
  .ruimte {
    flex: 1;
  }
  .primair {
    height: 38px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-tekst);
    font-weight: 600;
  }
  .kolommen {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  nav {
    width: 248px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 12px;
    border-right: 1px solid var(--rand);
    overflow: auto;
  }
  .kopje {
    padding: 16px 10px 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--tekst-zacht);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .kopje:first-child {
    padding-top: 4px;
    text-transform: none;
    letter-spacing: 0;
  }
  .map {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--tekst-mappen);
    text-align: left;
  }
  .map:hover {
    background: var(--rand-licht);
  }
  .map.actief {
    background: var(--map-actief);
    color: var(--accent);
    font-weight: 600;
  }
  .map.zacht {
    color: var(--tekst-zacht);
    font-size: 13px;
  }
  .teller {
    font-size: 12px;
    font-weight: 600;
  }
  .lijst {
    width: 440px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--vlak);
    border-right: 1px solid var(--rand);
  }
  .lijstkop {
    height: 48px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 18px;
    border-bottom: 1px solid var(--rand-licht);
    color: var(--tekst-zacht);
    font-size: 13px;
  }
  .lijstkop h2 {
    margin: 0;
    font-size: 15px;
    color: var(--tekst);
  }
  .rollen {
    flex: 1;
    overflow: auto;
  }
  .groep {
    padding: 12px 18px 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--tekst-zacht);
  }
  .regel {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 11px 18px;
    border: 0;
    border-bottom: 1px solid var(--rand-licht);
    background: transparent;
    text-align: left;
  }
  .regel:hover {
    background: var(--achtergrond);
  }
  .regel.gekozen {
    background: var(--gekozen);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .boven {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .van,
  .onderwerp {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .van {
    font-weight: 500;
  }
  .nieuw .van {
    font-weight: 700;
  }
  .nieuw .onderwerp {
    font-weight: 600;
  }
  .tijd {
    flex-shrink: 0;
    font-size: 12px;
    color: var(--tekst-zacht);
  }
  .label {
    font-size: 12px;
    color: var(--tekst-zacht);
    margin-right: 4px;
  }
  .meer {
    display: block;
    margin: 16px auto;
    padding: 8px 16px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--vlak);
  }
  .melding {
    padding: 24px;
    color: var(--tekst-zacht);
    text-align: center;
  }
  .lezen {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--vlak);
    overflow: auto;
  }
  .leeg {
    margin: auto;
  }
  .acties {
    position: sticky;
    top: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 12px 28px;
    background: var(--vlak);
    border-bottom: 1px solid var(--rand-licht);
    z-index: 1;
  }
  .acties button {
    height: 36px;
    padding: 0 14px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--vlak);
  }
  .acties .opbergen {
    border: 0;
    background: var(--accent-zacht);
    color: var(--accent);
    font-weight: 600;
  }
  .bericht {
    padding: 28px 40px 48px;
    max-width: 860px;
  }
  h1 {
    margin: 0 0 16px;
    font-size: 22px;
    font-weight: 600;
    line-height: 1.3;
  }
  .afzender {
    display: flex;
    gap: 12px;
    margin-bottom: 20px;
  }
  .bol {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--accent-zacht);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-weight: 600;
  }
  .naam {
    font-weight: 600;
  }
  .zacht {
    color: var(--tekst-zacht);
    font-size: 13px;
  }
  .bijlagen {
    list-style: none;
    margin: 0 0 20px;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .bijlagen li {
    padding: 8px 12px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
  }
  .fout {
    margin: 16px 28px 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--fout-zacht);
    color: var(--fout);
  }
</style>
