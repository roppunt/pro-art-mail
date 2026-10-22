//! Lokale kopie van de kopregels (afzender, onderwerp, datum, vlaggen) van alle mappen,
//! zodat bladeren en zoeken direct gaan. De mailtekst zelf blijft op de server en wordt
//! pas opgehaald als je een bericht opent. De database is alleen een kopie: weggooien kan
//! altijd, de app bouwt hem dan opnieuw op.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

use crate::mail::Kop;

pub struct Cache {
    db: Mutex<Connection>,
}

/// Vlaggen zoals ze in de kopie staan: (gelezen, beantwoord).
pub type Vlaggen = (bool, bool);

impl Cache {
    pub fn open(pad: &Path) -> Result<Self> {
        let db = Connection::open(pad)?;
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS mappen (
                 pad TEXT PRIMARY KEY,
                 uidvalidity INTEGER NOT NULL,
                 klaar INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS koppen (
                 map TEXT NOT NULL,
                 uid INTEGER NOT NULL,
                 van_naam TEXT NOT NULL,
                 van_adres TEXT NOT NULL,
                 onderwerp TEXT NOT NULL,
                 datum INTEGER NOT NULL,
                 gelezen INTEGER NOT NULL,
                 beantwoord INTEGER NOT NULL,
                 grootte INTEGER NOT NULL,
                 bijlage INTEGER NOT NULL,
                 -- kleine letters van afzender + onderwerp, om snel in te zoeken
                 zoek TEXT NOT NULL,
                 PRIMARY KEY (map, uid)
             );
             CREATE INDEX IF NOT EXISTS koppen_op_datum ON koppen (map, datum DESC);",
        )?;
        Ok(Cache { db: Mutex::new(db) })
    }

    /// Begint een map opnieuw als de server een andere UIDVALIDITY geeft (dan kloppen de UID's niet meer).
    pub fn controleer_map(&self, pad: &str, uidvalidity: u32) -> Result<()> {
        let db = self.db.lock().unwrap();
        let bekend: Option<u32> = db
            .query_row("SELECT uidvalidity FROM mappen WHERE pad = ?1", [pad], |r| r.get(0))
            .optional()?;
        if bekend != Some(uidvalidity) {
            db.execute("DELETE FROM koppen WHERE map = ?1", [pad])?;
            db.execute(
                "INSERT INTO mappen (pad, uidvalidity, klaar) VALUES (?1, ?2, 0)
                 ON CONFLICT (pad) DO UPDATE SET uidvalidity = ?2, klaar = 0",
                params![pad, uidvalidity],
            )?;
        }
        Ok(())
    }

    pub fn zet_klaar(&self, pad: &str) -> Result<()> {
        self.db.lock().unwrap().execute("UPDATE mappen SET klaar = 1 WHERE pad = ?1", [pad])?;
        Ok(())
    }

    pub fn is_klaar(&self, pad: &str) -> Result<bool> {
        let db = self.db.lock().unwrap();
        Ok(db
            .query_row("SELECT klaar FROM mappen WHERE pad = ?1", [pad], |r| r.get::<_, i64>(0))
            .optional()?
            .unwrap_or(0)
            == 1)
    }

    pub fn vlaggen(&self, pad: &str) -> Result<HashMap<u32, Vlaggen>> {
        let db = self.db.lock().unwrap();
        let mut st = db.prepare("SELECT uid, gelezen, beantwoord FROM koppen WHERE map = ?1")?;
        let rijen = st.query_map([pad], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?;
        Ok(rijen.collect::<Result<_, _>>()?)
    }

    /// Berichten zonder datum (van vóór de terugval op de ontvangstdatum): die worden opnieuw opgehaald.
    pub fn zonder_datum(&self, pad: &str) -> Result<Vec<u32>> {
        let db = self.db.lock().unwrap();
        let mut st = db.prepare("SELECT uid FROM koppen WHERE map = ?1 AND datum <= 0")?;
        let uids = st.query_map([pad], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(uids)
    }

    /// Verwerkt in één transactie: verwijderde berichten weg, gewijzigde vlaggen bij.
    pub fn werk_bij(&self, pad: &str, weg: &[u32], vlaggen: &[(u32, Vlaggen)]) -> Result<()> {
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        {
            let mut del = tx.prepare("DELETE FROM koppen WHERE map = ?1 AND uid = ?2")?;
            for uid in weg {
                del.execute(params![pad, uid])?;
            }
            let mut upd = tx.prepare("UPDATE koppen SET gelezen = ?3, beantwoord = ?4 WHERE map = ?1 AND uid = ?2")?;
            for (uid, (g, b)) in vlaggen {
                upd.execute(params![pad, uid, g, b])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn voeg_toe(&self, pad: &str, koppen: &[Kop]) -> Result<()> {
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        {
            let mut ins = tx.prepare(
                "INSERT OR REPLACE INTO koppen
                 (map, uid, van_naam, van_adres, onderwerp, datum, gelezen, beantwoord, grootte, bijlage, zoek)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for k in koppen {
                let zoek = format!("{} {} {}", k.van_naam, k.van_adres, k.onderwerp).to_lowercase();
                ins.execute(params![
                    pad, k.uid, k.van_naam, k.van_adres, k.onderwerp, k.datum, k.gelezen, k.beantwoord, k.grootte, k.bijlage, zoek
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn rij(r: &rusqlite::Row) -> rusqlite::Result<Kop> {
        Ok(Kop {
            map: r.get(0)?,
            uid: r.get(1)?,
            van_naam: r.get(2)?,
            van_adres: r.get(3)?,
            onderwerp: r.get(4)?,
            datum: r.get(5)?,
            gelezen: r.get(6)?,
            beantwoord: r.get(7)?,
            grootte: r.get(8)?,
            bijlage: r.get(9)?,
        })
    }

    const VELDEN: &str = "map, uid, van_naam, van_adres, onderwerp, datum, gelezen, beantwoord, grootte, bijlage";

    /// Een stuk van de lijst, nieuwste eerst.
    pub fn koppen(&self, pad: &str, vanaf: u32, aantal: u32) -> Result<(u32, Vec<Kop>)> {
        let db = self.db.lock().unwrap();
        let totaal: u32 = db.query_row("SELECT COUNT(*) FROM koppen WHERE map = ?1", [pad], |r| r.get(0))?;
        let mut st = db.prepare(&format!(
            "SELECT {} FROM koppen WHERE map = ?1 ORDER BY datum DESC, uid DESC LIMIT ?2 OFFSET ?3",
            Self::VELDEN
        ))?;
        let koppen = st.query_map(params![pad, aantal, vanaf], Self::rij)?.collect::<Result<_, _>>()?;
        Ok((totaal, koppen))
    }

    /// Zoeken op afzender en onderwerp in alle mappen. Elk woord moet ergens voorkomen.
    pub fn zoek(&self, tekst: &str, max: u32) -> Result<Vec<Kop>> {
        let woorden: Vec<String> = tekst.to_lowercase().split_whitespace().map(|w| format!("%{}%", w.replace(['%', '_'], ""))).collect();
        if woorden.is_empty() {
            return Ok(vec![]);
        }
        let voorwaarden = vec!["zoek LIKE ?"; woorden.len()].join(" AND ");
        let db = self.db.lock().unwrap();
        let mut st = db.prepare(&format!(
            "SELECT {} FROM koppen WHERE {voorwaarden} ORDER BY datum DESC LIMIT {max}",
            Self::VELDEN
        ))?;
        let koppen = st.query_map(rusqlite::params_from_iter(woorden), Self::rij)?.collect::<Result<_, _>>()?;
        Ok(koppen)
    }

    /// Koppen bij UID's die de server bij een zoekopdracht in de tekst teruggaf.
    pub fn koppen_bij(&self, pad: &str, uids: &[u32]) -> Result<Vec<Kop>> {
        let db = self.db.lock().unwrap();
        let mut st = db.prepare(&format!("SELECT {} FROM koppen WHERE map = ?1 AND uid = ?2", Self::VELDEN))?;
        let mut uit = Vec::new();
        for uid in uids {
            if let Some(k) = st.query_row(params![pad, uid], Self::rij).optional()? {
                uit.push(k);
            }
        }
        uit.sort_by(|a, b| b.datum.cmp(&a.datum));
        Ok(uit)
    }

    pub fn zet_gelezen(&self, pad: &str, uids: &[u32], gelezen: bool) -> Result<()> {
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        for uid in uids {
            tx.execute("UPDATE koppen SET gelezen = ?3 WHERE map = ?1 AND uid = ?2", params![pad, uid, gelezen])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Adressen waarvan mail binnenkwam, als "Naam <adres>", vaakst gebruikte eerst.
    pub fn adressen(&self, begin: &str, max: u32) -> Result<Vec<String>> {
        let b = begin.trim().to_lowercase().replace(['%', '_'], "");
        if b.len() < 2 {
            return Ok(vec![]);
        }
        let db = self.db.lock().unwrap();
        let mut st = db.prepare(
            "SELECT van_naam, van_adres FROM koppen
             WHERE van_adres != '' AND (lower(van_adres) LIKE ?1 OR lower(van_naam) LIKE ?1 OR lower(van_naam) LIKE ?2)
             GROUP BY lower(van_adres) ORDER BY COUNT(*) DESC LIMIT ?3",
        )?;
        let rijen = st.query_map(params![format!("{b}%"), format!("% {b}%"), max], |r| {
            let (naam, adres): (String, String) = (r.get(0)?, r.get(1)?);
            Ok(if naam.is_empty() || naam == adres { adres } else { format!("{naam} <{adres}>") })
        })?;
        Ok(rijen.collect::<Result<_, _>>()?)
    }

    /// Na afmelden: niets van dit account op de pc laten staan.
    pub fn wis_alles(&self) -> Result<()> {
        self.db.lock().unwrap().execute_batch("DELETE FROM koppen; DELETE FROM mappen; VACUUM;")?;
        Ok(())
    }

    /// Mappen die niet meer op de server staan, uit de kopie halen.
    pub fn ruim_mappen_op(&self, bestaand: &[String]) -> Result<()> {
        let db = self.db.lock().unwrap();
        let mut st = db.prepare("SELECT pad FROM mappen")?;
        let alle: Vec<String> = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        for pad in alle.iter().filter(|p| !bestaand.contains(p)) {
            db.execute("DELETE FROM koppen WHERE map = ?1", [pad])?;
            db.execute("DELETE FROM mappen WHERE pad = ?1", [pad])?;
        }
        Ok(())
    }
}
