<script lang="ts">
  import "@fontsource-variable/inter";
  import "@fontsource/atkinson-hyperlegible/400.css";
  import "@fontsource/atkinson-hyperlegible/700.css";
  import "@fontsource-variable/dm-sans";
  import "$lib/thema.css";
  import { huidigAccount, type Account } from "$lib/api";
  import Hoofd from "$lib/Hoofd.svelte";
  import Inloggen from "$lib/Inloggen.svelte";
  import { laadThema, zetThema, type ThemaId } from "$lib/thema";

  let account = $state<Account | null | undefined>(undefined);
  let thema = $state<ThemaId>(laadThema());
  huidigAccount().then((a) => (account = a), () => (account = null));

  $effect(() => zetThema(thema));
</script>

{#if account === undefined}
  <!-- even geduld: kijken of deze pc al is ingelogd -->
{:else if account === null}
  <Inloggen klaar={(a) => (account = a)} />
{:else}
  <Hoofd {account} afgemeld={() => (account = null)} {thema} kiesThema={(t) => (thema = t)} />
{/if}
