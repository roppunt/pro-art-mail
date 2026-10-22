<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import AccountFormulier from "./AccountFormulier.svelte";
  import * as api from "./api";
  import type { Account, Ondertekening } from "./api";
  import Pictogram from "./Pictogram.svelte";
  import { THEMAS, type ThemaId } from "./thema";

  let {
    thema,
    kies,
    accounts,
    accountsGewijzigd,
    sluit,
  }: {
    thema: ThemaId;
    kies: (t: ThemaId) => void;
    accounts: Account[];
    accountsGewijzigd: (lijst: Account[], kies?: string) => void;
    sluit: () => void;
  } = $props();

  let dialoog: HTMLDialogElement;
  // null = lijst tonen; "nieuw" = account toevoegen; anders het adres dat bewerkt wordt
  let bewerk = $state<string | null>(null);
  let ondertekeningen = $state<Ondertekening[]>([]);
  let ondertekeningenBewaard = $state(true);
  let fout = $state("");

  $effect(() => {
    dialoog.showModal();
    api.ondertekeningen().then((l) => (ondertekeningen = l));
  });

  async function verwijderAccount(email: string) {
    const ok = await ask(`${email} van deze pc halen? De mail blijft gewoon op de server staan.`, { title: "Mailadres weghalen", kind: "warning", okLabel: "Weghalen", cancelLabel: "Annuleren" });
    if (!ok) return;
    try {
      accountsGewijzigd(await api.accountVerwijderen(email));
    } catch (e) {
      fout = String(e);
    }
  }

  async function bewaarOndertekeningen() {
    try {
      await api.bewaarOndertekeningen(ondertekeningen.filter((o) => o.naam.trim() || o.tekst.trim()));
      ondertekeningenBewaard = true;
    } catch (e) {
      fout = String(e);
    }
  }
</script>

<dialog bind:this={dialoog} onclose={sluit} aria-labelledby="inst-titel">
  <div class="kop">
    <h2 id="inst-titel">Instellingen</h2>
    <button type="button" class="sluit" aria-label="Sluiten" onclick={() => dialoog.close()}><Pictogram naam="sluiten" maat={18} /></button>
  </div>

  <h3>Uiterlijk</h3>
  <p class="zacht">Kies wat voor jou het prettigst werkt. Deze pc onthoudt het.</p>
  <div class="themas">
    {#each THEMAS as t (t.id)}
      <button type="button" class="thema" class:gekozen={t.id === thema} aria-pressed={t.id === thema} onclick={() => kies(t.id)}>
        <span class="voorbeeld v-{t.id}" aria-hidden="true">
          <span></span><span></span><span></span>
        </span>
        <span class="naam">{t.naam}</span>
        <span class="uitleg">{t.uitleg}</span>
      </button>
    {/each}
  </div>

  <h3>Mailadressen op deze pc</h3>
  {#if fout}<p class="fout" role="alert">{fout}</p>{/if}
  {#if bewerk}
    <div class="kader">
      <AccountFormulier
        bestaand={bewerk === "nieuw" ? null : (accounts.find((a) => a.email === bewerk) ?? null)}
        klaar={(lijst, email) => {
          accountsGewijzigd(lijst, bewerk === "nieuw" ? email : undefined);
          bewerk = null;
        }}
        annuleer={() => (bewerk = null)}
      />
    </div>
  {:else}
    {#each accounts as a (a.email)}
      <div class="account">
        <span><strong>{a.email}</strong>{#if a.naam}<span class="zacht"> · {a.naam}</span>{/if}</span>
        <span class="knoppen">
          <button type="button" class="afmelden" onclick={() => (bewerk = a.email)}>Wijzigen</button>
          <button type="button" class="afmelden" onclick={() => verwijderAccount(a.email)}>Van deze pc halen</button>
        </span>
      </div>
    {/each}
    <button type="button" class="afmelden toevoegen" onclick={() => (bewerk = "nieuw")}><Pictogram naam="plus" maat={14} /> Mailadres toevoegen</button>
  {/if}

  <h3>Ondertekeningen</h3>
  <p class="zacht">Bij het schrijven van een mail kies je welke eronder komt.</p>
  {#each ondertekeningen as o, i (i)}
    <div class="ondertekening">
      <input type="text" bind:value={o.naam} placeholder="Naam, bv. Winkel" aria-label="Naam van de ondertekening" oninput={() => (ondertekeningenBewaard = false)} />
      <textarea rows="3" bind:value={o.tekst} placeholder={"Met vriendelijke groet,\nPro Art Nijmegen"} aria-label="Tekst van de ondertekening" oninput={() => (ondertekeningenBewaard = false)}></textarea>
      <button type="button" class="sluit" aria-label="Ondertekening weghalen" onclick={() => ((ondertekeningen = ondertekeningen.filter((_, j) => j !== i)), (ondertekeningenBewaard = false))}>
        <Pictogram naam="prullenbak" maat={16} />
      </button>
    </div>
  {/each}
  <div class="knoppen">
    <button type="button" class="afmelden" onclick={() => ((ondertekeningen = [...ondertekeningen, { naam: "", tekst: "" }]), (ondertekeningenBewaard = false))}>
      <Pictogram naam="plus" maat={14} /> Ondertekening toevoegen
    </button>
    {#if !ondertekeningenBewaard}<button type="button" class="bewaar" onclick={bewaarOndertekeningen}>Ondertekeningen bewaren</button>{/if}
  </div>
</dialog>

<style>
  dialog {
    width: min(760px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 28px 32px 32px;
    border: 1px solid var(--rand);
    border-radius: 16px;
    background: var(--vlak);
    color: var(--tekst);
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.35);
  }
  .kop {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h2 {
    margin: 0;
    font-size: 20px;
  }
  h3 {
    margin: 24px 0 4px;
    font-size: 15px;
  }
  .zacht {
    margin: 0 0 14px;
    color: var(--tekst-zacht);
  }
  .sluit {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--tekst-zacht);
  }
  .themas {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }
  .thema {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border: 2px solid var(--rand);
    border-radius: 12px;
    background: var(--vlak);
    text-align: left;
  }
  .thema.gekozen {
    border-color: var(--accent);
  }
  .naam {
    font-weight: 700;
  }
  .uitleg {
    font-size: 13px;
    color: var(--tekst-zacht);
    line-height: 1.4;
  }
  /* Miniatuur van elk thema: drie vlakjes in de kleuren van dat thema. */
  .voorbeeld {
    height: 72px;
    display: flex;
    gap: 4px;
    padding: 6px;
    border-radius: 8px;
  }
  .voorbeeld span {
    border-radius: 4px;
  }
  .v-a {
    background: #f4f5f7;
  }
  .v-a span:nth-child(1) {
    flex: 1;
    background: #e3e9f2;
  }
  .v-a span:nth-child(2) {
    flex: 2;
    background: #ffffff;
  }
  .v-a span:nth-child(3) {
    flex: 3;
    background: #ffffff;
    box-shadow: inset 0 3px 0 #1f4e8c;
  }
  .v-b {
    background: #fbfaf7;
  }
  .v-b span:nth-child(1) {
    display: none;
  }
  .v-b span:nth-child(2) {
    flex: 2;
    background: #fbeee4;
  }
  .v-b span:nth-child(3) {
    flex: 3;
    background: #ffffff;
    box-shadow: inset 0 -12px 0 #b4470b;
  }
  .v-c {
    background: #17181b;
  }
  .v-c span:nth-child(1) {
    flex: 1;
    background: #f2b35b;
    opacity: 0.8;
  }
  .v-c span:nth-child(2) {
    flex: 2;
    background: #212226;
  }
  .v-c span:nth-child(3) {
    flex: 3;
    background: #26282d;
  }
  .account {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border: 1px solid var(--rand);
    border-radius: 10px;
  }
  .afmelden {
    height: 36px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 14px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--vlak);
  }
  .account + .account {
    margin-top: 8px;
  }
  .toevoegen {
    margin-top: 10px;
  }
  .knoppen {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .kader {
    padding: 16px;
    border: 1px solid var(--rand);
    border-radius: 10px;
  }
  .ondertekening {
    display: grid;
    grid-template-columns: 160px minmax(0, 1fr) 36px;
    gap: 8px;
    margin-bottom: 10px;
  }
  .ondertekening input,
  .ondertekening textarea {
    padding: 8px 10px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--invoer);
    resize: vertical;
  }
  .ondertekening input {
    height: 38px;
    box-sizing: border-box;
  }
  .bewaar {
    height: 36px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-tekst);
    font-weight: 600;
  }
  .fout {
    margin: 0 0 10px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--fout-zacht);
    color: var(--fout);
  }
</style>
