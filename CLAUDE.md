# eigen email app

Een eigen email programma/app in eigen styl

## Over deze map
- Aangemaakt op 2026-10-22 via het Overzicht (`~/overzicht`, machine ai-ubuntu). Het project staat in
  de catalogus; de archivaris vult de beschrijving, diensten en links later aan op basis van wat hier staat.
- Houd deze CLAUDE.md bij: wat het project doet, hoe het draait, waar het staat en wat er nog open is.
- Staat onder Syncthing (PC ↔ NAS ↔ laptop); virtualenvs en `node_modules` worden genegeerd.
- **Openbare repository** https://github.com/roppunt/pro-art-mail. Geen wachtwoorden, IP-adressen of winkelgegevens in git:
  die horen in `lokaal/` (gitignored). De Stop-hook `~/.claude/auto-sync.sh` commit automatisch na elke beurt (pusht niet).

## Besluiten tot nu toe (2026-10-22)
- **Voor de winkel**, niet voor Robert privé. De winkel heeft een **eigen mailserver** (geen Gmail) → IMAP/SMTP.
- **Gebruikt op de computers in de winkel.**
- "Eigen stijl" = vooral **uiterlijk** (rustig, overzichtelijk, eigen kleuren/indeling). Geen AI-sortering in v1.
- **Eerste versie:** lezen, zoeken en beantwoorden. Extra's (bijlagen, mappen beheren, meerdere accounts) daarna.
- De mail blijft op de mailserver; de app is een "venster" erop. Gaat de app stuk, dan is er geen mail kwijt.

## Mailserver
- `mail.pro-art.nl`: IMAP 993 (SSL), SMTP 465 (SSL), inloggen met mailadres + wachtwoord. Meerdere adressen → meerdere accounts.
- Winkelsituatie, pc's, netwerk en hoster-details: **`lokaal/WINKEL.md`** (niet in git, repository is openbaar):

@lokaal/WINKEL.md

## Aanpak (akkoord Robert 2026-10-22)
- **Losse desktop-app per pc, gebouwd met Tauri** (Windows + Linux uit één code, klein en licht). Praat rechtstreeks met
  `mail.pro-art.nl` via IMAP/SMTP; geen eigen server. Mappen en gelezen/ongelezen staan op de server, dus alle pc's lopen gelijk.
- Op **alle vijf** de winkel-pc's (4× Windows, 1× Zorin). Hoofdgebruik: de kantoor-pc (Acer XC-840, **8 GB RAM**).
- Lokale cache per pc (SQLite) voor snel bladeren/zoeken in grote mappen (postvak IN ~27.000 berichten).
- Wachtwoorden in de opslag van het OS (Windows Referentiebeheer / Linux-sleutelbos), niet in een bestand.
- **Automatische updates** + Windows-builds via een GitHub-repository onder `roppunt` (GitHub Actions).
- eM Client blijft naast de nieuwe app werken tot de overstap rond is.

## Nog open
- **Uiterlijk: alle drie de ontwerpen worden thema's in de app; de gebruiker kiest zelf** (besluit Robert 2026-10-22). Canvas https://claude.ai/artifact/WwjHqQ7Ck7YUfRf44egvb7 :
  A rustig en vertrouwd (licht, 3 kolommen, blauw), B groot en eenvoudig (grote letters/knoppen, 2 kolommen, oranje),
  C atelier (donker, verfkleuren). De knop "Opbergen in <maand>" uit de ontwerpen vervalt (er zijn geen maandmappen);
  voorstel "Archiveren" → `Archives.<jaar>`, wacht op Robert.

## Code en bouwen (stap 1 gebouwd 2026-10-22)
- **Tauri 2 + SvelteKit (Svelte 5, TypeScript, SPA)**. Identifier `nl.pro-art.mail`, productnaam "Pro-Art Mail".
- Rust (`src-tauri/src/`):
  - `account.rs` — account op deze pc: `account.json` in de app-configmap (adres + vaste servers van WBV),
    wachtwoord via crate `keyring` 4 (Windows Referentiebeheer / Secret Service), dienstnaam `pro-art-mail`.
  - `imap.rs` — `async-imap` + `tokio-rustls` (ring, webpki-roots). **Stap 1 = alleen lezen**: mappen met `EXAMINE`,
    berichten met `BODY.PEEK`, dus niets verandert op de server (ook gelezen/ongelezen niet).
  - `mail.rs` — ontleden met `mail-parser`, HTML opschonen met `ammonia` (scripts/formulieren weg, inline stijlen blijven).
  - `lib.rs` — Tauri-opdrachten `huidig_account`, `inloggen`, `afmelden`, `mappen`, `berichten` (50 per pagina, nieuwste
    eerst), `bericht`. `mappen` doet STATUS per map (aantal/ongelezen) en maakt van dubbele vaste mappen (bv. `Sent` + `Sent Messages`)
    alleen de volste "Verzonden"; de andere blijft zichtbaar onder zijn eigen naam. Eén IMAP-verbinding, bij een fout één keer opnieuw verbinden (macro `met_verbinding!`).
- Interface (`src/`): `routes/+page.svelte` (inloggen of hoofdscherm, laadt lettertypen en thema), `lib/Hoofd.svelte`
  (alle drie de indelingen: A drie kolommen, B tabbladen + knoppen onderaan, C donker; klassen `.thema-a/b/c`),
  `lib/thema.ts` (keuze in `localStorage`, `data-thema` op `<html>`), `lib/Instellingen.svelte` (thema kiezen, afmelden),
  `lib/Pictogram.svelte` (lijnpictogrammen),
  `lib/Inloggen.svelte`, `lib/Inhoud.svelte` (mail-HTML in een shadow root; links openen in de browser),
  `lib/opmaak.ts` (datums, jaararchieven `Archives.<jaar>` herkennen), `lib/thema.css` (CSS-variabelen per thema). Lettertypen lokaal via @fontsource
  (Inter, Atkinson Hyperlegible, DM Sans): werkt zonder internet. In thema C staat de mailinhoud op een licht "vel papier".
- ⚠️ **Rust-bouwuitvoer staat buiten Syncthing**: `src-tauri/.cargo/config.toml` (niet in git) zet `target-dir` op
  `~/.cache/pro-art-mail-target`. Anders synct ~/Projects gigabytes. Op een nieuwe machine dat bestand meenemen of opnieuw maken.
- Bouwen/starten op ai-ubuntu (Rust via rustup in `~/.cargo`, systeempakketten `libwebkit2gtk-4.1-dev` e.a. geïnstalleerd 2026-10-22):
  ```bash
  export PATH=$HOME/.cargo/bin:$PATH
  npm install
  npx tauri dev                                # ontwikkelen
  npx tauri build --debug --no-bundle          # testversie → ~/.cache/pro-art-mail-target/debug/pro-art-mail
  npm run check                                # Svelte/TypeScript controleren
  ```

## Stappenplan
1. ✅ Basis: verbinden, mappen, postvak IN, bericht lezen (alleen lezen). Getest door Robert op ai-ubuntu 2026-10-22: inloggen werkt, mails zichtbaar.
2. ✅ (gebouwd 2026-10-22, wacht op oordeel Robert) De drie thema's (A/B/C) + keuze in Instellingen, per pc onthouden.
3. Zoeken en snel bladeren (lokale SQLite-kopie).
4. Beantwoorden/doorsturen/nieuw (SMTP 465, `lettre`), gelezen markeren, archiveren (?), verwijderen. Verzonden opslaan in de
   map die eM Client gebruikt.
5. Windows-installatie + automatische updates via GitHub (`roppunt/pro-art-mail`, **openbaar**, besluit Robert), proefdraaien op KANTOOR.
