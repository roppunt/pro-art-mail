<script lang="ts">
  import { inloggen, type Account } from "./api";

  let { klaar }: { klaar: (a: Account) => void } = $props();

  let email = $state("info@pro-art.nl");
  let wachtwoord = $state("");
  let bezig = $state(false);
  let fout = $state("");

  async function verstuur(e: SubmitEvent) {
    e.preventDefault();
    bezig = true;
    fout = "";
    try {
      klaar(await inloggen(email, wachtwoord));
    } catch (err) {
      fout = String(err);
    } finally {
      bezig = false;
      wachtwoord = "";
    }
  }
</script>

<main class="scherm">
  <form class="kaart" onsubmit={verstuur}>
    <div class="merk"><span class="logo">PA</span> Pro-Art Mail</div>
    <p class="uitleg">Log één keer in met het mailadres en het wachtwoord van de winkel. Deze pc onthoudt het daarna veilig.</p>
    <label>
      Mailadres
      <input type="email" bind:value={email} autocomplete="username" required />
    </label>
    <label>
      Wachtwoord
      <!-- svelte-ignore a11y_autofocus -->
      <input type="password" bind:value={wachtwoord} autocomplete="current-password" required autofocus />
    </label>
    {#if fout}<p class="fout" role="alert">{fout}</p>{/if}
    <button type="submit" disabled={bezig}>{bezig ? "Bezig met inloggen…" : "Inloggen"}</button>
  </form>
</main>

<style>
  .scherm {
    height: 100vh;
    display: grid;
    place-items: center;
    background: var(--achtergrond);
  }
  .kaart {
    width: min(400px, calc(100vw - 32px));
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 32px;
    background: var(--vlak);
    border: 1px solid var(--rand);
    border-radius: 14px;
  }
  .merk {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 18px;
    font-weight: 600;
  }
  .logo {
    width: 32px;
    height: 32px;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-tekst);
    display: grid;
    place-items: center;
    font-size: 14px;
    font-weight: 700;
  }
  .uitleg {
    margin: 0;
    color: var(--tekst-zacht);
    line-height: 1.5;
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
    background: var(--vlak);
  }
  button {
    height: 42px;
    border: 0;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-tekst);
    font-weight: 600;
  }
  .fout {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--fout-zacht);
    color: var(--fout);
  }
</style>
