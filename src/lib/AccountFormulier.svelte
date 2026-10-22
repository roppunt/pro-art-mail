<script lang="ts">
  // Account toevoegen of wijzigen. Voor @pro-art.nl worden de servers vanzelf ingevuld.
  import { accountBewaren, accountStandaard, type Account } from "./api";

  let {
    bestaand = null,
    klaar,
    annuleer = null,
  }: { bestaand?: Account | null; klaar: (lijst: Account[], email: string) => void; annuleer?: (() => void) | null } = $props();

  // svelte-ignore state_referenced_locally
  let acc = $state<Account>(
    bestaand ? { ...bestaand } : { email: "", naam: "", imap_host: "", imap_poort: 993, smtp_host: "", smtp_poort: 465, verzonden_map: "" },
  );
  let wachtwoord = $state("");
  let geavanceerd = $state(false);
  let bezig = $state(false);
  let fout = $state("");

  /** Na het invullen van het adres: serverinstellingen voorstellen (alleen bij een nieuw account). */
  async function adresIngevuld() {
    if (bestaand || !acc.email.includes("@")) return;
    const s = await accountStandaard(acc.email);
    acc = { ...s, naam: acc.naam };
  }

  async function verstuur(e: SubmitEvent) {
    e.preventDefault();
    if (!acc.imap_host) await adresIngevuld();
    bezig = true;
    fout = "";
    try {
      const lijst = await accountBewaren(acc, wachtwoord);
      klaar(lijst, acc.email.trim().toLowerCase());
    } catch (err) {
      fout = String(err);
      geavanceerd ||= !String(err).includes("wachtwoord");
    } finally {
      bezig = false;
    }
  }
</script>

<form class="formulier" onsubmit={verstuur}>
  <label>
    Mailadres
    <input type="email" bind:value={acc.email} onblur={adresIngevuld} autocomplete="username" required readonly={!!bestaand} />
  </label>
  <label>
    Wachtwoord
    <input
      type="password"
      bind:value={wachtwoord}
      autocomplete="current-password"
      required={!bestaand}
      placeholder={bestaand ? "Leeg laten = niet wijzigen" : ""}
    />
  </label>
  <label>
    Naam die ontvangers zien <span class="zacht">(mag leeg)</span>
    <input type="text" bind:value={acc.naam} placeholder="Bijvoorbeeld: Pro Art Nijmegen" />
  </label>

  <button type="button" class="link" onclick={() => (geavanceerd = !geavanceerd)} aria-expanded={geavanceerd}>
    {geavanceerd ? "Serverinstellingen verbergen" : "Serverinstellingen tonen"}
  </button>
  {#if geavanceerd}
    <div class="servers">
      <label>Server ontvangen (IMAP) <input type="text" bind:value={acc.imap_host} required /></label>
      <label class="poort">Poort <input type="number" bind:value={acc.imap_poort} required /></label>
      <label>Server versturen (SMTP) <input type="text" bind:value={acc.smtp_host} required /></label>
      <label class="poort">Poort <input type="number" bind:value={acc.smtp_poort} required /></label>
      <label class="breed">Map voor verstuurde mail <span class="zacht">(leeg = automatisch)</span><input type="text" bind:value={acc.verzonden_map} /></label>
    </div>
  {/if}

  {#if fout}<p class="fout" role="alert">{fout}</p>{/if}
  <div class="knoppen">
    {#if annuleer}<button type="button" class="tweede" onclick={annuleer}>Annuleren</button>{/if}
    <button type="submit" class="eerste" disabled={bezig}>{bezig ? "Verbinding testen…" : bestaand ? "Bewaren" : "Account toevoegen"}</button>
  </div>
</form>

<style>
  .formulier {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-weight: 500;
  }
  input {
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--invoer);
    font-weight: 400;
  }
  input[readonly] {
    opacity: 0.7;
  }
  .zacht {
    color: var(--tekst-zacht);
    font-weight: 400;
  }
  .link {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--op-accent-zacht);
    text-decoration: underline;
  }
  .servers {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 110px;
    gap: 12px;
  }
  .breed {
    grid-column: 1 / -1;
  }
  .fout {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--fout-zacht);
    color: var(--fout);
  }
  .knoppen {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .knoppen button {
    height: 42px;
    padding: 0 18px;
    border-radius: var(--radius);
    font-weight: 600;
  }
  .eerste {
    border: 0;
    background: var(--accent);
    color: var(--accent-tekst);
  }
  .tweede {
    border: 1px solid var(--rand);
    background: var(--knop);
  }
</style>
