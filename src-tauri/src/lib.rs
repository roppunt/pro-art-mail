mod account;
mod cache;
mod imap;
mod mail;
mod sync;
mod versturen;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use account::{Account, Ondertekening};
use cache::Cache;
use sync::Gebeurtenis;

/// Alles wat bij één account hoort: de verbinding voor de gebruiker, de lokale kopie en de
/// achtergrondsynchronisatie (met een eigen verbinding).
struct AccountToestand {
    verbinding: Mutex<Option<imap::Verbinding>>,
    cache: Arc<Cache>,
    sync: Arc<sync::Sync>,
}

#[derive(Default)]
struct Toestand {
    accounts: std::sync::Mutex<HashMap<String, Arc<AccountToestand>>>,
}

/// Fouten gaan als leesbare tekst naar de interface.
type Uitkomst<T> = Result<T, String>;

fn tekst(e: anyhow::Error) -> String {
    // Alleen de bovenste melding tonen; de ketting staat in de log.
    eprintln!("fout: {e:#}");
    e.to_string()
}

fn kopie_pad(app: &AppHandle, email: &str) -> anyhow::Result<PathBuf> {
    let map = app.path().app_data_dir()?;
    std::fs::create_dir_all(&map)?;
    let veilig: String = email.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    Ok(map.join(format!("kopie-{veilig}.sqlite")))
}

/// De toestand van een account; bij de eerste keer worden kopie en synchronisatie gestart.
fn haal(app: &AppHandle, t: &Toestand, email: &str) -> Uitkomst<Arc<AccountToestand>> {
    let mut lijst = t.accounts.lock().unwrap();
    if let Some(a) = lijst.get(email) {
        return Ok(a.clone());
    }
    account::zoek(app, email).map_err(tekst)?;
    let cache = Arc::new(Cache::open(&kopie_pad(app, email).map_err(tekst)?).map_err(tekst)?);
    let s = Arc::new(sync::Sync::default());
    sync::start(app.clone(), email.to_string(), cache.clone(), s.clone());
    let at = Arc::new(AccountToestand { verbinding: Mutex::new(None), cache, sync: s });
    lijst.insert(email.to_string(), at.clone());
    Ok(at)
}

/// Voert `$werk` uit op de verbinding van het account (`$v`), en bouwt die zo nodig (opnieuw) op.
/// Een macro in plaats van een functie met een async closure: zo blijft de future `Send`.
macro_rules! met_verbinding {
    ($app:expr, $at:expr, $email:expr, |$v:ident| $werk:expr) => {{
        let mut slot = $at.verbinding.lock().await;
        let mut uitkomst = Err(String::new());
        for poging in 0..2 {
            if slot.is_none() {
                let acc = account::zoek(&$app, &$email).map_err(tekst)?;
                let ww = account::wachtwoord(&acc.email).map_err(tekst)?;
                *slot = Some(imap::verbind(&acc.imap_host, acc.imap_poort, &acc.email, &ww, false).await.map_err(tekst)?);
            }
            let $v = slot.as_mut().unwrap();
            match $werk {
                Ok(w) => {
                    uitkomst = Ok(w);
                    break;
                }
                // Eén keer opnieuw verbinden: een verbinding die lang stil lag, is vaak door de server gesloten.
                Err(e) if poging == 0 => {
                    eprintln!("opnieuw verbinden na: {e:#}");
                    *slot = None;
                }
                Err(e) => uitkomst = Err(tekst(e)),
            }
        }
        uitkomst
    }};
}

fn gewijzigd(app: &AppHandle, account: &str, map: &str) {
    let _ = app.emit("map-gewijzigd", Gebeurtenis { account, map });
}

// ── Accounts ────────────────────────────────────────────────────────────────

#[tauri::command]
fn accounts(app: AppHandle) -> Uitkomst<Vec<Account>> {
    account::alle(&app).map_err(tekst)
}

/// Voorgestelde serverinstellingen bij een mailadres.
#[tauri::command]
fn account_standaard(email: String) -> Account {
    Account::standaard(&email)
}

/// Test de inlog en bewaart het account (nieuw, of gewijzigde instellingen).
/// Wachtwoord leeg = het bewaarde wachtwoord houden.
#[tauri::command]
async fn account_bewaren(app: AppHandle, t: State<'_, Toestand>, acc: Account, wachtwoord: String) -> Uitkomst<Vec<Account>> {
    let mut acc = acc;
    acc.email = acc.email.trim().to_lowercase();
    let ww = if wachtwoord.is_empty() { account::wachtwoord(&acc.email).map_err(tekst)? } else { wachtwoord.clone() };
    let v = imap::verbind(&acc.imap_host, acc.imap_poort, &acc.email, &ww, false).await.map_err(tekst)?;
    account::bewaar(&app, &acc, &wachtwoord).map_err(tekst)?;
    let at = haal(&app, &t, &acc.email)?;
    if let Some(oud) = at.verbinding.lock().await.replace(v) {
        oud.afmelden().await;
    }
    at.sync.signaal.notify_one();
    account::alle(&app).map_err(tekst)
}

/// Haalt een account van deze pc: wachtwoord, instellingen en lokale kopie gaan weg.
/// De mail zelf blijft gewoon op de server.
#[tauri::command]
async fn account_verwijderen(app: AppHandle, t: State<'_, Toestand>, email: String) -> Uitkomst<Vec<Account>> {
    let weg = t.accounts.lock().unwrap().remove(&email);
    if let Some(at) = weg {
        at.sync.stop();
        if let Some(v) = at.verbinding.lock().await.take() {
            v.afmelden().await;
        }
        at.cache.wis_alles().map_err(tekst)?;
    }
    account::verwijder(&app, &email).map_err(tekst)?;
    if let Ok(pad) = kopie_pad(&app, &email) {
        for achtervoegsel in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{achtervoegsel}", pad.display()));
        }
    }
    account::alle(&app).map_err(tekst)
}

// ── Lezen ───────────────────────────────────────────────────────────────────

#[tauri::command]
async fn mappen(app: AppHandle, t: State<'_, Toestand>, account: String) -> Uitkomst<Vec<imap::Map>> {
    let at = haal(&app, &t, &account)?;
    met_verbinding!(app, at, account, |v| v.mappen().await)
}

#[derive(Serialize)]
struct Pagina {
    totaal: u32,
    /// false zolang de map voor het eerst wordt klaargezet
    klaar: bool,
    berichten: Vec<mail::Kop>,
}

/// Een stuk van de berichtenlijst, uit de lokale kopie (direct).
#[tauri::command]
fn berichten(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, vanaf: u32, aantal: u32) -> Uitkomst<Pagina> {
    let at = haal(&app, &t, &account)?;
    let klaar = at.cache.is_klaar(&map).map_err(tekst)?;
    if vanaf == 0 {
        // Deze map krijgt voorrang bij het bijwerken; is hij nog niet klaar, dan meteen beginnen.
        let anders = at.sync.voorrang.lock().unwrap().replace(map.clone()).as_ref() != Some(&map);
        if !klaar && anders {
            at.sync.signaal.notify_one();
        }
    }
    let (totaal, berichten) = at.cache.koppen(&map, vanaf, aantal).map_err(tekst)?;
    Ok(Pagina { totaal, klaar, berichten })
}

/// Zoeken op afzender en onderwerp in alle mappen (lokale kopie, direct).
#[tauri::command]
fn zoek(app: AppHandle, t: State<'_, Toestand>, account: String, zoekterm: String) -> Uitkomst<Vec<mail::Kop>> {
    haal(&app, &t, &account)?.cache.zoek(&zoekterm, 300).map_err(tekst)
}

/// Zoeken in de volledige tekst van de mails in één map, door de server.
#[tauri::command]
async fn zoek_in_tekst(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, zoekterm: String) -> Uitkomst<Vec<mail::Kop>> {
    let at = haal(&app, &t, &account)?;
    let uids = met_verbinding!(app, at, account, |v| v.zoek_tekst(&map, &zoekterm).await)?;
    at.cache.koppen_bij(&map, &uids).map_err(tekst)
}

/// Adressen voor het aanvullen van Aan/Cc, uit de afzenders in de kopie.
#[tauri::command]
fn adressen(app: AppHandle, t: State<'_, Toestand>, account: String, begin: String) -> Uitkomst<Vec<String>> {
    haal(&app, &t, &account)?.cache.adressen(&begin, 8).map_err(tekst)
}

/// Opent een bericht. Met `markeer` wordt het meteen als gelezen gemarkeerd (zoals elk mailprogramma).
#[tauri::command]
async fn bericht(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, uid: u32, markeer: bool) -> Uitkomst<mail::Bericht> {
    let at = haal(&app, &t, &account)?;
    let b = met_verbinding!(app, at, account, |v| v.bericht(&map, uid).await)?;
    if markeer {
        met_verbinding!(app, at, account, |v| v.markeer(&map, &[uid], true).await)?;
        at.cache.zet_gelezen(&map, &[uid], true).map_err(tekst)?;
        gewijzigd(&app, &account, &map);
    }
    Ok(b)
}

// ── Ordenen ─────────────────────────────────────────────────────────────────

#[tauri::command]
async fn markeer(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, uids: Vec<u32>, gelezen: bool) -> Uitkomst<()> {
    let at = haal(&app, &t, &account)?;
    met_verbinding!(app, at, account, |v| v.markeer(&map, &uids, gelezen).await)?;
    at.cache.zet_gelezen(&map, &uids, gelezen).map_err(tekst)?;
    gewijzigd(&app, &account, &map);
    Ok(())
}

async fn verplaats_naar(app: &AppHandle, at: &AccountToestand, account: &str, map: &str, uids: &[u32], doel: &str) -> Uitkomst<()> {
    met_verbinding!(app, at, account, |v| v.verplaats(map, uids, doel).await)?;
    at.cache.werk_bij(map, uids, &[]).map_err(tekst)?;
    gewijzigd(app, account, map);
    // De doelmap haalt de achtergrondverbinding meteen bij.
    at.sync.werk_bij(doel);
    Ok(())
}

#[tauri::command]
async fn verplaats(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, uids: Vec<u32>, doel: String) -> Uitkomst<()> {
    let at = haal(&app, &t, &account)?;
    verplaats_naar(&app, &at, &account, &map, &uids, &doel).await
}

/// Naar de prullenbak (nooit meteen definitief weg).
#[tauri::command]
async fn verwijder(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, uids: Vec<u32>) -> Uitkomst<()> {
    let at = haal(&app, &t, &account)?;
    let prullenbak = met_verbinding!(app, at, account, |v| v.vaste_map("prullenbak").await)?
        .ok_or("Deze mailbox heeft geen prullenbak")?
        .0;
    if prullenbak == map {
        return Err("Dit bericht staat al in de prullenbak".into());
    }
    verplaats_naar(&app, &at, &account, &map, &uids, &prullenbak).await
}

/// Naar het jaararchief `Archives.<jaar van de mail>` (map wordt zo nodig aangemaakt).
#[tauri::command]
async fn archiveer(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, berichten: Vec<(u32, i64)>) -> Uitkomst<()> {
    let at = haal(&app, &t, &account)?;
    let (basis, scheiding) = met_verbinding!(app, at, account, |v| v.vaste_map("archief").await)?
        .unwrap_or_else(|| ("Archives".into(), ".".into()));
    let mut per_jaar: HashMap<Option<i32>, Vec<u32>> = HashMap::new();
    for (uid, datum) in berichten {
        per_jaar.entry(mail::jaar_van(datum)).or_default().push(uid);
    }
    for (jaar, uids) in per_jaar {
        let doel = match jaar {
            Some(j) => format!("{basis}{scheiding}{j}"),
            None => basis.clone(),
        };
        met_verbinding!(app, at, account, |v| v.zorg_map(&doel).await)?;
        verplaats_naar(&app, &at, &account, &map, &uids, &doel).await?;
    }
    Ok(())
}

// ── Bijlagen ────────────────────────────────────────────────────────────────

async fn bijlage(app: &AppHandle, t: &Toestand, account: &str, map: &str, uid: u32, index: usize) -> Uitkomst<(String, Vec<u8>)> {
    let at = haal(app, t, account)?;
    let ruw = met_verbinding!(app, at, account, |v| v.ruw(map, uid).await)?;
    let mut alle = mail::bijlagen_uit(&ruw).map_err(tekst)?;
    if index >= alle.len() {
        return Err("Bijlage niet gevonden".into());
    }
    let (naam, _, inhoud) = alle.swap_remove(index);
    Ok((naam, inhoud))
}

fn veilige_naam(naam: &str) -> String {
    let n: String = naam.chars().map(|c| if r#"/\:*?"<>|"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    if n.trim().is_empty() { "bijlage".into() } else { n }
}

/// Opent een bijlage met het standaardprogramma (pdf-lezer, Excel, …).
#[tauri::command]
async fn bijlage_openen(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, uid: u32, index: usize) -> Uitkomst<()> {
    let (naam, inhoud) = bijlage(&app, &t, &account, &map, uid, index).await?;
    let map_tijdelijk = std::env::temp_dir().join("pro-art-mail").join(format!("{uid}-{index}"));
    std::fs::create_dir_all(&map_tijdelijk).map_err(|e| e.to_string())?;
    let pad = map_tijdelijk.join(veilige_naam(&naam));
    std::fs::write(&pad, inhoud).map_err(|e| e.to_string())?;
    app.opener().open_path(pad.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
async fn bijlage_opslaan(app: AppHandle, t: State<'_, Toestand>, account: String, map: String, uid: u32, index: usize, pad: PathBuf) -> Uitkomst<()> {
    let (_, inhoud) = bijlage(&app, &t, &account, &map, uid, index).await?;
    std::fs::write(&pad, inhoud).with_context(|| format!("Kan {} niet opslaan", pad.display())).map_err(tekst)
}

// ── Schrijven ───────────────────────────────────────────────────────────────

async fn bouw_concept(app: &AppHandle, at: &AccountToestand, c: &versturen::Concept) -> Uitkomst<(Account, lettre::Message)> {
    let acc = account::zoek(app, &c.account).map_err(tekst)?;
    let mut overgenomen = vec![];
    if let Some(v) = &c.bijlagen_van {
        let ruw = met_verbinding!(app, at, c.account, |w| w.ruw(&v.map, v.uid).await)?;
        overgenomen = mail::bijlagen_uit(&ruw).map_err(tekst)?;
        overgenomen.retain(|(naam, _, _)| !c.zonder_bijlagen.contains(naam));
    }
    let m = versturen::bouw(&acc, c, overgenomen).map_err(tekst)?;
    Ok((acc, m))
}

/// Haalt eerdere versies van dit concept (zelfde Message-ID) uit Concepten.
async fn wis_oude_concepten(app: &AppHandle, at: &AccountToestand, account: &str, message_id: &str) -> Uitkomst<()> {
    if let Some((concepten, _)) = met_verbinding!(app, at, account, |v| v.vaste_map("concepten").await)? {
        let oud = met_verbinding!(app, at, account, |v| v.zoek_message_id(&concepten, message_id).await)?;
        if !oud.is_empty() {
            met_verbinding!(app, at, account, |v| v.wis(&concepten, &oud).await)?;
            at.cache.werk_bij(&concepten, &oud, &[]).map_err(tekst)?;
            gewijzigd(app, account, &concepten);
        }
    }
    Ok(())
}

#[tauri::command]
async fn verstuur(app: AppHandle, t: State<'_, Toestand>, concept: versturen::Concept) -> Uitkomst<()> {
    let account = concept.account.clone();
    let at = haal(&app, &t, &account)?;
    if concept.aan.iter().chain(&concept.cc).chain(&concept.bcc).all(|a| a.trim().is_empty()) {
        return Err("Vul minstens één ontvanger in".into());
    }
    let (acc, mail) = bouw_concept(&app, &at, &concept).await?;
    let ww = account::wachtwoord(&acc.email).map_err(tekst)?;
    versturen::verstuur(&acc, &ww, &mail).await.map_err(tekst)?;

    // Vanaf hier is de mail weg; fouten hierna melden we, maar de mail is wel verstuurd.
    let mut naderhand = Vec::new();
    let verzonden = if acc.verzonden_map.is_empty() {
        met_verbinding!(app, at, account, |v| v.vaste_map("verzonden").await).ok().flatten().map(|m| m.0)
    } else {
        Some(acc.verzonden_map.clone())
    };
    match verzonden {
        Some(map) => match met_verbinding!(app, at, account, |v| v.bewaar_in(&map, "(\\Seen)", &mail.formatted()).await) {
            Ok(()) => at.sync.werk_bij(&map),
            Err(e) => naderhand.push(format!("bewaren in Verzonden mislukt ({e})")),
        },
        None => naderhand.push("geen map Verzonden gevonden".into()),
    }
    if let Some(o) = &concept.antwoord_op {
        if met_verbinding!(app, at, account, |v| v.markeer_beantwoord(&o.map, o.uid).await).is_ok() {
            gewijzigd(&app, &account, &o.map);
        }
    }
    if let Err(e) = wis_oude_concepten(&app, &at, &account, &concept.message_id).await {
        naderhand.push(format!("concept opruimen mislukt ({e})"));
    }
    at.sync.signaal.notify_one();
    if naderhand.is_empty() {
        Ok(())
    } else {
        Err(format!("De mail is verstuurd, maar: {}", naderhand.join("; ")))
    }
}

/// Bewaart het concept in Concepten (vervangt een eerdere versie van hetzelfde concept).
#[tauri::command]
async fn bewaar_concept(app: AppHandle, t: State<'_, Toestand>, concept: versturen::Concept) -> Uitkomst<()> {
    let account = concept.account.clone();
    let at = haal(&app, &t, &account)?;
    let (_, mail) = bouw_concept(&app, &at, &concept).await?;
    let (concepten, _) = met_verbinding!(app, at, account, |v| v.vaste_map("concepten").await)?
        .ok_or("Deze mailbox heeft geen map Concepten")?;
    wis_oude_concepten(&app, &at, &account, &concept.message_id).await?;
    met_verbinding!(app, at, account, |v| v.bewaar_in(&concepten, "(\\Seen \\Draft)", &mail.formatted()).await)?;
    at.sync.werk_bij(&concepten);
    Ok(())
}

#[tauri::command]
fn ondertekeningen(app: AppHandle) -> Uitkomst<Vec<Ondertekening>> {
    account::ondertekeningen(&app).map_err(tekst)
}

#[tauri::command]
fn bewaar_ondertekeningen(app: AppHandle, lijst: Vec<Ondertekening>) -> Uitkomst<()> {
    account::bewaar_ondertekeningen(&app, &lijst).map_err(tekst)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Toestand::default())
        .setup(|app| {
            // Kopie van versie 1 (één account) is overbodig: wordt per account opnieuw opgebouwd.
            if let Ok(map) = app.path().app_data_dir() {
                for n in ["kopie.sqlite", "kopie.sqlite-wal", "kopie.sqlite-shm"] {
                    let _ = std::fs::remove_file(map.join(n));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            accounts,
            account_standaard,
            account_bewaren,
            account_verwijderen,
            mappen,
            berichten,
            zoek,
            zoek_in_tekst,
            adressen,
            bericht,
            markeer,
            verplaats,
            verwijder,
            archiveer,
            bijlage_openen,
            bijlage_opslaan,
            verstuur,
            bewaar_concept,
            ondertekeningen,
            bewaar_ondertekeningen
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
