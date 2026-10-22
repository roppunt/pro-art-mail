//! Mail ontleden (kopregels, tekst, bijlagen) en HTML veilig maken voor weergave.

use std::collections::HashSet;

use anyhow::{Result, anyhow};
use mail_parser::{Address, MessageParser, MimeHeaders};
use serde::Serialize;

#[derive(Serialize)]
pub struct Kop {
    pub map: String,
    pub uid: u32,
    pub van_naam: String,
    pub van_adres: String,
    pub onderwerp: String,
    /// Unix-tijd in seconden; 0 als de datum ontbreekt.
    pub datum: i64,
    pub gelezen: bool,
    pub beantwoord: bool,
    pub grootte: u32,
    pub bijlage: bool,
    /// Ster (IMAP \Flagged): in de winkel gebruikt als "moet nog gebeuren".
    pub gemarkeerd: bool,
}

#[derive(Serialize)]
pub struct Persoon {
    pub naam: String,
    pub adres: String,
}

#[derive(Serialize)]
pub struct Bijlage {
    pub naam: String,
    pub grootte: usize,
    pub soort: String,
}

#[derive(Serialize)]
pub struct Bericht {
    pub uid: u32,
    pub onderwerp: String,
    pub van: Vec<Persoon>,
    /// Reply-To; leeg = antwoord naar `van`.
    pub antwoord_aan: Vec<Persoon>,
    pub message_id: String,
    /// References + Message-ID, voor de draad bij beantwoorden.
    pub referenties: String,
    pub aan: Vec<Persoon>,
    pub cc: Vec<Persoon>,
    pub datum: i64,
    /// Opgeschoonde HTML; leeg als het bericht alleen tekst heeft.
    pub html: String,
    pub tekst: String,
    pub bijlagen: Vec<Bijlage>,
}

fn personen(a: Option<&Address<'_>>) -> Vec<Persoon> {
    a.map(|a| {
        a.iter()
            .map(|p| Persoon {
                naam: p.name().unwrap_or_default().to_string(),
                adres: p.address().unwrap_or_default().to_string(),
            })
            .collect()
    })
    .unwrap_or_default()
}

pub fn kop(map: &str, uid: u32, gelezen: bool, beantwoord: bool, grootte: u32, header: &[u8]) -> Kop {
    let m = MessageParser::default().parse_headers(header);
    let (van_naam, van_adres, onderwerp, datum, bijlage) = match &m {
        Some(m) => {
            let van = personen(m.from()).into_iter().next();
            let ct = m.content_type();
            // Een "mixed"-bericht heeft vrijwel altijd een bijlage; precies weten kan pas bij openen.
            let bijlage = ct.map(|c| c.ctype() == "multipart" && c.subtype() == Some("mixed")).unwrap_or(false);
            (
                van.as_ref().map(|p| p.naam.clone()).unwrap_or_default(),
                van.map(|p| p.adres).unwrap_or_default(),
                m.subject().unwrap_or_default().to_string(),
                m.date().map(|d| d.to_timestamp()).unwrap_or(0),
                bijlage,
            )
        }
        None => Default::default(),
    };
    Kop { map: map.to_string(), uid, van_naam, van_adres, onderwerp, datum, gelezen, beantwoord, grootte, bijlage, gemarkeerd: false }
}

pub fn bericht(uid: u32, ruw: &[u8]) -> Result<Bericht> {
    let m = MessageParser::default().parse(ruw).ok_or_else(|| anyhow!("Kan dit bericht niet lezen"))?;
    let html = m.body_html(0).map(|h| schoon(&inline_afbeeldingen(&m, &h))).unwrap_or_default();
    let tekst = m.body_text(0).map(|t| t.to_string()).unwrap_or_default();
    let bijlagen = bijlagen(&m).into_iter().map(|(naam, soort, inhoud)| Bijlage { naam, grootte: inhoud.len(), soort }).collect();
    let message_id = m.message_id().map(|i| format!("<{i}>")).unwrap_or_default();
    let mut refs: Vec<String> = match m.references() {
        mail_parser::HeaderValue::Text(t) => vec![format!("<{t}>")],
        mail_parser::HeaderValue::TextList(l) => l.iter().map(|t| format!("<{t}>")).collect(),
        _ => vec![],
    };
    if !message_id.is_empty() {
        refs.push(message_id.clone());
    }
    Ok(Bericht {
        uid,
        onderwerp: m.subject().unwrap_or_default().to_string(),
        van: personen(m.from()),
        antwoord_aan: personen(m.reply_to()),
        message_id,
        referenties: refs.join(" "),
        aan: personen(m.to()),
        cc: personen(m.cc()),
        datum: m.date().map(|d| d.to_timestamp()).unwrap_or(0),
        html,
        tekst,
        bijlagen,
    })
}

/// Echte bijlagen (geen ingesloten afbeeldingen die al in de tekst staan): (naam, mime-type, inhoud).
fn bijlagen(m: &mail_parser::Message<'_>) -> Vec<(String, String, Vec<u8>)> {
    m.attachments()
        .filter(|p| !(p.content_id().is_some() && p.content_disposition().is_some_and(|d| d.ctype() == "inline")))
        .map(|p| {
            let soort = p
                .content_type()
                .map(|c| format!("{}/{}", c.ctype(), c.subtype().unwrap_or("octet-stream")))
                .unwrap_or_else(|| "application/octet-stream".into());
            let naam = p.attachment_name().map(str::to_string).unwrap_or_else(|| {
                if p.is_message() { "doorgestuurd-bericht.eml".into() } else { "bijlage".into() }
            });
            (naam, soort, p.contents().to_vec())
        })
        .collect()
}

/// Alle bijlagen uit een ruw bericht (om op te slaan, te openen of door te sturen).
pub fn bijlagen_uit(ruw: &[u8]) -> Result<Vec<(String, String, Vec<u8>)>> {
    let m = MessageParser::default().parse(ruw).ok_or_else(|| anyhow!("Kan dit bericht niet lezen"))?;
    Ok(bijlagen(&m))
}

/// Vervangt `cid:`-verwijzingen (afbeeldingen die in de mail zelf zitten, zoals logo's) door
/// data-URI's, zodat ze zichtbaar zijn zonder iets van internet te laden.
fn inline_afbeeldingen(m: &mail_parser::Message<'_>, html: &str) -> String {
    use base64::Engine;
    let mut uit = html.to_string();
    for p in m.parts.iter() {
        let (Some(cid), Some(ct)) = (p.content_id(), p.content_type()) else { continue };
        if ct.ctype() != "image" || p.contents().len() > 5_000_000 {
            continue;
        }
        let data = format!(
            "data:image/{};base64,{}",
            ct.subtype().unwrap_or("png"),
            base64::engine::general_purpose::STANDARD.encode(p.contents())
        );
        uit = uit.replace(&format!("cid:{cid}"), &data);
    }
    uit
}

/// Haalt scripts, formulieren en andere actieve inhoud weg, maar laat de opmaak van
/// nieuwsbrieven en facturen (inline stijlen, tabellen, afbeeldingen) staan.
fn schoon(html: &str) -> String {
    let mut b = ammonia::Builder::default();
    let extra_tags: HashSet<&str> = ["style", "center", "font"].into();
    let algemeen: HashSet<&str> =
        ["style", "class", "align", "valign", "width", "height", "bgcolor", "color", "border", "cellpadding", "cellspacing", "dir"].into();
    b.add_tags(extra_tags)
        .rm_clean_content_tags(["style"])
        .add_generic_attributes(algemeen)
        .add_tag_attributes("font", ["face", "size"])
        .link_rel(Some("noopener noreferrer"))
        .add_url_schemes(["data", "cid"])
        .url_relative(ammonia::UrlRelative::Deny);
    b.clean(html).to_string()
}

/// Kalenderjaar van een Unix-tijd (voor het jaararchief); None als de datum ontbreekt.
pub fn jaar_van(sec: i64) -> Option<i32> {
    (sec > 0).then(|| mail_parser::DateTime::from_timestamp(sec).year as i32)
}
