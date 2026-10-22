//! Verbinding met de IMAP-server. Stap 1: alleen lezen — mappen worden met EXAMINE geopend
//! en berichten met BODY.PEEK opgehaald, zodat er op de server niets verandert (ook niet gelezen/ongelezen).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_imap::types::{Flag, NameAttribute};
use async_imap::Session;
use futures::TryStreamExt;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

use crate::{cache, mail};

pub type ImapSessie = Session<TlsStream<TcpStream>>;

pub struct Verbinding {
    sessie: ImapSessie,
    /// Welke map nu met EXAMINE open staat, om niet bij elke opdracht opnieuw te openen.
    open_map: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Map {
    /// Naam zoals de server hem kent (nodig om hem te openen).
    pub pad: String,
    /// Leesbare naam, zonder `INBOX.`-voorvoegsel.
    pub naam: String,
    /// inbox | concepten | verzonden | spam | prullenbak | archief | gewoon
    pub soort: String,
    pub scheiding: String,
    pub aantal: u32,
    pub ongelezen: u32,
}


pub async fn verbind(host: &str, poort: u16, gebruiker: &str, wachtwoord: &str) -> Result<Verbinding> {
    let mut wortels = RootCertStore::empty();
    wortels.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder().with_root_certificates(wortels).with_no_client_auth();
    let tcp = tokio::time::timeout(std::time::Duration::from_secs(15), TcpStream::connect((host, poort)))
        .await
        .context("De mailserver reageert niet")?
        .context("Kan de mailserver niet bereiken")?;
    let naam = ServerName::try_from(host.to_string())?;
    let tls = TlsConnector::from(Arc::new(config))
        .connect(naam, tcp)
        .await
        .context("Beveiligde verbinding mislukt")?;
    let mut client = async_imap::Client::new(tls);
    client.read_response().await?.ok_or_else(|| anyhow!("Geen welkomstbericht van de server"))?;
    let sessie = client.login(gebruiker, wachtwoord).await.map_err(|(e, _)| match e {
        async_imap::error::Error::No(_) | async_imap::error::Error::Bad(_) => {
            anyhow!("Inloggen geweigerd: klopt het wachtwoord?")
        }
        e => anyhow!(e).context("Inloggen mislukt"),
    })?;
    Ok(Verbinding { sessie, open_map: None })
}

impl Verbinding {
    pub async fn mappen(&mut self) -> Result<Vec<Map>> {
        let namen: Vec<_> = self.sessie.list(Some(""), Some("*")).await?.try_collect().await?;
        let mut uit = Vec::new();
        for n in &namen {
            if n.attributes().iter().any(|a| matches!(a, NameAttribute::NoSelect)) {
                continue;
            }
            let pad = n.name().to_string();
            let scheiding = n.delimiter().unwrap_or(".").to_string();
            let leesbaar = utf7_imap::decode_utf7_imap(pad.clone());
            let kort = leesbaar
                .strip_prefix(&format!("INBOX{scheiding}"))
                .unwrap_or(&leesbaar)
                .to_string();
            let soort = soort_van(&pad, &kort, n.attributes());
            let naam = match soort {
                "inbox" => "Postvak IN".into(),
                "concepten" => "Concepten".into(),
                "verzonden" => "Verzonden".into(),
                "spam" => "Spam".into(),
                "prullenbak" => "Prullenbak".into(),
                "archief" => "Archief".into(),
                _ => kort,
            };
            let st = self.sessie.status(&pad, "(MESSAGES UNSEEN)").await?;
            uit.push(Map { pad, naam, soort: soort.into(), scheiding, aantal: st.exists, ongelezen: st.unseen.unwrap_or(0) });
        }
        // Dubbele vaste mappen (bv. `Sent` én `Sent Messages`, door verschillende mailprogramma's
        // aangemaakt): de volste blijft "Verzonden", de rest wordt een gewone map met zijn eigen naam.
        let mut volgorde: Vec<usize> = (0..uit.len()).collect();
        volgorde.sort_by_key(|&i| std::cmp::Reverse(uit[i].aantal));
        let mut gezien = std::collections::HashSet::new();
        for i in volgorde {
            let m = &mut uit[i];
            if m.soort != "gewoon" && m.soort != "inbox" && !gezien.insert(m.soort.clone()) {
                m.soort = "gewoon".into();
                let leesbaar = utf7_imap::decode_utf7_imap(m.pad.clone());
                m.naam = leesbaar.strip_prefix(&format!("INBOX{}", m.scheiding)).unwrap_or(&leesbaar).to_string();
            }
        }
        Ok(uit)
    }

    /// Opent een map alleen-lezen (EXAMINE). Geeft (UIDVALIDITY, aantal berichten).
    pub async fn open(&mut self, map: &str) -> Result<(u32, u32)> {
        let mb = self.sessie.examine(map).await.with_context(|| format!("Kan map {map} niet openen"))?;
        self.open_map = Some(map.to_string());
        Ok((mb.uid_validity.unwrap_or(0), mb.exists))
    }

    async fn zorg_open(&mut self, map: &str) -> Result<()> {
        if self.open_map.as_deref() != Some(map) {
            self.open(map).await?;
        }
        Ok(())
    }

    /// Alle selecteerbare mappen, zonder verdere gegevens (voor het bijwerken van de kopie).
    pub async fn mappaden(&mut self) -> Result<Vec<String>> {
        let namen: Vec<_> = self.sessie.list(Some(""), Some("*")).await?.try_collect().await?;
        Ok(namen
            .iter()
            .filter(|n| !n.attributes().iter().any(|a| matches!(a, NameAttribute::NoSelect)))
            .map(|n| n.name().to_string())
            .collect())
    }

    /// UID en vlaggen van alle berichten in de open map. Ook de manier om te zien welke berichten
    /// er niet meer zijn (ongeveer 35 bytes per bericht, dus ~1 MB voor 27.000 berichten).
    pub async fn alle_vlaggen(&mut self) -> Result<HashMap<u32, cache::Vlaggen>> {
        let fetches: Vec<_> = self.sessie.uid_fetch("1:*", "(UID FLAGS)").await?.try_collect().await?;
        Ok(fetches
            .iter()
            .filter_map(|f| {
                let gelezen = f.flags().any(|v| matches!(v, Flag::Seen));
                let beantwoord = f.flags().any(|v| matches!(v, Flag::Answered));
                Some((f.uid?, (gelezen, beantwoord)))
            })
            .collect())
    }

    /// Kopregels van de gegeven UID's (alleen de velden die de lijst nodig heeft).
    pub async fn koppen(&mut self, map: &str, uids: &[u32]) -> Result<Vec<mail::Kop>> {
        self.zorg_open(map).await?;
        let fetches: Vec<_> = self
            .sessie
            .uid_fetch(
                uid_reeks(uids),
                "(UID FLAGS RFC822.SIZE INTERNALDATE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE CONTENT-TYPE)])",
            )
            .await?
            .try_collect()
            .await?;
        Ok(fetches
            .iter()
            .filter_map(|f| {
                let gelezen = f.flags().any(|v| matches!(v, Flag::Seen));
                let beantwoord = f.flags().any(|v| matches!(v, Flag::Answered));
                let mut k = mail::kop(map, f.uid?, gelezen, beantwoord, f.size.unwrap_or(0), f.header()?);
                if k.datum <= 0 {
                    // Geen (geldige) Date-kopregel: dan de ontvangstdatum van de server.
                    k.datum = f.internal_date().map(|d| d.timestamp()).unwrap_or(0);
                }
                Some(k)
            })
            .collect())
    }

    /// Laat de server zoeken in de volledige tekst van de berichten in één map.
    pub async fn zoek_tekst(&mut self, map: &str, tekst: &str) -> Result<Vec<u32>> {
        self.zorg_open(map).await?;
        let mut voorwaarden = Vec::new();
        for woord in tekst.split_whitespace() {
            let schoon: String = woord.chars().filter(|c| !c.is_control() && *c != '"' && *c != '\\').collect();
            if !schoon.is_empty() {
                voorwaarden.push(format!("TEXT \"{schoon}\""));
            }
        }
        if voorwaarden.is_empty() {
            return Ok(vec![]);
        }
        let uids = self.sessie.uid_search(format!("CHARSET UTF-8 {}", voorwaarden.join(" "))).await?;
        Ok(uids.into_iter().collect())
    }

    /// Wacht (IDLE) op nieuws in de open map, tot `max` verstreken is of `signaal` komt.
    /// Geeft de verbinding daarna terug; IDLE neemt hem tijdelijk over.
    pub async fn wacht(self, signaal: &Notify, max: Duration) -> Result<Self> {
        let open_map = self.open_map;
        let mut handle = self.sessie.idle();
        handle.init().await?;
        {
            let (klaar, stop) = handle.wait_with_timeout(max);
            tokio::pin!(klaar);
            let mut stop = Some(stop);
            tokio::select! {
                r = &mut klaar => { r?; }
                _ = signaal.notified() => {
                    drop(stop.take());
                    klaar.await?;
                }
            }
        }
        let sessie = handle.done().await?;
        Ok(Verbinding { sessie, open_map })
    }

    pub async fn bericht(&mut self, map: &str, uid: u32) -> Result<mail::Bericht> {
        self.zorg_open(map).await?;
        let fetches: Vec<_> = self.sessie.uid_fetch(uid.to_string(), "(UID FLAGS BODY.PEEK[])").await?.try_collect().await?;
        let f = fetches.first().ok_or_else(|| anyhow!("Bericht niet gevonden (misschien verplaatst)"))?;
        let ruw = f.body().ok_or_else(|| anyhow!("Leeg bericht"))?;
        mail::bericht(uid, ruw)
    }

    pub async fn afmelden(mut self) {
        let _ = self.sessie.logout().await;
    }
}

fn soort_van(pad: &str, kort: &str, attr: &[NameAttribute<'_>]) -> &'static str {
    if pad.eq_ignore_ascii_case("INBOX") {
        return "inbox";
    }
    for a in attr {
        match a {
            NameAttribute::Drafts => return "concepten",
            NameAttribute::Sent => return "verzonden",
            NameAttribute::Junk => return "spam",
            NameAttribute::Trash => return "prullenbak",
            NameAttribute::Archive => return "archief",
            _ => {}
        }
    }
    // Dovecot zonder SPECIAL-USE: op naam herkennen.
    match kort.to_lowercase().as_str() {
        "drafts" | "concepten" => "concepten",
        "sent" | "sent items" | "sent messages" | "verzonden" | "verzonden items" => "verzonden",
        "junk" | "spam" | "ongewenste e-mail" => "spam",
        "trash" | "deleted items" | "deleted messages" | "prullenbak" | "verwijderde items" => "prullenbak",
        _ => "gewoon",
    }
}

/// Maakt van UID's een compacte reeks voor IMAP: [1,2,3,7,9,10] → "1:3,7,9:10".
fn uid_reeks(uids: &[u32]) -> String {
    let mut v = uids.to_vec();
    v.sort_unstable();
    let mut delen = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let begin = v[i];
        while i + 1 < v.len() && v[i + 1] == v[i] + 1 {
            i += 1;
        }
        delen.push(if begin == v[i] { begin.to_string() } else { format!("{begin}:{}", v[i]) });
        i += 1;
    }
    delen.join(",")
}

#[cfg(test)]
mod tests {
    #[test]
    fn uid_reeks() {
        assert_eq!(super::uid_reeks(&[9, 1, 2, 3, 7, 10]), "1:3,7,9:10");
        assert_eq!(super::uid_reeks(&[5]), "5");
    }
}
