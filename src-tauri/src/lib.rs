mod account;
mod imap;
mod mail;

use tauri::{AppHandle, State};
use tokio::sync::Mutex;

use account::Account;

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
async fn inloggen(app: AppHandle, t: State<'_, Toestand>, email: String, wachtwoord: String) -> Uitkomst<Account> {
    let acc = Account::voor(email.trim());
    let v = imap::verbind(&acc.imap_host, acc.imap_poort, &acc.email, &wachtwoord).await.map_err(tekst)?;
    account::bewaar(&app, &acc, &wachtwoord).map_err(tekst)?;
    *t.verbinding.lock().await = Some(v);
    Ok(acc)
}

#[tauri::command]
async fn afmelden(app: AppHandle, t: State<'_, Toestand>) -> Uitkomst<()> {
    if let Some(v) = t.verbinding.lock().await.take() {
        v.afmelden().await;
    }
    account::vergeet(&app).map_err(tekst)
}

#[tauri::command]
async fn mappen(app: AppHandle, t: State<'_, Toestand>) -> Uitkomst<Vec<imap::Map>> {
    met_verbinding!(app, t, |v| v.mappen().await)
}

#[tauri::command]
async fn berichten(app: AppHandle, t: State<'_, Toestand>, map: String, pagina: u32) -> Uitkomst<imap::Pagina> {
    met_verbinding!(app, t, |v| v.berichten(&map, pagina, 50).await)
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
        .invoke_handler(tauri::generate_handler![huidig_account, inloggen, afmelden, mappen, berichten, bericht])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
