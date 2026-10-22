// Automatische updates: bij het opstarten kijken of er op GitHub een nieuwere versie staat.
// Zo ja: op de achtergrond downloaden, installeren en de app opnieuw starten.
// Updates zijn ondertekend; de app installeert alleen wat met de sleutel van Pro-Art is gemaakt.
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

export async function zoekUpdate(melding: (tekst: string) => void) {
  try {
    const update = await check();
    if (!update) return;
    melding(`Nieuwe versie ${update.version} wordt geïnstalleerd…`);
    await update.downloadAndInstall();
    await relaunch();
  } catch (e) {
    // Geen internet of GitHub onbereikbaar: gewoon doorwerken met deze versie.
    console.warn("Update zoeken mislukt:", e);
  }
}
