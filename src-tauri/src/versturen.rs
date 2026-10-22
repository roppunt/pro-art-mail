//! Een concept omzetten in een echte mail (lettre) en versturen via SMTP.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use lettre::message::header::{ContentType, HeaderName, HeaderValue};
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;

use crate::account::Account;

/// Verwijzing naar een bestaand bericht op de server.
#[derive(Deserialize, Clone)]
pub struct Verwijzing {
    pub map: String,
    pub uid: u32,
}

#[derive(Deserialize, Clone)]
pub struct Concept {
    pub account: String,
    pub aan: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    #[serde(default)]
    pub bcc: Vec<String>,
    pub onderwerp: String,
    pub html: String,
    pub tekst: String,
    /// Bestanden van deze pc.
    #[serde(default)]
    pub bestanden: Vec<PathBuf>,
    /// Bijlagen overnemen uit dit bericht (doorsturen, of een eerder bewaard concept).
    pub bijlagen_van: Option<Verwijzing>,
    /// Weg te laten bijlagen uit `bijlagen_van` (op naam).
    #[serde(default)]
    pub zonder_bijlagen: Vec<String>,
    /// Bij beantwoorden: het oorspronkelijke bericht (krijgt de vlag "beantwoord").
    pub antwoord_op: Option<Verwijzing>,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    /// Vaste Message-ID van dit concept, zodat een eerdere versie in Concepten te vinden is.
    pub message_id: String,
}

fn adres(s: &str) -> Result<Mailbox> {
    s.trim().parse::<Mailbox>().with_context(|| format!("\"{}\" is geen geldig mailadres", s.trim()))
}

/// Bouwt de mail. `bijlagen` = al opgehaalde bijlagen uit `bijlagen_van`.
pub fn bouw(acc: &Account, c: &Concept, bijlagen: Vec<(String, String, Vec<u8>)>) -> Result<Message> {
    let van = Mailbox::new((!acc.naam.is_empty()).then(|| acc.naam.clone()), acc.email.parse()?);
    let mut b = Message::builder().from(van).subject(c.onderwerp.clone());
    for a in c.aan.iter().filter(|a| !a.trim().is_empty()) {
        b = b.to(adres(a)?);
    }
    for a in c.cc.iter().filter(|a| !a.trim().is_empty()) {
        b = b.cc(adres(a)?);
    }
    for a in c.bcc.iter().filter(|a| !a.trim().is_empty()) {
        b = b.bcc(adres(a)?);
    }
    if let Some(id) = &c.in_reply_to {
        b = b.in_reply_to(id.clone());
    }
    if let Some(r) = &c.references {
        b = b.references(r.clone());
    }
    b = b.message_id(Some(c.message_id.clone())).raw_header(HeaderValue::new(
        HeaderName::new_from_ascii_str("X-Mailer"),
        "Pro-Art Mail".into(),
    ));

    let mut alle: Vec<(String, String, Vec<u8>)> = bijlagen;
    for pad in &c.bestanden {
        let inhoud = std::fs::read(pad).with_context(|| format!("Kan {} niet lezen", pad.display()))?;
        let naam = pad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "bijlage".into());
        let soort = mime_guess::from_path(pad).first_or_octet_stream().to_string();
        alle.push((naam, soort, inhoud));
    }

    let tekst = MultiPart::alternative_plain_html(c.tekst.clone(), c.html.clone());
    let lijf = if alle.is_empty() {
        tekst
    } else {
        let mut m = MultiPart::mixed().multipart(tekst);
        for (naam, soort, inhoud) in alle {
            let ct = ContentType::parse(&soort).unwrap_or(ContentType::parse("application/octet-stream")?);
            let deel: SinglePart = Attachment::new(naam).body(inhoud, ct);
            m = m.singlepart(deel);
        }
        m
    };
    Ok(b.multipart(lijf)?)
}

pub async fn verstuur(acc: &Account, wachtwoord: &str, mail: &Message) -> Result<()> {
    let bouwer = if acc.smtp_poort == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&acc.smtp_host)?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&acc.smtp_host)?
    };
    let transport = bouwer
        .port(acc.smtp_poort)
        .credentials(Credentials::new(acc.email.clone(), wachtwoord.to_string()))
        .timeout(Some(std::time::Duration::from_secs(60)))
        .build();
    transport.send(mail.clone()).await.map_err(|e| {
        if e.is_permanent() {
            anyhow!("De mailserver weigerde de mail: {e}")
        } else {
            anyhow!("Versturen mislukt (probeer het zo nog eens): {e}")
        }
    })?;
    Ok(())
}
