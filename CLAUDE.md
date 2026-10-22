# eigen email app

Een eigen email programma/app in eigen styl

## Over deze map
- Aangemaakt op 2026-10-22 via het Overzicht (`~/overzicht`, machine ai-ubuntu). Het project staat in
  de catalogus; de archivaris vult de beschrijving, diensten en links later aan op basis van wat hier staat.
- Houd deze CLAUDE.md bij: wat het project doet, hoe het draait, waar het staat en wat er nog open is.
- Staat onder Syncthing (PC ↔ NAS ↔ laptop); virtualenvs en `node_modules` worden genegeerd.
- **Openbare repository** https://github.com/roppunt/pro-art-mail. Geen wachtwoorden, IP-adressen of winkelgegevens in git:
  die horen in `lokaal/` (gitignored). De Stop-hook `~/.claude/auto-sync.sh` commit automatisch na elke beurt (pusht niet). Per afgeronde stap: auto-sync-commits
  samenvoegen (`git reset --soft origin/main` + nette commit), scannen op IP-adressen/sleutels, dan `git push` (gh is gekoppeld via `gh auth setup-git`).

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
  C atelier (donker, verfkleuren). De knop "Opbergen in <maand>" uit de ontwerpen vervalt (er zijn geen maandmappen).

## Besluiten voor stap 4 (Robert, 2026-10-22)
- **Archiveren** → `Archives.<jaar van de mail>` (map zo nodig aanmaken), zoals het tot 2022 ging.
- **Verzonden** opslaan in `Sent Messages` (in gebruik t/m vandaag; `Sent` laatst 2026-09-28). Prullenbak = `Trash`
  (`Deleted Messages` sinds 2022 ongebruikt).
- **Dubbele mappen laten zoals ze zijn** (geen risico voor eM Client/telefoons).
- **Ondertekening:** Robert overlegt nog. Voorlopig: kiezen bij elke mail uit een lijstje (beheren in Instellingen).

## Code en bouwen (stap 1 gebouwd 2026-10-22)
- **Tauri 2 + SvelteKit (Svelte 5, TypeScript, SPA)**. Identifier `nl.pro-art.mail`, productnaam "Pro-Art Mail".
- Rust (`src-tauri/src/`):
  - `account.rs` — **meerdere accounts** in `accounts.json` (app-configmap; overstap van v1 `account.json` automatisch):
    adres, afzendernaam, IMAP/SMTP-host+poort, `verzonden_map` (pro-art.nl: `Sent Messages`). Standaarden per domein in
    `Account::standaard` (pro-art.nl → mail.pro-art.nl, gmail.com → imap/smtp.gmail.com, anders mail.<domein>).
    Wachtwoorden via crate `keyring` 4 (dienst `pro-art-mail`). Ook `ondertekeningen.json` (gedeeld door alle accounts).
  - `imap.rs` — `async-imap` + `tokio-rustls` (ring, webpki-roots). Berichten altijd met `BODY.PEEK`; gelezen markeren
    gaat expliciet (`markeer`). Achtergrondverbinding opent mappen met EXAMINE, de gebruikersverbinding met SELECT.
    Schrijven: `markeer`, `markeer_beantwoord`, `verplaats` (UID MOVE, anders COPY+EXPUNGE), `zorg_map` (CREATE),
    `bewaar_in` (APPEND), `wis` (alleen oude conceptversies), `vaste_map` (SPECIAL-USE, anders op naam).
  - `mail.rs` — ontleden met `mail-parser`, HTML opschonen met `ammonia` (scripts/formulieren weg, inline stijlen blijven);
    `cid:`-afbeeldingen worden data-URI's; bijlagen ophalen; Message-ID/References voor beantwoorden.
  - `versturen.rs` — concept → mail met `lettre` (alternative text+html, bijlagen van pc of uit een bestaand bericht),
    SMTP 465 = implicit TLS, anders STARTTLS. Vaste Message-ID per concept, zodat een oude versie in Concepten te vinden is.
  - `cache.rs` — **lokale kopie** van de kopregels (SQLite via `rusqlite` bundled, `kopie.sqlite` in de app-datamap, ~6 MB
    voor ~33.600 berichten). Tabellen `mappen` (pad, uidvalidity, klaar) en `koppen` (map, uid, afzender, onderwerp, datum,
    vlaggen, kolom `zoek` = kleine letters voor LIKE-zoeken, ~4 ms). Alleen een kopie: weggooien mag, wordt opnieuw opgebouwd.
    Bij afmelden gewist.
  - `sync.rs` — **achtergrondverbinding** (tweede IMAP-login): eerst de map met voorrang (waar de gebruiker naar kijkt), dan
    INBOX, dan alle mappen (eerste keer, daarna elk kwartier). Per map: `UID FETCH 1:* (FLAGS)` → verwijderd/gewijzigd/nieuw;
    nieuwe kopregels in stukken van 500 met `BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE CONTENT-TYPE)]` + INTERNALDATE
    (terugval als Date ontbreekt). Daarna IDLE op INBOX (max 10 min). Events naar de interface: `map-gewijzigd`,
    `sync-voortgang`, `sync-fout`. Eerste volledige opbouw op ai-ubuntu: < 1 minuut.
  - `lib.rs` — per account een `AccountToestand` (verbinding, kopie `kopie-<adres>.sqlite`, sync-taak). Na versturen:
    APPEND in Verzonden, `\Answered` op het origineel, oude conceptversies weg. Archiveren → `Archives.<jaar van de mail>`.
    Verwijderen → prullenbak (nooit definitief). Bijlage openen = tijdelijk bestand + standaardprogramma.
    Tauri-opdrachten `huidig_account`, `inloggen`, `afmelden`, `mappen`, `berichten` (uit de kopie, `vanaf`/`aantal`),
    `zoek` (kopie, alle mappen), `zoek_in_tekst` (IMAP `UID SEARCH TEXT` op de server, één map), `bericht`. `mappen` doet STATUS per map (aantal/ongelezen) en maakt van dubbele vaste mappen (bv. `Sent` + `Sent Messages`)
    alleen de volste "Verzonden"; de andere blijft zichtbaar onder zijn eigen naam. Eén IMAP-verbinding, bij een fout één keer opnieuw verbinden (macro `met_verbinding!`).
- Interface (`src/`): `routes/+page.svelte` (accounts laden, laatst gebruikte account onthouden, lettertypen en thema),
  `lib/AccountFormulier.svelte` (toevoegen/wijzigen, servers automatisch), `lib/Opstellen.svelte` (opstelvenster: aanvullen
  van adressen uit de kopie, opmaak, ondertekening kiezen, bijlagen; sluiten = bewaren in Concepten), `lib/Hoofd.svelte`
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

## Uitbrengen en updates (stap 5, opgezet 2026-10-22)
- **Nieuwe versie:** versie ophogen in `package.json` (tauri.conf.json leest die), committen, dan
  `git tag v0.x.y && git push origin main --tags`. GitHub Actions (`.github/workflows/uitbrengen.yml`, tauri-action) bouwt op
  `windows-latest` (NSIS `.exe`, per gebruiker, Nederlands) en `ubuntu-22.04` (AppImage + `.deb`; 22.04 = basis van Zorin 17)
  en zet alles met `latest.json` in een GitHub-release.
- **Automatische updates:** `tauri-plugin-updater`; de app kijkt bij het starten naar
  `https://github.com/roppunt/pro-art-mail/releases/latest/download/latest.json`, installeert en herstart (`src/lib/bijwerken.ts`).
  Windows: installMode `passive`. Linux: alleen de **AppImage** werkt zichzelf bij (niet de .deb).
- ⚠️ **Ondertekensleutel voor updates:** `~/.tauri/pro-art-mail.key` + `.key.wachtwoord` (600) op ai-ubuntu, en als GitHub-secrets
  `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)`. Publieke sleutel staat in `tauri.conf.json`. **Kwijt = bestaande installaties
  krijgen geen updates meer** → Robert bewaart beide ook in zijn wachtwoordbeheerder.
- Geen code-certificaat: Windows toont bij de eerste installatie "Windows heeft uw pc beschermd" → *Meer info* → *Toch uitvoeren*.
- Pictogram: `ontwerp/pictogram.svg` (envelop + drie verfstrepen), omgezet met `npx tauri icon ontwerp/pictogram.svg`.
- Lokaal een ondertekende release bouwen (test): sleutel in `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)` zetten, dan
  `npx tauri build --bundles appimage` (~2 min, AppImage ~88 MB).

## Ideeën voor later
- "Verzonden" als samengevoegde weergave van `Sent` + `Sent Messages` (zolang webmail en eM Client naast de app gebruikt worden).

## Stappenplan
1. ✅ Basis: verbinden, mappen, postvak IN, bericht lezen (alleen lezen). Getest door Robert op ai-ubuntu 2026-10-22: inloggen werkt, mails zichtbaar.
2. ✅ (2026-10-22, goedgekeurd door Robert, gepusht) De drie thema's (A/B/C) + keuze in Instellingen, per pc onthouden.
3. ✅ (2026-10-22, goedgekeurd, gepusht; tekstzoeken op de server < 1 s) Zoeken en snel bladeren (lokale SQLite-kopie), vloeiend scrollen, IDLE.
4. ✅ (2026-10-22, getest door Robert: versturen, beantwoorden, bijlagen, archiveren, verwijderen, concepten, ondertekening; gepusht) Beantwoorden/allen/doorsturen/nieuw, concepten, ondertekeningen,
   bijlagen openen/opslaan/toevoegen, gelezen/ongelezen, archiveren, verwijderen, **meerdere accounts**.
5. ✅ (opgezet 2026-10-22) Windows-installatie + automatische updates via GitHub (`roppunt/pro-art-mail`, **openbaar**).
   **v0.1.0 uitgebracht 2026-10-22** (bouw ~10 min; .exe 4 MB, AppImage 81 MB, .deb 6 MB). Nog: proefdraaien op de kantoor-pc.
