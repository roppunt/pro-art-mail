//! Het account op deze pc. Adres en serverinstellingen staan in `account.json` in de
//! app-map; het wachtwoord staat in de wachtwoordkluis van het besturingssysteem
//! (Windows Referentiebeheer / Linux Secret Service), nooit in een bestand.

use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const DIENST: &str = "pro-art-mail";

#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
    pub email: String,
    pub imap_host: String,
    pub imap_poort: u16,
    pub smtp_host: String,
    pub smtp_poort: u16,
}

impl Account {
    /// Vaste instellingen van de mailserver van de winkel (WBV Online, zie CLAUDE.md).
    pub fn voor(email: &str) -> Self {
        Account {
            email: email.to_lowercase(),
            imap_host: "mail.pro-art.nl".into(),
            imap_poort: 993,
            smtp_host: "mail.pro-art.nl".into(),
            smtp_poort: 465,
        }
    }
}

fn bestand(app: &AppHandle) -> Result<std::path::PathBuf> {
    let map = app.path().app_config_dir()?;
    fs::create_dir_all(&map)?;
    Ok(map.join("account.json"))
}

pub fn lees(app: &AppHandle) -> Result<Option<Account>> {
    let pad = bestand(app)?;
    if !pad.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&fs::read_to_string(pad)?)?))
}

pub fn bewaar(app: &AppHandle, acc: &Account, wachtwoord: &str) -> Result<()> {
    keyring::Entry::new(DIENST, &acc.email)?
        .set_password(wachtwoord)
        .context("Kan het wachtwoord niet veilig opslaan")?;
    fs::write(bestand(app)?, serde_json::to_string_pretty(acc)?)?;
    Ok(())
}

pub fn wachtwoord(email: &str) -> Result<String> {
    keyring::Entry::new(DIENST, email)?
        .get_password()
        .context("Wachtwoord niet gevonden; log opnieuw in")
}

pub fn vergeet(app: &AppHandle) -> Result<()> {
    if let Some(acc) = lees(app)? {
        let _ = keyring::Entry::new(DIENST, &acc.email).and_then(|e| e.delete_credential());
    }
    let pad = bestand(app)?;
    if pad.exists() {
        fs::remove_file(pad)?;
    }
    Ok(())
}
