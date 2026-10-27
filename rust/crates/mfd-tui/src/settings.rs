//! Le impostazioni persistenti del TUI (F2): tema, la lente (ordinamento e
//! filtro, la stessa per lista e dettaglio), la barra e la lingua. Tutto il resto —
//! navigazione, warning — e' sessione, non identita'. F2 fotografa lo stato
//! corrente e lo
//! scrive nel profilo utente: %APPDATA%\mfd-tui\settings.txt su Windows,
//! ~/.config/mfd-tui/settings.txt altrove. Al prossimo avvio, se il file
//! c'e', le impostazioni si riprendono; F2 di nuovo le aggiorna; eliminare
//! il file riporta i default di fabbrica. Il formato e' ASCII puro, una
//! chiave=valore per riga (# per i commenti) e i codici sono quelli della
//! CLI (-o "T.D", le specifiche di -f): anche su disco si parla il lessico
//! del prodotto. Il file e' pensato per essere scritto dal TUI, non editato
//! a mano — ma from_text e' comunque tollerante: le righe che non capisce
//! diventano default, mai un errore.

use crate::app::App;
use crate::theme::ThemeId;
use mfd_core::{Filter, Order};
use std::path::{Path, PathBuf};

/// Le impostazioni che sopravvivono alla sessione. I default sono i default
/// di fabbrica del TUI (gli stessi di `App::base`): la Console, taglia
/// decrescente, nessun filtro, barra visibile.
#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    pub theme: ThemeId,
    pub order: Order,
    /// la specifica del filtro (glob o REG(...)); None: nessuno
    pub filter: Option<String>,
    pub show_bar: bool,
    /// il codice della lingua scelta (l / F12 nel TUI); None: fabbrica,
    /// l'inglese incorporato
    pub lang: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: ThemeId::DEFAULT,
            order: Order::TD,
            filter: None,
            show_bar: true,
            lang: None,
        }
    }
}

impl Settings {
    /// La fotografia dello stato corrente: quello che F2 porta su disco.
    pub fn of(app: &App) -> Settings {
        Settings {
            theme: app.theme_id,
            order: app.view.order,
            filter: app
                .view
                .filter
                .as_ref()
                .map(|f| f.display().to_string()),
            show_bar: app.view.show_bar,
            lang: if app.cat.active.code == "en" {
                None // il fabbrica non si salva: e' il default di partenza
            } else {
                Some(app.cat.active.code.clone())
            },
        }
    }

    /// Applica le impostazioni all'app: tema, lente, barra. Il filtro e'
    /// ricostruito con le regole di -f; una specifica non valida (file
    /// ritoccato a mano) si ignora, non e' un errore.
    pub fn apply(&self, app: &mut App) {
        app.theme_id = self.theme;
        app.theme = crate::theme::Theme::for_id(self.theme);
        app.view.order = self.order;
        app.view.filter = self
            .filter
            .as_deref()
            .and_then(|s| Filter::build(s).ok());
        // la barra e' un marchio: o c'e' o non c'e', in entrambi i pannelli
        app.view.show_bar = self.show_bar;
        // la lingua salvata si adotta in silenzio (e' l'avvio, non una
        // scelta: il flash non c'entra). Un codice che non esiste piu'
        // (file cancellato dal profilo) si ignora: si resta dov'eravamo
        if let Some(code) = &self.lang {
            app.adopt_lang(code);
        }
        app.refresh();
    }

    /// Il testo del file: chiave=valore, una riga ciascuna, commenti con #.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("# mfd-tui: impostazioni salvate con F2 (F2 le aggiorna;\n");
        out.push_str("# si elimina il file per tornare ai default di fabbrica)\n");
        out.push_str(&format!("tema={}\n", self.theme.code()));
        out.push_str(&format!("ord={}\n", self.order.code()));
        out.push_str(&format!("fil={}\n", self.filter.as_deref().unwrap_or("")));
        out.push_str(&format!(
            "barra={}\n",
            if self.show_bar { "on" } else { "off" }
        ));
        if let Some(l) = &self.lang {
            out.push_str(&format!("lingua={}\n", l));
        }
        out
    }

    /// Rilegge il testo: tollerante. Commenti e righe vuote si saltano, le
    /// chiavi sconosciute pure; i valori che non passano la grammatica del
    /// prodotto (codici di -o, temi, specifiche di -f) tornano al default —
    /// un file ritoccato male non fa mai crashare l'avvio.
    pub fn from_text(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, val)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "tema" => {
                    if let Some(t) = ThemeId::parse(val) {
                        s.theme = t;
                    }
                }
                "ord" => {
                    if let Some(o) = Order::parse(val) {
                        s.order = o;
                    }
                }
                "fil" => s.filter = filtro_valido(val),
                "barra" => match val.trim() {
                    "on" => s.show_bar = true,
                    "off" => s.show_bar = false,
                    _ => {}
                },
                // il codice lingua lo valida chi lo usa (il TUI lo cerca
                // tra le lingue trovate): qui si accetta qualunque riga
                "lingua" => {
                    let v = val.trim();
                    if !v.is_empty() {
                        s.lang = Some(v.to_string());
                    }
                }
                _ => {}
            }
        }
        s
    }

    /// Dove vivono le impostazioni: nel profilo utente —
    /// %APPDATA%\mfd-tui\settings.txt su Windows,
    /// ~/.config/mfd-tui/settings.txt altrove (la scelta dei tool da
    /// terminale). None solo in ambienti senza profilazione: F2 lo dice.
    pub fn default_path() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            std::env::var_os("APPDATA")
                .map(|d| PathBuf::from(d).join("mfd-tui").join("settings.txt"))
        }
        #[cfg(not(windows))]
        {
            std::env::var_os("HOME")
                .map(|d| PathBuf::from(d).join(".config").join("mfd-tui").join("settings.txt"))
        }
    }

    /// Scrive il file (creando la cartella del profilo se manca).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_text())
    }

    /// Legge il file: None se manca o non si legge (chi non ha mai salvato
    /// non nota niente: si parte coi default).
    pub fn load(path: &Path) -> Option<Settings> {
        let text = std::fs::read_to_string(path).ok()?;
        Some(Settings::from_text(&text))
    }
}

/// Una specifica di filtro valida (regole di -f) o niente.
fn filtro_valido(val: &str) -> Option<String> {
    let spec = val.trim();
    if spec.is_empty() {
        return None;
    }
    Filter::build(spec).ok().map(|_| spec.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_dal_testo_alle_impostazioni() {
        let s = Settings {
            theme: ThemeId::Debian,
            order: Order::AC,
            filter: Some("*.dll".to_string()),
            show_bar: false,
            lang: Some("it".to_string()),
        };
        assert_eq!(Settings::from_text(&s.to_text()), s);
    }

    #[test]
    fn il_testo_tollera_commenti_righe_vuote_e_garbage() {
        // chiave sbagliata (il casing conta), valore fuori grammatica,
        // riga senza '=', chiacchiere: tutto si ignora; il resto passa
        let s = Settings::from_text(
            "# un commento\n\nORD=AC\nord=xx\nche cosa?\ntema=minimal\nbarra=off\n",
        );
        assert_eq!(s.theme, ThemeId::Minimal); // la riga buona passa
        assert_eq!(s.order, Order::TD); // ne' chiave ne' valore validi
        assert!(s.filter.is_none());
        assert!(!s.show_bar);
        assert!(s.lang.is_none()); // nemmeno la lingua si inventa
    }

    #[test]
    fn la_lingua_si_salva_e_si_rilegge() {
        let s = Settings {
            lang: Some("pt-BR".to_string()),
            ..Settings::default()
        };
        assert_eq!(Settings::from_text(&s.to_text()), s);
        // vuota: nessuna lingua (si torna al fabbrica, non a un codice "")
        assert!(Settings::from_text("lingua=\n").lang.is_none());
        assert!(Settings::from_text("lingua=   \n").lang.is_none());
    }

    #[test]
    fn una_specifica_di_filtro_non_valida_non_si_carica() {
        let s = Settings::from_text("fil=REG([)\n");
        assert!(s.filter.is_none());
    }

    #[test]
    fn save_e_load_attraverso_il_file() {
        let dir = std::env::temp_dir().join(format!("mfd-settings-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.txt");
        let s = Settings {
            theme: ThemeId::Minimal,
            order: Order::CC,
            ..Settings::default()
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
        assert!(Settings::load(&dir.join("manca.txt")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
