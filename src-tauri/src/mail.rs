//! Mail ontleden (kopregels, tekst, bijlagen) en HTML veilig maken voor weergave.

use std::collections::HashSet;

use anyhow::{Result, anyhow};
use mail_parser::{Address, MessageParser, MimeHeaders};
use serde::Serialize;

#[derive(Serialize)]
pub struct Kop {
    pub uid: u32,
    pub volgnummer: u32,
    pub van_naam: String,
    pub van_adres: String,
    pub onderwerp: String,
    /// Unix-tijd in seconden; 0 als de datum ontbreekt.
    pub datum: i64,
    pub gelezen: bool,
    pub beantwoord: bool,
    pub grootte: u32,
    pub bijlage: bool,
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

pub fn kop(uid: u32, volgnummer: u32, gelezen: bool, beantwoord: bool, grootte: u32, header: &[u8]) -> Kop {
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
    Kop { uid, volgnummer, van_naam, van_adres, onderwerp, datum, gelezen, beantwoord, grootte, bijlage }
}

pub fn bericht(uid: u32, ruw: &[u8]) -> Result<Bericht> {
    let m = MessageParser::default().parse(ruw).ok_or_else(|| anyhow!("Kan dit bericht niet lezen"))?;
    let html = m.body_html(0).map(|h| schoon(&h)).unwrap_or_default();
    let tekst = m.body_text(0).map(|t| t.to_string()).unwrap_or_default();
    let bijlagen = m
        .attachments()
        .filter(|p| !p.is_message() || p.attachment_name().is_some())
        .map(|p| Bijlage {
            naam: p.attachment_name().unwrap_or("bijlage").to_string(),
            grootte: p.contents().len(),
            soort: p
                .content_type()
                .map(|c| format!("{}/{}", c.ctype(), c.subtype().unwrap_or("")))
                .unwrap_or_default(),
        })
        .collect();
    Ok(Bericht {
        uid,
        onderwerp: m.subject().unwrap_or_default().to_string(),
        van: personen(m.from()),
        aan: personen(m.to()),
        cc: personen(m.cc()),
        datum: m.date().map(|d| d.to_timestamp()).unwrap_or(0),
        html,
        tekst,
        bijlagen,
    })
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
        .url_relative(ammonia::UrlRelative::Deny);
    b.clean(html).to_string()
}
