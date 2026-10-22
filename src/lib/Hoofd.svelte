<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { untrack } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import * as api from "./api";
  import type { Account, Bericht, Gebeurtenis, Kop, Map, Voortgang } from "./api";
  import Inhoud from "./Inhoud.svelte";
  import Instellingen from "./Instellingen.svelte";
  import Opstellen, { type Begin } from "./Opstellen.svelte";
  import Pictogram from "./Pictogram.svelte";
  import { grootte, groep, indeling, initiaal, korteDatum, langeDatum, naamVan, personen } from "./opmaak";
  import type { ThemaId } from "./thema";

  let {
    accounts,
    account,
    wisselAccount,
    accountsGewijzigd,
    thema,
    kiesThema,
  }: {
    accounts: Account[];
    account: string;
    wisselAccount: (email: string) => void;
    accountsGewijzigd: (lijst: Account[], kies?: string) => void;
    thema: ThemaId;
    kiesThema: (t: ThemaId) => void;
  } = $props();

  let lijstMappen = $state<Map[]>([]);
  let huidigeMap = $state("INBOX");
  let koppen = $state<Kop[]>([]);
  let totaal = $state(0);
  let klaar = $state(true);
  let laden = $state(false);
  let voortgang = $state<Voortgang | null>(null);
  let verbindingsfout = $state("");
  // Zoeken: null = gewone lijst; anders de resultaten.
  let zoekterm = $state("");
  let resultaten = $state<Kop[] | null>(null);
  let inTekstGezocht = $state(false);
  let zoekBezig = $state(false);
  let rollen = $state<HTMLDivElement>();
  const STUK = 100;
  /** Schijnmap met alle mails met een ster, uit alle mappen. */
  const STER = "__ster__";
  let aantalSter = $state(0);
  let gekozen = $state<number | null>(null);
  let open = $state<Bericht | null>(null);
  let ladenBericht = $state(false);
  let fout = $state("");
  let instellingenOpen = $state(false);
  let openKop = $state<Kop | null>(null);
  let opstellen = $state<Begin | null>(null);
  let melding = $state("");
  let bezigMetActie = $state(false);

  const ind = $derived(indeling(lijstMappen));
  const naamHuidig = $derived(huidigeMap === STER ? "Met ster" : (lijstMappen.find((m) => m.pad === huidigeMap)?.naam ?? "Postvak IN"));
  const ongelezenHuidig = $derived(lijstMappen.find((m) => m.pad === huidigeMap)?.ongelezen ?? 0);
  const mapNaam = (pad: string) => lijstMappen.find((m) => m.pad === pad)?.naam ?? pad;
  // Bij Concepten het totaal (die zijn nooit "ongelezen"), elders het aantal ongelezen.
  const teller = (m: Map) => (m.soort === "concepten" ? m.aantal : m.ongelezen);
  const soortVan = (pad: string) => lijstMappen.find((m) => m.pad === pad)?.soort;
  const zichtbaar = $derived(resultaten ?? koppen);
  const bezigMetKlaarzetten = $derived(!resultaten && voortgang?.map === huidigeMap && voortgang.gedaan < voortgang.totaal ? voortgang : null);
  // Thema B: de vaste mappen als tabbladen, de rest in "Meer mappen".
  const tabs = $derived(ind.vast.filter((m) => ["inbox", "verzonden", "concepten", "archief"].includes(m.soort)));
  const meerMappen = $derived([...ind.vast.filter((m) => !tabs.includes(m)), ...ind.jaren, ...ind.overig]);
  const meerGekozen = $derived(meerMappen.some((m) => m.pad === huidigeMap));
  // Koppen per groep (Vandaag, Gisteren, …), in volgorde.
  const groepen = $derived.by(() => {
    const uit: { titel: string; koppen: Kop[] }[] = [];
    for (const k of zichtbaar) {
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
    wisZoeken();
    huidigeMap = pad;
    koppen = [];
    gekozen = null;
    open = null;
    rollen?.scrollTo(0, 0);
    await herlaad(STUK);
  }

  /** Haalt de bovenste `aantal` berichten opnieuw uit de lokale kopie (direct). */
  async function herlaad(aantal: number) {
    const map = huidigeMap;
    if (map === STER) {
      const l = await poging(() => api.metSter(account));
      if (!l || map !== huidigeMap) return;
      koppen = l;
      totaal = aantalSter = l.length;
      klaar = true;
      return;
    }
    const p = await poging(() => api.berichten(account, map, 0, aantal));
    if (!p || map !== huidigeMap) return;
    totaal = p.totaal;
    klaar = p.klaar;
    koppen = p.berichten;
  }

  async function meer() {
    if (laden || resultaten || huidigeMap === STER || koppen.length >= totaal) return;
    laden = true;
    const map = huidigeMap;
    const p = await poging(() => api.berichten(account, map, koppen.length, STUK));
    laden = false;
    if (!p || map !== huidigeMap) return;
    totaal = p.totaal;
    koppen = [...koppen, ...p.berichten];
  }

  /** Vloeiend scrollen: bijna onderaan → volgende stuk erbij. */
  function gescrold(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 800) meer();
  }

  let zoekTimer: ReturnType<typeof setTimeout>;
  function typ() {
    clearTimeout(zoekTimer);
    inTekstGezocht = false;
    if (!zoekterm.trim()) return wisZoeken();
    zoekTimer = setTimeout(async () => {
      const term = zoekterm;
      const r = await poging(() => api.zoek(account, term));
      if (r && term === zoekterm) resultaten = r;
    }, 150);
  }

  async function zoekInTekst() {
    const term = zoekterm;
    zoekBezig = true;
    const r = await poging(() => api.zoekInTekst(account, huidigeMap, term));
    zoekBezig = false;
    if (r && term === zoekterm) {
      resultaten = r;
      inTekstGezocht = true;
    }
  }

  function wisZoeken() {
    clearTimeout(zoekTimer);
    zoekterm = "";
    resultaten = null;
    inTekstGezocht = false;
  }

  async function kies(k: Kop) {
    gekozen = k.uid;
    ladenBericht = true;
    // Openen = gelezen, behalve in Concepten.
    const markeer = !k.gelezen && soortVan(k.map) !== "concepten";
    const b = await poging(() => api.bericht(account, k.map, k.uid, markeer));
    ladenBericht = false;
    if (b && gekozen === k.uid) {
      open = b;
      openKop = k;
      if (markeer) zetGelezen(k, true);
    }
  }

  async function telSterren() {
    const l = await api.metSter(account).catch(() => null);
    if (l) aantalSter = l.length;
  }

  /** Ster aan/uit. Meteen zichtbaar; lukt het op de server niet, dan terug. */
  async function wisselSter(k: Kop, e?: Event) {
    e?.stopPropagation();
    const aan = !k.gemarkeerd;
    k.gemarkeerd = aan;
    aantalSter += aan ? 1 : -1;
    const ok = await poging(async () => (await api.ster(account, k.map, [k.uid], aan), true));
    if (!ok) {
      k.gemarkeerd = !aan;
      aantalSter += aan ? -1 : 1;
    }
  }

  function zetGelezen(k: Kop, gelezen: boolean) {
    for (const lijst of [koppen, resultaten ?? []]) {
      const x = lijst.find((y) => y.uid === k.uid && y.map === k.map);
      if (x) x.gelezen = gelezen;
    }
  }

  function toon(tekst: string) {
    melding = tekst;
    setTimeout(() => melding === tekst && (melding = ""), 5000);
  }

  /** Na verplaatsen of verwijderen: het volgende bericht in de lijst openen. */
  function volgende(weg: Kop) {
    const lijst = zichtbaar;
    const i = lijst.findIndex((k) => k.uid === weg.uid && k.map === weg.map);
    const filter = (l: Kop[]) => l.filter((k) => !(k.uid === weg.uid && k.map === weg.map));
    koppen = filter(koppen);
    if (resultaten) resultaten = filter(resultaten);
    open = null;
    openKop = null;
    const daarna = filter(lijst)[i];
    if (daarna) kies(daarna);
    else gekozen = null;
  }

  async function actie(f: () => Promise<unknown>, k: Kop, klaar: string) {
    bezigMetActie = true;
    const ok = await poging(async () => (await f(), true));
    bezigMetActie = false;
    if (ok) {
      volgende(k);
      toon(klaar);
    }
  }

  const archiveren = (k: Kop) =>
    actie(() => api.archiveer(account, k.map, [[k.uid, k.datum]]), k, `Gearchiveerd in ${new Date(k.datum * 1000).getFullYear() || "Archief"}`);
  const verwijderen = (k: Kop) => actie(() => api.verwijder(account, k.map, [k.uid]), k, "Naar de prullenbak verplaatst");

  async function ongelezen(k: Kop) {
    if (await poging(async () => (await api.markeer(account, k.map, [k.uid], false), true))) {
      zetGelezen(k, false);
      open = null;
      openKop = null;
      gekozen = null;
    }
  }

  async function bijlageOpenen(k: Kop, i: number) {
    await poging(() => api.bijlageOpenen(account, k.map, k.uid, i));
  }

  async function bijlageOpslaan(k: Kop, i: number, naam: string) {
    const pad = await save({ defaultPath: naam, title: "Bijlage opslaan" });
    if (pad && (await poging(async () => (await api.bijlageOpslaan(account, k.map, k.uid, i, pad), true)))) toon(`${naam} opgeslagen`);
  }

  // ── Schrijven ──────────────────────────────────────────────────────────
  const esc = (t: string) => {
    const d = document.createElement("div");
    d.textContent = t;
    return d.innerHTML;
  };
  /** HTML van een bericht zonder <style>-blokken, zodat het citaat de app niet beïnvloedt. */
  function zonderStijl(html: string) {
    const doc = new DOMParser().parseFromString(html, "text/html");
    doc.querySelectorAll("style, meta, title, link").forEach((e) => e.remove());
    return doc.body.innerHTML;
  }
  const inhoudAlsHtml = (b: Bericht) => (b.html ? zonderStijl(b.html) : esc(b.tekst).replace(/\n/g, "<br>"));
  const adresTekst = (p: { naam: string; adres: string }) => (p.naam && p.naam !== p.adres ? `${p.naam} <${p.adres}>` : p.adres);
  const metVoorvoegsel = (onderwerp: string, voor: string, al: RegExp) => (al.test(onderwerp) ? onderwerp : `${voor} ${onderwerp}`);

  function leeg(): Begin {
    return { aan: "", cc: "", onderwerp: "", html: "", isConcept: false, bijlagen_van: null, overgenomen: [], antwoord_op: null, in_reply_to: null, references: null, message_id: null };
  }

  function beantwoord(b: Bericht, k: Kop, allen: boolean) {
    const eigen = new Set(accounts.map((a) => a.email.toLowerCase()));
    const naar = (b.antwoord_aan.length ? b.antwoord_aan : b.van).filter((p) => !eigen.has(p.adres.toLowerCase()) || !allen);
    const gezien = new Set(naar.map((p) => p.adres.toLowerCase()));
    const kopie = allen
      ? [...b.aan, ...b.cc].filter((p) => {
          const a = p.adres.toLowerCase();
          if (eigen.has(a) || gezien.has(a)) return false;
          gezien.add(a);
          return true;
        })
      : [];
    opstellen = {
      ...leeg(),
      aan: naar.map(adresTekst).join(", "),
      cc: kopie.map(adresTekst).join(", "),
      onderwerp: metVoorvoegsel(b.onderwerp, "Re:", /^\s*(re|antw|aw)\s*:/i),
      html: `<br><div>Op ${esc(langeDatum(b.datum))} schreef ${esc(personen(b.van))}:</div><blockquote style="margin:0 0 0 4px;padding-left:12px;border-left:2px solid #c8ccd4">${inhoudAlsHtml(b)}</blockquote>`,
      antwoord_op: { map: k.map, uid: k.uid },
      in_reply_to: b.message_id || null,
      references: b.referenties || null,
    };
  }

  function doorsturen(b: Bericht, k: Kop) {
    const kop = [
      "---------- Doorgestuurd bericht ----------",
      `Van: ${personen(b.van)}`,
      `Datum: ${langeDatum(b.datum)}`,
      `Onderwerp: ${b.onderwerp}`,
      `Aan: ${personen(b.aan)}`,
      ...(b.cc.length ? [`Cc: ${personen(b.cc)}`] : []),
    ];
    opstellen = {
      ...leeg(),
      onderwerp: metVoorvoegsel(b.onderwerp, "Fwd:", /^\s*(fwd?|doorst|wg)\s*:/i),
      html: `<br><div>${kop.map(esc).join("<br>")}</div><br>${inhoudAlsHtml(b)}`,
      bijlagen_van: { map: k.map, uid: k.uid },
      overgenomen: b.bijlagen.map((x) => x.naam),
    };
  }

  function verderSchrijven(b: Bericht, k: Kop) {
    opstellen = {
      ...leeg(),
      aan: b.aan.map(adresTekst).join(", "),
      cc: b.cc.map(adresTekst).join(", "),
      onderwerp: b.onderwerp,
      html: b.html ? zonderStijl(b.html) : esc(b.tekst).replace(/\n/g, "<br>"),
      isConcept: true,
      bijlagen_van: b.bijlagen.length ? { map: k.map, uid: k.uid } : null,
      overgenomen: b.bijlagen.map((x) => x.naam),
      message_id: b.message_id || null,
    };
  }

  function opstellenGesloten(tekst?: string) {
    opstellen = null;
    if (tekst) toon(tekst);
  }

  // Aantallen per map bijwerken, hooguit eens per 5 seconden.
  let mappenTimer: ReturnType<typeof setTimeout> | undefined;
  function mappenBijwerken() {
    if (mappenTimer) return;
    mappenTimer = setTimeout(async () => {
      mappenTimer = undefined;
      const m = await api.mappen(account).catch(() => null);
      if (m) lijstMappen = m;
    }, 5000);
  }


  // Eenmalig bij het openen: mappen ophalen, lijst vullen en luisteren naar de achtergrondsynchronisatie.
  // `untrack`: anders zou dit opnieuw draaien telkens als huidigeMap of koppen verandert.
  $effect(() => untrack(() => {
    poging(() => api.mappen(account)).then((m) => m && (lijstMappen = m));
    herlaad(STUK);
    telSterren();
    const weg = [
      listen<Gebeurtenis>("map-gewijzigd", (e) => {
        if (e.payload.account !== account) return;
        if (e.payload.map === huidigeMap || huidigeMap === STER) herlaad(Math.max(STUK, koppen.length));
        mappenBijwerken();
        telSterren();
      }),
      listen<Voortgang>("sync-voortgang", (e) => e.payload.account === account && (voortgang = e.payload)),
      listen<{ account: string; fout: string }>("sync-fout", (e) => e.payload.account === account && (verbindingsfout = e.payload.fout)),
    ];
    return () => weg.forEach((w) => w.then((f) => f()));
  }));
</script>

{#snippet zoekveld()}
  <label class="zoek">
    <Pictogram naam="zoek" maat={thema === "b" ? 20 : 16} />
    <input
      type="search"
      placeholder="Zoek op afzender of onderwerp…"
      aria-label="Zoeken"
      bind:value={zoekterm}
      oninput={typ}
      onkeydown={(e) => e.key === "Escape" && wisZoeken()}
    />
  </label>
{/snippet}

{#snippet mapknop(m: Map)}
  <button type="button" class="map" class:actief={m.pad === huidigeMap} onclick={() => kiesMap(m.pad)}>
    {#if thema === "c"}<span class="stip" style="background: {STIPPEN[m.soort] ?? '#5b5f66'}" aria-hidden="true"></span>{/if}
    <span class="mapnaam">{m.naam}</span>
    {#if teller(m)}<span class="teller">{teller(m)}</span>{/if}
  </button>
{/snippet}

{#snippet sterknopMap()}
  <button type="button" class="map" class:actief={huidigeMap === STER} onclick={() => kiesMap(STER)}>
    {#if thema === "c"}<span class="stip" style="background: #f2b35b" aria-hidden="true"></span>{:else}<span class="sterpicto"><Pictogram naam="ster" maat={14} gevuld /></span>{/if}
    <span class="mapnaam">Met ster</span>
    {#if aantalSter}<span class="teller">{aantalSter}</span>{/if}
  </button>
{/snippet}

{#snippet acties(b: Bericht, k: Kop)}
  {#if soortVan(k.map) === "concepten"}
    <button type="button" class="primair" onclick={() => verderSchrijven(b, k)}>Verder schrijven</button>
  {:else}
    <button type="button" class:primair={thema === "b"} onclick={() => beantwoord(b, k, false)}>Beantwoorden</button>
    <button type="button" onclick={() => beantwoord(b, k, true)}>Allen beantwoorden</button>
    <button type="button" onclick={() => doorsturen(b, k)}>Doorsturen</button>
  {/if}
  <div class="ruimte"></div>
  <button type="button" class="ster-actie" class:aan={k.gemarkeerd} aria-pressed={k.gemarkeerd} onclick={() => wisselSter(k)} title={k.gemarkeerd ? "Ster weghalen" : "Ster geven"}>
    <Pictogram naam="ster" gevuld={k.gemarkeerd} /> {k.gemarkeerd ? "Ster weghalen" : "Ster"}
  </button>
  {#if thema !== "b"}<button type="button" onclick={() => ongelezen(k)} title="Markeer als ongelezen">Ongelezen</button>{/if}
  {#if !["archief", "prullenbak", "concepten"].includes(soortVan(k.map) ?? "") && !/^archive/i.test(k.map)}
    <button type="button" class="zacht-accent" disabled={bezigMetActie} onclick={() => archiveren(k)}><Pictogram naam="archief" /> Archiveren</button>
  {/if}
  {#if soortVan(k.map) !== "prullenbak"}
    <button type="button" disabled={bezigMetActie} onclick={() => verwijderen(k)} aria-label="Verwijderen" title="Naar de prullenbak"><Pictogram naam="prullenbak" /></button>
  {/if}
{/snippet}

{#snippet accountkeuze()}
  {#if accounts.length > 1}
    <select class="accountkeuze" value={account} onchange={(e) => wisselAccount(e.currentTarget.value)} aria-label="Mailadres">
      {#each accounts as a (a.email)}<option value={a.email}>{a.email}</option>{/each}
    </select>
  {:else}
    {account}
  {/if}
{/snippet}

<div class="app thema-{thema}">
  {#if thema !== "c"}
    <header>
      <div class="merk"><span class="logo">PA</span> Pro-Art Mail</div>
      {#if thema === "b"}
        <nav class="tabs" aria-label="Mappen">
          {#each tabs as m, i (m.pad)}
            <button type="button" class="tab" class:actief={m.pad === huidigeMap} onclick={() => kiesMap(m.pad)}>{m.naam}{#if teller(m)} <span class="teller">{teller(m)}</span>{/if}</button>
            {#if i === 0}
              <button type="button" class="tab" class:actief={huidigeMap === STER} onclick={() => kiesMap(STER)}>Met ster{#if aantalSter} <span class="teller">{aantalSter}</span>{/if}</button>
            {/if}
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
      {#if thema === "b" && accounts.length > 1}{@render accountkeuze()}{/if}
      {#if thema === "a"}{@render zoekveld()}{/if}
      <div class="ruimte"></div>
      {#if thema === "b"}{@render zoekveld()}{/if}
      <button type="button" class="pictoknop" aria-label="Instellingen" title="Instellingen" onclick={() => (instellingenOpen = true)}>
        <Pictogram naam="instellingen" maat={thema === "b" ? 22 : 18} />
      </button>
      <button class="primair" type="button" onclick={() => (opstellen = leeg())}><Pictogram naam="plus" /> Nieuwe mail</button>
    </header>
  {/if}

  <div class="kolommen">
    {#if thema !== "b"}
      <nav class="mappen" aria-label="Mappen">
        {#if thema === "c"}
          <div class="merk-c">
            <span class="verf" aria-hidden="true"><span></span><span></span><span></span></span> Pro-Art Mail
          </div>
          <button class="primair" type="button" onclick={() => (opstellen = leeg())}>Nieuwe mail</button>
        {/if}
        <div class="kopje eerste">{@render accountkeuze()}</div>
        {#each ind.vast as m, i (m.pad)}
          {@render mapknop(m)}
          {#if i === 0}{@render sterknopMap()}{/if}
        {/each}
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
        {#if resultaten}
          <h2>{inTekstGezocht ? `In de tekst van ${naamHuidig}` : "Zoekresultaten"}</h2>
          <span>{resultaten.length === 300 ? "eerste 300" : resultaten.length} gevonden · <button type="button" class="link" onclick={wisZoeken}>wissen</button></span>
        {:else}
          <h2>{naamHuidig}</h2>
          <span>{ongelezenHuidig ? `${ongelezenHuidig} ongelezen · ` : ""}{totaal.toLocaleString("nl-NL")} berichten</span>
        {/if}
      </div>
      {#if resultaten && !inTekstGezocht}
        <div class="tekstzoek">
          <button type="button" onclick={zoekInTekst} disabled={zoekBezig}>
            <Pictogram naam="zoek" maat={14} />
            {zoekBezig ? "Bezig met zoeken in de tekst…" : `Ook in de tekst van de mails zoeken (${naamHuidig})`}
          </button>
        </div>
      {/if}
      {#if verbindingsfout}
        <p class="strook waarschuwing" role="status">Geen verbinding met de mailserver. De app probeert het zo opnieuw.</p>
      {/if}
      {#if bezigMetKlaarzetten}
        <p class="strook" role="status">
          {naamHuidig} wordt klaargezet: {bezigMetKlaarzetten.gedaan.toLocaleString("nl-NL")} van {bezigMetKlaarzetten.totaal.toLocaleString("nl-NL")}
        </p>
      {/if}
      <div class="rollen" bind:this={rollen} onscroll={gescrold}>
        {#each groepen as g (g.titel)}
          <div class="groep">{g.titel}</div>
          {#each g.koppen as k (k.uid)}
            <div class="regelhouder">
            <button type="button" class="regel" class:nieuw={!k.gelezen} class:gekozen={k.uid === gekozen && k.map === openKop?.map} onclick={() => kies(k)}>
              {#if thema === "b"}
                <span class="bol" style="background: {bolkleur(naamVan(k))}" aria-hidden="true">{initiaal(naamVan(k))}</span>
              {/if}
              <span class="regeltekst">
                <span class="boven">
                  {#if thema === "c"}<span class="nieuwstip" aria-hidden="true"></span>{/if}
                  <span class="van">{naamVan(k)}</span>
                  <span class="tijd">{korteDatum(k.datum)}</span>
                </span>
                {#if (resultaten || huidigeMap === STER) && k.map !== huidigeMap}<span class="mapetiket">{mapNaam(k.map)}</span>{/if}
                <span class="onderwerp">
                  {#if k.beantwoord}<span class="label" title="Beantwoord"><Pictogram naam="beantwoord" maat={13} /></span>{/if}
                  {#if k.bijlage}<span class="label" title="Bijlage"><Pictogram naam="bijlage" maat={13} /></span>{/if}
                  {k.onderwerp || "(geen onderwerp)"}
                </span>
              </span>
            </button>
            <button
              type="button"
              class="sterknop"
              class:aan={k.gemarkeerd}
              aria-pressed={k.gemarkeerd}
              aria-label={k.gemarkeerd ? "Ster weghalen" : "Ster geven"}
              title={k.gemarkeerd ? "Ster weghalen" : "Ster geven"}
              onclick={(e) => wisselSter(k, e)}
            >
              <Pictogram naam="ster" maat={thema === "b" ? 22 : 18} gevuld={k.gemarkeerd} />
            </button>
            </div>
          {/each}
        {/each}
        {#if resultaten}
          {#if !resultaten.length}<p class="melding">Niets gevonden{inTekstGezocht ? "" : " in afzender of onderwerp"}.</p>{/if}
        {:else if laden}
          <p class="melding">Berichten ophalen…</p>
        {:else if !koppen.length}
          <p class="melding">{klaar ? "Deze map is leeg." : "Deze map wordt klaargezet…"}</p>
        {/if}
      </div>
    </section>

    <main class="lezen">
      {#if fout}<p class="fout" role="alert">{fout}</p>{/if}
      {#if open}
        {#if thema !== "b" && openKop}<div class="acties boven">{@render acties(open, openKop)}</div>{/if}
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
                  <li>
                    <button type="button" class="bijlage" onclick={() => openKop && bijlageOpenen(openKop, i)} title="Openen">
                      <Pictogram naam="bijlage" maat={14} /> {b.naam} <span class="zacht">({grootte(b.grootte)})</span>
                    </button>
                    <button type="button" class="opslaan" onclick={() => openKop && bijlageOpslaan(openKop, i, b.naam)} aria-label="{b.naam} opslaan" title="Opslaan als…">
                      <Pictogram naam="omlaag" maat={14} />
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
            <div class="papier"><Inhoud html={open.html} tekst={open.tekst} /></div>
          </article>
        </div>
        {#if thema === "b" && openKop}<div class="acties onder">{@render acties(open, openKop)}</div>{/if}
      {:else if ladenBericht}
        <p class="melding">Bericht openen…</p>
      {:else}
        <p class="melding leeg">Kies een bericht om te lezen.</p>
      {/if}
    </main>
  </div>
</div>

{#if instellingenOpen}
  <Instellingen {thema} kies={kiesThema} {accounts} {accountsGewijzigd} sluit={() => (instellingenOpen = false)} />
{/if}

{#if opstellen}
  <Opstellen {accounts} {account} begin={opstellen} sluit={opstellenGesloten} />
{/if}

{#if melding}<div class="melding-onder" role="status">{melding}</div>{/if}

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
  .regelhouder {
    position: relative;
  }
  .sterknop {
    position: absolute;
    right: 10px;
    bottom: 8px;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--tekst-zacht);
    opacity: 0.55;
  }
  .sterknop:hover {
    opacity: 1;
    background: var(--rand-licht);
  }
  .sterknop.aan,
  .ster-actie.aan,
  .sterpicto {
    color: #e0a100;
    opacity: 1;
  }
  .thema-c .sterknop.aan,
  .thema-c .ster-actie.aan {
    color: #f2b35b;
  }
  .regel .onderwerp {
    padding-right: 30px;
  }
  .thema-b .sterknop {
    right: 26px;
    bottom: 50%;
    translate: 0 50%;
    width: 40px;
    height: 40px;
  }
  .thema-b .regel .regeltekst {
    padding-right: 44px;
  }
  .sterpicto {
    display: inline-flex;
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
  .strook {
    margin: 0;
    padding: 8px 18px;
    font-size: 13px;
    background: var(--accent-zacht);
    color: var(--op-accent-zacht);
  }
  .strook.waarschuwing {
    background: var(--fout-zacht);
    color: var(--fout);
  }
  .tekstzoek {
    padding: 8px 18px;
    border-bottom: 1px solid var(--rand-licht);
  }
  .tekstzoek button {
    width: 100%;
    min-height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 6px 12px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--knop);
    color: var(--op-accent-zacht);
    font-weight: 500;
  }
  .thema-b .tekstzoek button {
    min-height: 44px;
  }
  .mapetiket {
    align-self: flex-start;
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 12px;
    background: var(--rand-licht);
    color: var(--tekst-zacht);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--op-accent-zacht);
    text-decoration: underline;
    font-size: inherit;
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
  .bijlagen li {
    padding: 0;
    overflow: hidden;
  }
  .bijlagen .bijlage,
  .bijlagen .opslaan {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 36px;
    padding: 0 12px;
    border: 0;
    background: transparent;
  }
  .bijlagen .opslaan {
    padding: 0 10px;
    border-left: 1px solid var(--rand);
    color: var(--tekst-zacht);
  }
  .bijlagen button:hover {
    background: var(--rand-licht);
  }
  .accountkeuze {
    max-width: 100%;
    padding: 4px 6px;
    border: 1px solid var(--rand);
    border-radius: 6px;
    background: var(--knop);
    color: var(--tekst);
    font-size: 13px;
  }
  .melding-onder {
    position: fixed;
    left: 50%;
    bottom: 24px;
    translate: -50% 0;
    z-index: 20;
    padding: 12px 20px;
    border-radius: 10px;
    background: var(--tekst);
    color: var(--achtergrond);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.2);
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
