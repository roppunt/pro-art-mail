//! Linux, AppImage: een AppImage zet zelf niets in het menu of op het bureaublad. Na het sluiten
//! is het programma dan "weg". Daarom regelt de app dat bij elke start zelf:
//! - de AppImage krijgt een vaste plek (`~/Applications/pro-art-mail.AppImage`), zodat hij niet
//!   in Downloads blijft staan en de updater altijd hetzelfde bestand bijwerkt;
//! - een menu-item (`~/.local/share/applications`) en een icoon op het bureaublad, beide naar die vaste plek.
//!
//! Alles mag mislukken zonder dat de app daar last van heeft; fouten gaan alleen naar de log.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const NAAM: &str = "pro-art-mail";
const PICTOGRAM: &[u8] = include_bytes!("../icons/128x128@2x.png");

pub fn regel() {
    // Alleen als AppImage; de .deb zet zelf al een menu-item neer.
    let Some(appimage) = std::env::var_os("APPIMAGE").map(PathBuf::from) else { return };
    if let Err(e) = probeer(&appimage) {
        eprintln!("snelkoppeling: {e:#}");
    }
}

fn probeer(appimage: &Path) -> anyhow::Result<()> {
    let thuis = PathBuf::from(std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("geen HOME"))?);
    let doel = vaste_plek(&thuis, appimage)?;

    let pictogram = thuis.join(".local/share/icons/hicolor/256x256/apps").join(format!("{NAAM}.png"));
    schrijf_als_anders(&pictogram, PICTOGRAM)?;

    let regels = format!(
        "[Desktop Entry]\nType=Application\nName=Pro-Art Mail\nComment=Mail voor de winkel\n\
         Exec=\"{}\" %U\nIcon={NAAM}\nTerminal=false\nCategories=Network;Email;Office;\n\
         StartupWMClass={NAAM}\n",
        doel.display()
    );
    // Bestandsnaam = identifier: daaraan koppelt GNOME onder Wayland het venster aan dit menu-item.
    let menu = thuis.join(".local/share/applications/nl.pro-art.mail.desktop");
    schrijf_als_anders(&menu, regels.as_bytes())?;
    let _ = std::process::Command::new("update-desktop-database").arg(menu.parent().unwrap()).status();

    // Bureaublad: alleen de eerste keer neerzetten. Heeft iemand het icoon weggehaald, dan blijft het weg.
    let vlag = thuis.join(".local/share").join(NAAM).join("bureaublad-gedaan");
    if !vlag.exists() {
        if let Some(bureaublad) = bureaublad(&thuis) {
            let icoon = bureaublad.join(format!("{NAAM}.desktop"));
            schrijf_als_anders(&icoon, regels.as_bytes())?;
            std::fs::set_permissions(&icoon, std::fs::Permissions::from_mode(0o755))?;
            // GNOME/Zorin start een bureaublad-icoon pas zonder vraag als het "vertrouwd" is.
            let _ = std::process::Command::new("gio")
                .args(["set", &icoon.to_string_lossy(), "metadata::trusted", "true"])
                .status();
        }
        schrijf_als_anders(&vlag, b"")?;
    } else if let Some(bureaublad) = bureaublad(&thuis) {
        // Wel bijwerken als het er nog staat (bv. na verhuizen van de AppImage).
        let icoon = bureaublad.join(format!("{NAAM}.desktop"));
        if icoon.exists() {
            schrijf_als_anders(&icoon, regels.as_bytes())?;
        }
    }
    Ok(())
}

/// Kopieert de AppImage naar `~/Applications` als hij daar nog niet staat, en geeft het pad dat
/// de snelkoppelingen moeten gebruiken.
fn vaste_plek(thuis: &Path, appimage: &Path) -> anyhow::Result<PathBuf> {
    let doel = thuis.join("Applications").join(format!("{NAAM}.AppImage"));
    if appimage == doel {
        return Ok(doel);
    }
    std::fs::create_dir_all(doel.parent().unwrap())?;
    // Eerst naar een tijdelijke naam, dan hernoemen: zo staat er nooit een half bestand.
    let tijdelijk = doel.with_extension("AppImage.nieuw");
    std::fs::copy(appimage, &tijdelijk)?;
    std::fs::set_permissions(&tijdelijk, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&tijdelijk, &doel)?;
    Ok(doel)
}

fn bureaublad(thuis: &Path) -> Option<PathBuf> {
    // `xdg-user-dir` kent de taal (Nederlands: ~/Bureaublad).
    let uit = std::process::Command::new("xdg-user-dir").arg("DESKTOP").output().ok()?;
    let pad = PathBuf::from(String::from_utf8_lossy(&uit.stdout).trim());
    if pad.is_dir() && pad != thuis {
        return Some(pad);
    }
    ["Bureaublad", "Desktop"].iter().map(|n| thuis.join(n)).find(|p| p.is_dir())
}

fn schrijf_als_anders(pad: &Path, inhoud: &[u8]) -> anyhow::Result<()> {
    if std::fs::read(pad).ok().as_deref() == Some(inhoud) {
        return Ok(());
    }
    std::fs::create_dir_all(pad.parent().unwrap())?;
    std::fs::write(pad, inhoud)?;
    Ok(())
}
