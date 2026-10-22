//! De accounts op deze pc. Adressen en serverinstellingen staan in `accounts.json` in de
//! app-map; wachtwoorden staan in de wachtwoordkluis van het besturingssysteem
//! (Windows Referentiebeheer / Linux Secret Service), nooit in een bestand.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const DIENST: &str = "pro-art-mail";

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Account {
    pub email: String,
    /// Naam die ontvangers zien, bv. "Pro Art Nijmegen". Leeg = alleen het adres.
    #[serde(default)]
    pub naam: String,
    pub imap_host: String,
    pub imap_poort: u16,
    pub smtp_host: String,
    pub smtp_poort: u16,
    /// Map voor verstuurde mail. Leeg = de map die de server als "Verzonden" aanmerkt.
    #[serde(default)]
    pub verzonden_map: String,
}

impl Account {
    /// Standaardinstellingen bij een adres. Voor pro-art.nl de mailserver van de winkel
    /// (zie CLAUDE.md); voor andere domeinen de gebruikelijke `mail.<domein>`.
    pub fn standaard(email: &str) -> Self {
        let email = email.trim().to_lowercase();
        let domein = email.rsplit('@').next().unwrap_or_default().to_string();
        let (host, verzonden) = match domein.as_str() {
            "pro-art.nl" => ("mail.pro-art.nl".to_string(), "Sent Messages".to_string()),
            "gmail.com" | "googlemail.com" => ("imap.gmail.com".to_string(), String::new()),
            _ => (format!("mail.{domein}"), String::new()),
        };
        let smtp = if host == "imap.gmail.com" { "smtp.gmail.com".to_string() } else { host.clone() };
        Account { email, naam: String::new(), imap_host: host, imap_poort: 993, smtp_host: smtp, smtp_poort: 465, verzonden_map: verzonden }
    }
}

fn bestand(app: &AppHandle) -> Result<PathBuf> {
    let map = app.path().app_config_dir()?;
    fs::create_dir_all(&map)?;
    Ok(map.join("accounts.json"))
}

pub fn alle(app: &AppHandle) -> Result<Vec<Account>> {
    let pad = bestand(app)?;
    if pad.exists() {
        return Ok(serde_json::from_str(&fs::read_to_string(pad)?)?);
    }
    // Overstap van versie 1 (één account in account.json).
    let oud = pad.with_file_name("account.json");
    if oud.exists() {
        let mut acc: Account = serde_json::from_str(&fs::read_to_string(&oud)?)?;
        if acc.verzonden_map.is_empty() {
            acc.verzonden_map = Account::standaard(&acc.email).verzonden_map;
        }
        bewaar_lijst(app, std::slice::from_ref(&acc))?;
        fs::remove_file(oud)?;
        return Ok(vec![acc]);
    }
    Ok(vec![])
}

pub fn zoek(app: &AppHandle, email: &str) -> Result<Account> {
    alle(app)?
        .into_iter()
        .find(|a| a.email == email)
        .with_context(|| format!("Account {email} staat niet (meer) op deze pc"))
}

fn bewaar_lijst(app: &AppHandle, lijst: &[Account]) -> Result<()> {
    fs::write(bestand(app)?, serde_json::to_string_pretty(lijst)?)?;
    Ok(())
}

/// Voegt een account toe of werkt het bij (zelfde adres). Wachtwoord leeg = ongewijzigd laten.
pub fn bewaar(app: &AppHandle, acc: &Account, wachtwoord: &str) -> Result<()> {
    if !wachtwoord.is_empty() {
        keyring::Entry::new(DIENST, &acc.email)?
            .set_password(wachtwoord)
            .context("Kan het wachtwoord niet veilig opslaan")?;
    }
    let mut lijst = alle(app)?;
    match lijst.iter_mut().find(|a| a.email == acc.email) {
        Some(a) => *a = acc.clone(),
        None => lijst.push(acc.clone()),
    }
    bewaar_lijst(app, &lijst)
}

pub fn wachtwoord(email: &str) -> Result<String> {
    keyring::Entry::new(DIENST, email)?
        .get_password()
        .context("Wachtwoord niet gevonden; voeg het account opnieuw toe")
}

pub fn verwijder(app: &AppHandle, email: &str) -> Result<()> {
    let _ = keyring::Entry::new(DIENST, email).and_then(|e| e.delete_credential());
    let lijst: Vec<Account> = alle(app)?.into_iter().filter(|a| a.email != email).collect();
    bewaar_lijst(app, &lijst)
}

/// Ondertekeningen (gedeeld door alle accounts op deze pc).
#[derive(Serialize, Deserialize, Clone)]
pub struct Ondertekening {
    pub naam: String,
    pub tekst: String,
}

pub fn ondertekeningen(app: &AppHandle) -> Result<Vec<Ondertekening>> {
    let pad = bestand(app)?.with_file_name("ondertekeningen.json");
    if !pad.exists() {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str(&fs::read_to_string(pad)?)?)
}

pub fn bewaar_ondertekeningen(app: &AppHandle, lijst: &[Ondertekening]) -> Result<()> {
    fs::write(bestand(app)?.with_file_name("ondertekeningen.json"), serde_json::to_string_pretty(lijst)?)?;
    Ok(())
}
