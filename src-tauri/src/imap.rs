//! Verbinding met de IMAP-server. Stap 1: alleen lezen — mappen worden met EXAMINE geopend
//! en berichten met BODY.PEEK opgehaald, zodat er op de server niets verandert (ook niet gelezen/ongelezen).

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_imap::types::{Flag, NameAttribute};
use async_imap::Session;
use futures::TryStreamExt;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

use crate::mail;

pub type ImapSessie = Session<TlsStream<TcpStream>>;

pub struct Verbinding {
    sessie: ImapSessie,
    /// Welke map nu met EXAMINE open staat, om niet bij elke opdracht opnieuw te openen.
    open_map: Option<String>,
    aantal: u32,
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
}

#[derive(Serialize)]
pub struct Pagina {
    pub totaal: u32,
    pub berichten: Vec<mail::Kop>,
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
    Ok(Verbinding { sessie, open_map: None, aantal: 0 })
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
                _ => kort,
            };
            uit.push(Map { pad, naam, soort: soort.into(), scheiding });
        }
        Ok(uit)
    }

    async fn open(&mut self, map: &str) -> Result<()> {
        if self.open_map.as_deref() != Some(map) {
            let mb = self.sessie.examine(map).await.with_context(|| format!("Kan map {map} niet openen"))?;
            self.aantal = mb.exists;
            self.open_map = Some(map.to_string());
        } else {
            // Even vragen of er nieuwe mail is; NOOP werkt het aantal bij.
            self.sessie.noop().await?;
            let mb = self.sessie.examine(map).await?;
            self.aantal = mb.exists;
        }
        Ok(())
    }

    /// Een pagina koppen, nieuwste eerst. Pagina 0 = de nieuwste `per_pagina` berichten.
    pub async fn berichten(&mut self, map: &str, pagina: u32, per_pagina: u32) -> Result<Pagina> {
        self.open(map).await?;
        let totaal = self.aantal;
        if totaal == 0 || pagina * per_pagina >= totaal {
            return Ok(Pagina { totaal, berichten: vec![] });
        }
        let hoog = totaal - pagina * per_pagina;
        let laag = hoog.saturating_sub(per_pagina - 1).max(1);
        let fetches: Vec<_> = self
            .sessie
            .fetch(format!("{laag}:{hoog}"), "(UID FLAGS RFC822.SIZE BODY.PEEK[HEADER])")
            .await?
            .try_collect()
            .await?;
        let mut koppen: Vec<mail::Kop> = fetches
            .iter()
            .filter_map(|f| {
                let uid = f.uid?;
                let gelezen = f.flags().any(|v| matches!(v, Flag::Seen));
                let beantwoord = f.flags().any(|v| matches!(v, Flag::Answered));
                Some(mail::kop(uid, f.message, gelezen, beantwoord, f.size.unwrap_or(0), f.header()?))
            })
            .collect();
        koppen.sort_by(|a, b| b.volgnummer.cmp(&a.volgnummer));
        Ok(Pagina { totaal, berichten: koppen })
    }

    pub async fn bericht(&mut self, map: &str, uid: u32) -> Result<mail::Bericht> {
        self.open(map).await?;
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
