//! Houdt de lokale kopie (cache.rs) bij, met een eigen IMAP-verbinding op de achtergrond.
//!
//! Ronde: eerst de map waar de gebruiker naar kijkt, dan het postvak IN, dan (de eerste keer
//! en daarna elk kwartier) alle andere mappen. Tussen de rondes wacht de verbinding met IDLE
//! op het postvak IN, zodat nieuwe mail binnen enkele seconden verschijnt.
//! Ook hier alleen lezen: EXAMINE en BODY.PEEK.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

use crate::{account, cache::Cache, imap};

const PER_KEER: usize = 500;
const ALLES_ELKE: Duration = Duration::from_secs(15 * 60);
const IDLE_MAX: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
pub struct Sync {
    /// Wekt de achtergrondverbinding (na inloggen, of als er een map met voorrang is).
    pub signaal: Notify,
    /// Map waar de gebruiker nu naar kijkt; die komt als eerste aan de beurt.
    pub voorrang: Mutex<Option<String>>,
    /// Account verwijderd: de achtergrondverbinding stopt.
    pub gestopt: AtomicBool,
    /// Mappen die de volgende ronde zeker bijgewerkt moeten worden (bv. Verzonden na versturen,
    /// de doelmap na verplaatsen), ook als het geen ronde voor alle mappen is.
    pub extra: Mutex<HashSet<String>>,
}

impl Sync {
    /// Werk deze map zo snel mogelijk bij.
    pub fn werk_bij(&self, map: &str) {
        self.extra.lock().unwrap().insert(map.to_string());
        self.signaal.notify_one();
    }

    pub fn stop(&self) {
        self.gestopt.store(true, Ordering::SeqCst);
        self.signaal.notify_one();
    }
}

#[derive(Serialize, Clone)]
struct Voortgang<'a> {
    account: &'a str,
    map: &'a str,
    gedaan: usize,
    totaal: usize,
}

#[derive(Serialize, Clone)]
pub struct Gebeurtenis<'a> {
    pub account: &'a str,
    pub map: &'a str,
}

#[derive(Serialize, Clone)]
struct Fout<'a> {
    account: &'a str,
    fout: String,
}

pub fn start(app: AppHandle, email: String, cache: Arc<Cache>, sync: Arc<Sync>) {
    tauri::async_runtime::spawn(async move {
        while !sync.gestopt.load(Ordering::SeqCst) {
            match ronde(&app, &email, &cache, &sync).await {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("bijwerken {email} mislukt: {e:#}");
                    let _ = app.emit("sync-fout", Fout { account: &email, fout: e.to_string() });
                    // Even wachten (of tot er iets gebeurt) en dan opnieuw verbinden.
                    let _ = tokio::time::timeout(Duration::from_secs(60), sync.signaal.notified()).await;
                }
            }
        }
    });
}

async fn ronde(app: &AppHandle, email: &str, cache: &Cache, sync: &Sync) -> Result<()> {
    let acc = account::zoek(app, email)?;
    let ww = account::wachtwoord(&acc.email)?;
    let mut v = imap::verbind(&acc.imap_host, acc.imap_poort, &acc.email, &ww, true).await?;
    let _ = app.emit("sync-fout", Fout { account: email, fout: String::new() });
    let mut laatste_alles: Option<Instant> = None;
    loop {
        let paden = v.mappaden().await?;
        cache.ruim_mappen_op(&paden)?;
        let alles = laatste_alles.is_none_or(|t| t.elapsed() > ALLES_ELKE);
        let voorrang = sync.voorrang.lock().unwrap().clone();
        let extra: Vec<String> = sync.extra.lock().unwrap().drain().collect();

        let mut volgorde: Vec<String> = Vec::new();
        for p in voorrang.iter().chain(std::iter::once(&"INBOX".to_string())) {
            if paden.contains(p) && !volgorde.contains(p) {
                volgorde.push(p.clone());
            }
        }
        for p in &paden {
            if !volgorde.contains(p) && (alles || extra.contains(p) || !cache.is_klaar(p)?) {
                volgorde.push(p.clone());
            }
        }
        for p in &volgorde {
            werk_map_bij(app, email, cache, &mut v, p).await?;
        }
        if alles {
            laatste_alles = Some(Instant::now());
        }

        // Account verwijderd of instellingen gewijzigd? Dan stoppen of opnieuw verbinden.
        if sync.gestopt.load(Ordering::SeqCst) || account::zoek(app, email).ok() != Some(acc.clone()) {
            v.afmelden().await;
            return Ok(());
        }
        v.open("INBOX").await?;
        v = v.wacht(&sync.signaal, IDLE_MAX).await?;
    }
}

async fn werk_map_bij(app: &AppHandle, account: &str, cache: &Cache, v: &mut imap::Verbinding, map: &str) -> Result<()> {
    let (uidvalidity, aantal) = v.open(map).await?;
    cache.controleer_map(map, uidvalidity)?;
    let server = if aantal > 0 { v.alle_vlaggen().await? } else { Default::default() };
    let lokaal = cache.vlaggen(map)?;

    let weg: Vec<u32> = lokaal.keys().filter(|u| !server.contains_key(u)).copied().collect();
    let gewijzigd: Vec<_> = server
        .iter()
        .filter(|(u, f)| lokaal.get(u).is_some_and(|l| l != *f))
        .map(|(u, f)| (*u, *f))
        .collect();
    let bekend: HashSet<u32> = lokaal.keys().copied().collect();
    let mut nieuw: Vec<u32> = server.keys().filter(|u| !bekend.contains(u)).copied().collect();
    nieuw.extend(cache.zonder_datum(map)?.into_iter().filter(|u| server.contains_key(u)));
    // Nieuwste eerst, zodat de lijst bovenaan meteen klopt.
    nieuw.sort_unstable_by(|a, b| b.cmp(a));

    let veranderd = !weg.is_empty() || !gewijzigd.is_empty() || !nieuw.is_empty();
    cache.werk_bij(map, &weg, &gewijzigd)?;
    for (i, stuk) in nieuw.chunks(PER_KEER).enumerate() {
        let koppen = v.koppen(map, stuk).await?;
        if koppen.is_empty() && !stuk.is_empty() {
            return Err(anyhow!("Server gaf geen kopregels terug voor {map}"));
        }
        cache.voeg_toe(map, &koppen)?;
        let gedaan = (i * PER_KEER + stuk.len()).min(nieuw.len());
        let _ = app.emit("sync-voortgang", Voortgang { account, map, gedaan, totaal: nieuw.len() });
        let _ = app.emit("map-gewijzigd", Gebeurtenis { account, map });
    }
    cache.zet_klaar(map)?;
    if veranderd {
        let _ = app.emit("map-gewijzigd", Gebeurtenis { account, map });
    }
    Ok(())
}
