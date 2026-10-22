<script lang="ts">
  import Pictogram from "./Pictogram.svelte";
  import { THEMAS, type ThemaId } from "./thema";

  let {
    thema,
    kies,
    email,
    afmelden,
    sluit,
  }: { thema: ThemaId; kies: (t: ThemaId) => void; email: string; afmelden: () => void; sluit: () => void } = $props();

  let dialoog: HTMLDialogElement;
  $effect(() => dialoog.showModal());
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

  <h3>Account</h3>
  <div class="account">
    <span>{email}</span>
    <button type="button" class="afmelden" onclick={afmelden}>Afmelden op deze pc</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(760px, calc(100vw - 32px));
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
    padding: 0 14px;
    border: 1px solid var(--rand);
    border-radius: var(--radius);
    background: var(--vlak);
  }
</style>
