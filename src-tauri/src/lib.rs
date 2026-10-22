mod account;
mod cache;
mod imap;
mod mail;
mod sync;

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

use account::Account;
use cache::Cache;

/// De ene open IMAP-verbinding. Valt hij weg (slaapstand, netwerk), dan wordt hij bij de
/// volgende opdracht opnieuw opgebouwd met het opgeslagen wachtwoord.
#[derive(Default)]
struct Toestand {
    verbinding: Mutex<Option<imap::Verbinding>>,
}

/// Fouten gaan als leesbare tekst naar de interface.
type Uitkomst<T> = Result<T, String>;

fn tekst(e: anyhow::Error) -> String {
    // Alleen de bovenste melding tonen; de ketting staat in de log.
    eprintln!("fout: {e:#}");
    e.to_string()
}

/// Voert `$werk` uit op de open verbinding (`$v`), en bouwt die zo nodig (opnieuw) op.
/// Een macro in plaats van een functie met een async closure: zo blijft de future `Send`.
macro_rules! met_verbinding {
    ($app:expr, $t:expr, |$v:ident| $werk:expr) => {{
        let mut slot = $t.verbinding.lock().await;
        let mut uitkomst = Err(String::new());
        for poging in 0..2 {
            if slot.is_none() {
                let acc = account::lees(&$app).map_err(tekst)?.ok_or("Nog niet ingelogd")?;
                let ww = account::wachtwoord(&acc.email).map_err(tekst)?;
                *slot = Some(imap::verbind(&acc.imap_host, acc.imap_poort, &acc.email, &ww).await.map_err(tekst)?);
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

#[tauri::command]
fn huidig_account(app: AppHandle) -> Uitkomst<Option<Account>> {
    account::lees(&app).map_err(tekst)
}

#[tauri::command]
async fn inloggen(
    app: AppHandle,
    t: State<'_, Toestand>,
    s: State<'_, Arc<sync::Sync>>,
    email: String,
    wachtwoord: String,
) -> Uitkomst<Account> {
    let acc = Account::voor(email.trim());
    let v = imap::verbind(&acc.imap_host, acc.imap_poort, &acc.email, &wachtwoord).await.map_err(tekst)?;
    account::bewaar(&app, &acc, &wachtwoord).map_err(tekst)?;
    *t.verbinding.lock().await = Some(v);
    s.signaal.notify_one();
    Ok(acc)
}

#[tauri::command]
async fn afmelden(
    app: AppHandle,
    t: State<'_, Toestand>,
    c: State<'_, Arc<Cache>>,
    s: State<'_, Arc<sync::Sync>>,
) -> Uitkomst<()> {
    if let Some(v) = t.verbinding.lock().await.take() {
        v.afmelden().await;
    }
    account::vergeet(&app).map_err(tekst)?;
    c.wis_alles().map_err(tekst)?;
    s.signaal.notify_one();
    Ok(())
}

#[tauri::command]
async fn mappen(app: AppHandle, t: State<'_, Toestand>) -> Uitkomst<Vec<imap::Map>> {
    met_verbinding!(app, t, |v| v.mappen().await)
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
fn berichten(c: State<'_, Arc<Cache>>, s: State<'_, Arc<sync::Sync>>, map: String, vanaf: u32, aantal: u32) -> Uitkomst<Pagina> {
    let klaar = c.is_klaar(&map).map_err(tekst)?;
    if vanaf == 0 {
        // Deze map krijgt voorrang bij het bijwerken; is hij nog niet klaar, dan meteen beginnen.
        let anders = s.voorrang.lock().unwrap().replace(map.clone()).as_ref() != Some(&map);
        if !klaar && anders {
            s.signaal.notify_one();
        }
    }
    let (totaal, berichten) = c.koppen(&map, vanaf, aantal).map_err(tekst)?;
    Ok(Pagina { totaal, klaar, berichten })
}

/// Zoeken op afzender en onderwerp in alle mappen (lokale kopie, direct).
#[tauri::command]
fn zoek(c: State<'_, Arc<Cache>>, zoekterm: String) -> Uitkomst<Vec<mail::Kop>> {
    c.zoek(&zoekterm, 300).map_err(tekst)
}

/// Zoeken in de volledige tekst van de mails in één map, door de server (langzamer).
#[tauri::command]
async fn zoek_in_tekst(
    app: AppHandle,
    t: State<'_, Toestand>,
    c: State<'_, Arc<Cache>>,
    map: String,
    zoekterm: String,
) -> Uitkomst<Vec<mail::Kop>> {
    let uids = met_verbinding!(app, t, |v| v.zoek_tekst(&map, &zoekterm).await)?;
    c.koppen_bij(&map, &uids).map_err(tekst)
}

#[tauri::command]
async fn bericht(app: AppHandle, t: State<'_, Toestand>, map: String, uid: u32) -> Uitkomst<mail::Bericht> {
    met_verbinding!(app, t, |v| v.bericht(&map, uid).await)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Toestand::default())
        .setup(|app| {
            let map = app.path().app_data_dir()?;
            std::fs::create_dir_all(&map)?;
            let cache = Arc::new(Cache::open(&map.join("kopie.sqlite"))?);
            let s = Arc::new(sync::Sync::default());
            sync::start(app.handle().clone(), cache.clone(), s.clone());
            app.manage(cache);
            app.manage(s);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            huidig_account,
            inloggen,
            afmelden,
            mappen,
            berichten,
            zoek,
            zoek_in_tekst,
            bericht
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
