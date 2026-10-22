<script lang="ts">
  import "@fontsource-variable/inter";
  import "@fontsource/atkinson-hyperlegible/400.css";
  import "@fontsource/atkinson-hyperlegible/700.css";
  import "@fontsource-variable/dm-sans";
  import "$lib/thema.css";
  import { accounts as haalAccounts, type Account } from "$lib/api";
  import Hoofd from "$lib/Hoofd.svelte";
  import Inloggen from "$lib/Inloggen.svelte";
  import { zoekUpdate } from "$lib/bijwerken";
  import { laadThema, zetThema, type ThemaId } from "$lib/thema";

  const LAATSTE = "pro-art-mail.account";

  let accounts = $state<Account[] | undefined>(undefined);
  let huidig = $state("");
  let thema = $state<ThemaId>(laadThema());

  function kies(lijst: Account[], voorkeur?: string) {
    accounts = lijst;
    let laatste: string | null = null;
    try {
      laatste = localStorage.getItem(LAATSTE);
    } catch {
      // geen opslag beschikbaar
    }
    const gewenst = [voorkeur, huidig, laatste].find((e) => e && lijst.some((a) => a.email === e));
    wissel(gewenst ?? lijst[0]?.email ?? "");
  }

  function wissel(email: string) {
    huidig = email;
    try {
      localStorage.setItem(LAATSTE, email);
    } catch {
      // niet erg
    }
  }

  let updateMelding = $state("");

  haalAccounts().then((l) => kies(l), () => kies([]));
  // Niet in de ontwikkelversie: die heeft geen update-gegevens.
  if (!import.meta.env.DEV) zoekUpdate((t) => (updateMelding = t));

  $effect(() => zetThema(thema));
</script>

{#if updateMelding}<div class="update" role="status">{updateMelding}</div>{/if}

{#if accounts === undefined}
  <!-- even geduld: accounts van deze pc ophalen -->
{:else if !accounts.length || !huidig}
  <Inloggen klaar={(lijst, email) => kies(lijst, email)} />
{:else}
  {#key huidig}
    <Hoofd
      {accounts}
      account={huidig}
      wisselAccount={wissel}
      accountsGewijzigd={(lijst, voorkeur) => kies(lijst, voorkeur)}
      {thema}
      kiesThema={(t) => (thema = t)}
    />
  {/key}
{/if}

<style>
  .update {
    position: fixed;
    top: 12px;
    left: 50%;
    translate: -50% 0;
    z-index: 30;
    padding: 10px 18px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-tekst);
    font-weight: 600;
  }
</style>
