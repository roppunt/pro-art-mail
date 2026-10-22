<script lang="ts">
  // Eerste start op een pc: het eerste account toevoegen.
  import AccountFormulier from "./AccountFormulier.svelte";
  import type { Account } from "./api";

  let { klaar }: { klaar: (lijst: Account[], email: string) => void } = $props();
</script>

<main class="scherm">
  <div class="kaart">
    <div class="merk"><span class="logo">PA</span> Pro-Art Mail</div>
    <p class="uitleg">Voeg het mailadres van de winkel toe. Deze pc onthoudt het wachtwoord daarna veilig. Later kun je in Instellingen meer adressen toevoegen.</p>
    <AccountFormulier {klaar} />
  </div>
</main>

<style>
  .scherm {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 24px 16px;
    box-sizing: border-box;
    background: var(--achtergrond);
  }
  .kaart {
    width: min(460px, 100%);
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 32px;
    box-sizing: border-box;
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
</style>
