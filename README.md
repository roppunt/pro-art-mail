# Pro-Art Mail

Een eenvoudige, rustige mailapp voor de computers van de winkel Pro-Art. Gebouwd met
[Tauri](https://tauri.app) (Rust) en Svelte; draait op Windows en Linux.

De app praat rechtstreeks met de mailserver via IMAP en SMTP. Alle mail blijft op de server;
de app bewaart alleen het mailadres in een instellingenbestand en het wachtwoord in de
wachtwoordkluis van het besturingssysteem.

## Ontwikkelen

```bash
npm install
npx tauri dev
```

Benodigd: Node.js 22+, Rust (rustup) en de [Tauri-systeemvereisten](https://tauri.app/start/prerequisites/).
