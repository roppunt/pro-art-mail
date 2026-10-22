<script lang="ts">
  import "@fontsource-variable/inter";
  import "$lib/thema.css";
  import { huidigAccount, type Account } from "$lib/api";
  import Hoofd from "$lib/Hoofd.svelte";
  import Inloggen from "$lib/Inloggen.svelte";

  let account = $state<Account | null | undefined>(undefined);
  huidigAccount().then((a) => (account = a), () => (account = null));
</script>

{#if account === undefined}
  <!-- even geduld: kijken of deze pc al is ingelogd -->
{:else if account === null}
  <Inloggen klaar={(a) => (account = a)} />
{:else}
  <Hoofd {account} afgemeld={() => (account = null)} />
{/if}
