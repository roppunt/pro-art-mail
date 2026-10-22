<script lang="ts">
  import * as api from "./api";
  import type { Account, Bericht, Kop, Map } from "./api";
  import Inhoud from "./Inhoud.svelte";
  import Instellingen from "./Instellingen.svelte";
  import Pictogram from "./Pictogram.svelte";
  import { grootte, groep, indeling, initiaal, korteDatum, langeDatum, naamVan, personen } from "./opmaak";
  import type { ThemaId } from "./thema";

  let {
    account,
    afgemeld,
    thema,
    kiesThema,
  }: { account: Account; afgemeld: () => void; thema: ThemaId; kiesThema: (t: ThemaId) => void } = $props();

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
  let instellingenOpen = $state(false);

  const ind = $derived(indeling(lijstMappen));
  const naamHuidig = $derived(lijstMappen.find((m) => m.pad === huidigeMap)?.naam ?? "Postvak IN");
  const ongelezenHuidig = $derived(lijstMappen.find((m) => m.pad === huidigeMap)?.ongelezen ?? 0);
  // Thema B: de vaste mappen als tabbladen, de rest in "Meer mappen".
  const tabs = $derived(ind.vast.filter((m) => ["inbox", "verzonden", "concepten", "archief"].includes(m.soort)));
  const meerMappen = $derived([...ind.vast.filter((m) => !tabs.includes(m)), ...ind.jaren, ...ind.overig]);
  const meerGekozen = $derived(meerMappen.some((m) => m.pad === huidigeMap));
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

  // Vaste kleur per afzender voor de bolletjes in thema B.
  const BOLKLEUREN = ["#fde7d6", "#e0ecf8", "#e3f1e7", "#efe6f7", "#f6efd9", "#dff3f1"];
  function bolkleur(s: string) {
    let h = 0;
    for (const c of s) h = (h * 31 + c.charCodeAt(0)) >>> 0;
    return BOLKLEUREN[h % BOLKLEUREN.length];
  }
  // Kleurstip per map in thema C.
  const STIPPEN: Record<string, string> = {
    inbox: "#f2b35b",
    concepten: "#7fc8a9",
    verzonden: "#5fb3e4",
    archief: "#c49bd9",
    spam: "#8e8a83",
    prullenbak: "#e4572e",
  };

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
    instellingenOpen = false;
    await poging(() => api.afmelden());
    afgemeld();
  }

  $effect(() => {
    poging(() => api.mappen()).then((m) => m && (lijstMappen = m));
    laadPagina();
  });
</script>

{#snippet zoekveld()}
  <label class="zoek">
    <Pictogram naam="zoek" maat={thema === "b" ? 20 : 16} />
    <input type="search" placeholder="Zoeken komt in stap 3" aria-label="Zoeken" disabled />
  </label>
{/snippet}

{#snippet mapknop(m: Map)}
  <button type="button" class="map" class:actief={m.pad === huidigeMap} onclick={() => kiesMap(m.pad)}>
    {#if thema === "c"}<span class="stip" style="background: {STIPPEN[m.soort] ?? '#5b5f66'}" aria-hidden="true"></span>{/if}
    <span class="mapnaam">{m.naam}</span>
    {#if m.ongelezen}<span class="teller">{m.ongelezen}</span>{/if}
  </button>
{/snippet}

{#snippet acties()}
  <button type="button" class:primair={thema === "b"} disabled title="Komt in stap 4">Beantwoorden</button>
  {#if thema !== "b"}<button type="button" disabled title="Komt in stap 4">Allen beantwoorden</button>{/if}
  <button type="button" disabled title="Komt in stap 4">Doorsturen</button>
  <div class="ruimte"></div>
  <button type="button" class="zacht-accent" disabled title="Komt in stap 4"><Pictogram naam="archief" /> Archiveren</button>
  <button type="button" disabled title="Komt in stap 4" aria-label="Verwijderen"><Pictogram naam="prullenbak" /></button>
{/snippet}

<div class="app thema-{thema}">
  {#if thema !== "c"}
    <header>
      <div class="merk"><span class="logo">PA</span> Pro-Art Mail</div>
      {#if thema === "b"}
        <nav class="tabs" aria-label="Mappen">
          {#each tabs as m (m.pad)}
            <button type="button" class="tab" class:actief={m.pad === huidigeMap} onclick={() => kiesMap(m.pad)}>{m.naam}</button>
          {/each}
          <label class="tab meer" class:actief={meerGekozen}>
            <span class="verborgen">Meer mappen</span>
            <span class="pijl" aria-hidden="true"><Pictogram naam="omlaag" /></span>
            <select value={meerGekozen ? huidigeMap : ""} onchange={(e) => kiesMap(e.currentTarget.value)}>
              <option value="" disabled>Meer mappen</option>
              {#each meerMappen as m (m.pad)}
                <option value={m.pad}>{ind.jaren.includes(m) ? `Archief ${m.naam}` : m.naam}</option>
              {/each}
            </select>
          </label>
        </nav>
      {/if}
      {#if thema === "a"}{@render zoekveld()}{/if}
      <div class="ruimte"></div>
      {#if thema === "b"}{@render zoekveld()}{/if}
      <button type="button" class="pictoknop" aria-label="Instellingen" title="Instellingen" onclick={() => (instellingenOpen = true)}>
        <Pictogram naam="instellingen" maat={thema === "b" ? 22 : 18} />
      </button>
      <button class="primair" type="button" disabled title="Komt in stap 4"><Pictogram naam="plus" /> Nieuwe mail</button>
    </header>
  {/if}

  <div class="kolommen">
    {#if thema !== "b"}
      <nav class="mappen" aria-label="Mappen">
        {#if thema === "c"}
          <div class="merk-c">
            <span class="verf" aria-hidden="true"><span></span><span></span><span></span></span> Pro-Art Mail
          </div>
          <button class="primair" type="button" disabled title="Komt in stap 4">Nieuwe mail</button>
        {/if}
        <div class="kopje eerste">{account.email}</div>
        {#each ind.vast as m (m.pad)}{@render mapknop(m)}{/each}
        {#if ind.jaren.length}
          <div class="kopje">Archief per jaar</div>
          {#each ind.jaren as m (m.pad)}{@render mapknop(m)}{/each}
        {/if}
        {#if ind.overig.length}
          <div class="kopje">Andere mappen</div>
          {#each ind.overig as m (m.pad)}{@render mapknop(m)}{/each}
        {/if}
        <div class="ruimte"></div>
        {#if thema === "c"}
          <button type="button" class="map" onclick={() => (instellingenOpen = true)}>
            <Pictogram naam="instellingen" /> <span class="mapnaam">Instellingen</span>
          </button>
        {/if}
      </nav>
    {/if}

    <section class="lijst" aria-label="Berichten">
      {#if thema === "c"}<div class="zoek-c">{@render zoekveld()}</div>{/if}
      <div class="lijstkop">
        <h2>{naamHuidig}</h2>
        <span>{ongelezenHuidig ? `${ongelezenHuidig} ongelezen · ` : ""}{totaal.toLocaleString("nl-NL")} berichten</span>
      </div>
      <div class="rollen">
        {#each groepen as g (g.titel)}
          <div class="groep">{g.titel}</div>
          {#each g.koppen as k (k.uid)}
            <button type="button" class="regel" class:nieuw={!k.gelezen} class:gekozen={k.uid === gekozen} onclick={() => kies(k)}>
              {#if thema === "b"}
                <span class="bol" style="background: {bolkleur(naamVan(k))}" aria-hidden="true">{initiaal(naamVan(k))}</span>
              {/if}
              <span class="regeltekst">
                <span class="boven">
                  {#if thema === "c"}<span class="nieuwstip" aria-hidden="true"></span>{/if}
                  <span class="van">{naamVan(k)}</span>
                  <span class="tijd">{korteDatum(k.datum)}</span>
                </span>
                <span class="onderwerp">
                  {#if k.beantwoord}<span class="label" title="Beantwoord"><Pictogram naam="beantwoord" maat={13} /></span>{/if}
                  {#if k.bijlage}<span class="label" title="Bijlage"><Pictogram naam="bijlage" maat={13} /></span>{/if}
                  {k.onderwerp || "(geen onderwerp)"}
                </span>
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
        {#if thema !== "b"}<div class="acties boven">{@render acties()}</div>{/if}
        <div class="rol-lezen">
          <article class="bericht">
            <h1>{open.onderwerp || "(geen onderwerp)"}</h1>
            <div class="afzender">
              <span class="bol groot" style={thema === "b" ? `background: ${bolkleur(naamVan(open.van[0] ?? {}))}` : ""} aria-hidden="true">
                {initiaal(naamVan(open.van[0] ?? {}))}
              </span>
              <div>
                <div class="naam">{personen(open.van)}</div>
                <div class="zacht">aan {personen(open.aan)}{open.cc.length ? ` · cc ${personen(open.cc)}` : ""}</div>
                <div class="zacht">{langeDatum(open.datum)}</div>
              </div>
            </div>
            {#if open.bijlagen.length}
              <ul class="bijlagen">
                {#each open.bijlagen as b, i (i)}
                  <li><Pictogram naam="bijlage" maat={14} /> {b.naam} <span class="zacht">({grootte(b.grootte)})</span></li>
                {/each}
              </ul>
            {/if}
            <div class="papier"><Inhoud html={open.html} tekst={open.tekst} /></div>
          </article>
        </div>
        {#if thema === "b"}<div class="acties onder">{@render acties()}</div>{/if}
      {:else if ladenBericht}
        <p class="melding">Bericht openen…</p>
      {:else}
        <p class="melding leeg">Kies een bericht om te lezen.</p>
      {/if}
    </main>
  </div>
</div>

{#if instellingenOpen}
  <Instellingen {thema} kies={kiesThema} email={account.email} {afmelden} sluit={() => (instellingenOpen = false)} />
{/if}

<style>
  /* ── Gemeenschappelijk (thema A is de basis) ───────────────────────── */
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--achtergrond);
  }
  .ruimte {
    flex: 1;
  }
  header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 20px;
    min-height: 60px;
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
    font-size: 14px;
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
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
  }
  .primair {
    height: 38px;
    flex-shrink: 0;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-tekst);
    font-weight: 600;
  }
  .pictoknop {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--tekst-zacht);
  }
  .pictoknop:hover {
    background: var(--rand-licht);
  }
  .kolommen {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .mappen {
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
  .kopje.eerste {
    padding-top: 4px;
    text-transform: none;
    letter-spacing: 0;
  }
  .map {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--tekst-mappen);
    text-align: left;
  }
  .mapnaam {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .map:hover {
    background: var(--rand-licht);
  }
  .map.actief {
    background: var(--map-actief);
    color: var(--op-accent-zacht);
    font-weight: 600;
  }
  .teller {
    font-size: 12px;
    font-weight: 600;
  }
  .lijst {
    width: clamp(320px, 32vw, 440px);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--vlak);
    border-right: 1px solid var(--rand);
  }
  .lijstkop {
    min-height: 48px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
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
    align-items: center;
    gap: 12px;
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
  .regeltekst {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .boven {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .van,
  .onderwerp {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .van {
    flex: 1;
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
    display: inline-flex;
    vertical-align: -1px;
    margin-right: 4px;
    color: var(--tekst-zacht);
  }
  .meer {
    display: block;
    margin: 16px auto;
    padding: 8px 16px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--knop);
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
    background: var(--vlak-lezen);
  }
  .leeg {
    margin: auto;
  }
  .rol-lezen {
    flex: 1;
    overflow: auto;
  }
  .acties {
    flex-shrink: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 12px 28px;
    background: var(--vlak-lezen);
  }
  .acties.boven {
    border-bottom: 1px solid var(--rand-licht);
  }
  .acties button {
    height: var(--knop-hoog);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 14px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--knop);
  }
  .acties .primair {
    border: 0;
    background: var(--accent);
    color: var(--accent-tekst);
  }
  .acties .zacht-accent {
    border: 0;
    background: var(--accent-zacht);
    color: var(--op-accent-zacht);
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
    color: var(--op-accent-zacht);
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
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
  }
  .papier {
    background: var(--papier);
    border-radius: var(--radius);
  }
  .fout {
    margin: 16px 28px 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--fout-zacht);
    color: var(--fout);
  }
  .verborgen {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  /* ── B: groot en eenvoudig ─────────────────────────────────────────── */
  .thema-b header {
    gap: 12px;
    padding: 14px 28px;
    border-bottom-width: 2px;
  }
  .thema-b .merk {
    width: auto;
    white-space: nowrap;
    margin-right: 12px;
    font-size: 22px;
    font-weight: 700;
  }
  .thema-b .logo {
    display: none;
  }
  .tabs {
    display: flex;
    gap: 4px;
    min-width: 0;
  }
  .tab {
    position: relative;
    flex-shrink: 0;
    padding: 10px 16px;
    white-space: nowrap;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--tekst-mappen);
  }
  .tab.actief {
    background: var(--map-actief);
    color: var(--op-accent-zacht);
    font-weight: 700;
  }
  .tab.meer {
    padding: 0;
  }
  .tab.meer select {
    appearance: none;
    height: 100%;
    padding: 10px 40px 10px 16px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: inherit;
    font-weight: inherit;
    cursor: pointer;
  }
  .tab.meer .pijl {
    position: absolute;
    right: 14px;
    top: 50%;
    translate: 0 -50%;
    display: flex;
    pointer-events: none;
  }
  .thema-b .zoek {
    flex: 1 1 200px;
    min-width: 150px;
    max-width: 340px;
    height: 48px;
    border: 2px solid var(--rand);
  }
  .thema-b .primair {
    flex-shrink: 0;
    white-space: nowrap;
    height: 48px;
    padding: 0 22px;
    font-weight: 700;
  }
  .thema-b .lijst {
    width: clamp(360px, 36vw, 540px);
    border-right-width: 2px;
  }
  .thema-b .lijstkop {
    min-height: 60px;
    padding: 0 28px;
    font-size: 16px;
    border: 0;
  }
  .thema-b .lijstkop h2 {
    font-size: 20px;
  }
  .thema-b .rollen {
    padding: 0 14px 14px;
  }
  .thema-b .groep {
    padding: 14px 14px 6px;
    font-size: 14px;
  }
  .thema-b .regel {
    gap: 14px;
    padding: 14px;
    border: 0;
    border-radius: 14px;
    margin-bottom: 4px;
  }
  .thema-b .regel.gekozen {
    box-shadow: none;
    outline: 2px solid var(--accent);
  }
  .thema-b .bol {
    width: 44px;
    height: 44px;
    font-size: 18px;
    font-weight: 700;
    color: #292524;
  }
  .thema-b .bol.groot {
    width: 48px;
    height: 48px;
    font-size: 20px;
  }
  .thema-b .tijd {
    font-size: 15px;
  }
  .thema-b .bericht {
    margin: 32px 40px;
    padding: 32px 36px;
    max-width: 760px;
    background: var(--vlak);
    border: 2px solid var(--rand);
    border-radius: 16px;
  }
  .thema-b h1 {
    font-size: 26px;
    font-weight: 700;
  }
  .thema-b .zacht {
    font-size: 15px;
  }
  .thema-b .acties.onder {
    padding: 16px 40px;
    gap: 12px;
    background: var(--vlak);
    border-top: 2px solid var(--rand);
  }
  .thema-b .acties button {
    padding: 0 24px;
    border-width: 2px;
    font-size: 18px;
  }

  /* ── C: atelier, donker ────────────────────────────────────────────── */
  .thema-c .mappen {
    width: 232px;
    padding: 22px 14px;
    border: 0;
  }
  .merk-c {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 20px;
    font-size: 17px;
    font-weight: 700;
  }
  .verf {
    display: flex;
    gap: 3px;
  }
  .verf span {
    width: 8px;
    height: 22px;
    border-radius: 4px;
  }
  .verf span:nth-child(1) {
    background: #e4572e;
  }
  .verf span:nth-child(2) {
    background: #f2b35b;
  }
  .verf span:nth-child(3) {
    background: #3c8dbc;
  }
  .thema-c .mappen .primair {
    margin: 0 4px 12px;
    height: 42px;
    font-weight: 700;
  }
  .stip {
    width: 10px;
    height: 10px;
    flex-shrink: 0;
    border-radius: 3px;
  }
  .thema-c .lijst {
    width: clamp(320px, 30vw, 420px);
    margin: 12px 0;
    border: 0;
    border-radius: 16px 0 0 16px;
  }
  .zoek-c {
    padding: 18px 20px 6px;
  }
  .zoek-c .zoek {
    max-width: none;
    height: 40px;
  }
  .thema-c .lijstkop {
    border: 0;
    padding: 0 20px;
  }
  .thema-c .rollen {
    padding: 0 10px 10px;
  }
  .thema-c .groep {
    padding: 12px 12px 4px;
  }
  .thema-c .regel {
    padding: 12px;
    border: 0;
    border-radius: 12px;
    margin-bottom: 2px;
  }
  .thema-c .regel:hover {
    background: var(--rand-licht);
  }
  .thema-c .regel.gekozen {
    box-shadow: none;
  }
  .nieuwstip {
    width: 7px;
    height: 7px;
    flex-shrink: 0;
    border-radius: 50%;
    background: transparent;
  }
  .thema-c .nieuw .nieuwstip {
    background: var(--accent);
  }
  .thema-c .onderwerp {
    padding-left: 15px;
  }
  .thema-c .nieuw .van {
    color: #ffffff;
  }
  .thema-c .lezen {
    margin: 12px 12px 12px 0;
    border-radius: 0 16px 16px 0;
  }
  .thema-c .acties.boven {
    border-color: var(--rand);
    border-radius: 0 16px 0 0;
  }
  .thema-c .acties button {
    border: 0;
  }
  .thema-c .acties .zacht-accent {
    border: 1px solid var(--accent);
    background: transparent;
  }
  .thema-c h1 {
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .thema-c .bol {
    background: #3c8dbc;
    color: #0d1b24;
    font-weight: 700;
  }
  .thema-c .papier {
    padding: 24px 28px;
  }
</style>
