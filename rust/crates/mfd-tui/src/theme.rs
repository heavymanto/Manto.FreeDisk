//! I quattro temi del TUI (tasto t / F3): **Notturno** (blu Norton/TC,
//! selezione gialla), **Debian** (sfondo blu dell'installer, finestre grigie,
//! selezione rossa), **Console** (il default: bianco su nero, accenti gialli,
//! selettore verde) e **CLI** (nero: i colori del terminale). 16 colori ANSI,
//! niente truecolor. La scelta e' di sessione; con F2 si puo' farla
//! sopravvivere nel profilo utente (settings.rs — docs/DESIGN-TUI.md, sezione 6).
//! Ogni tema porta anche i colori della propria ombra: il velo che scurisce
//! cio' che sta dietro le finestre — l'effetto popup, non una lastra piena.

use ratatui::style::{Color, Modifier, Style};

/// L'identita' di un tema: quello che si sceglie nel menu (t / F3) e che
/// l'app ricorda. L'ordine dei varianti e' l'ordine del menu.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeId {
    Notturno,
    Debian,
    Console,
    Minimal,
}

impl ThemeId {
    /// I quattro temi, nell'ordine del menu.
    pub const ALL: [ThemeId; 4] = [ThemeId::Notturno, ThemeId::Debian, ThemeId::Console, ThemeId::Minimal];

    /// Il tema di partenza, e quello di fabbrica delle impostazioni: la Console.
    pub const DEFAULT: ThemeId = ThemeId::Console;

    /// La chiave del vocabolario per il nome nel menu (i18n: il menu t / F3
    /// e il flash traducono; il nome non vive piu' qui). I nomi restano
    /// nomi propri — Notturno, Debian — anche nelle altre lingue.
    pub fn key(self) -> &'static str {
        match self {
            ThemeId::Notturno => "tema.notturno",
            ThemeId::Debian => "tema.debian",
            ThemeId::Console => "tema.console",
            ThemeId::Minimal => "tema.minimal",
        }
    }

    /// La chiave del vocabolario per la riga descrittiva nel menu.
    pub fn blurb_key(self) -> &'static str {
        match self {
            ThemeId::Notturno => "tema.notturno.b",
            ThemeId::Debian => "tema.debian.b",
            ThemeId::Console => "tema.console.b",
            ThemeId::Minimal => "tema.minimal.b",
        }
    }

    /// Il codice del tema nel file delle impostazioni di F2 (ASCII puro).
    pub fn code(self) -> &'static str {
        match self {
            ThemeId::Minimal => "minimal",
            ThemeId::Notturno => "notturno",
            ThemeId::Debian => "debian",
            ThemeId::Console => "console",
        }
    }

    /// Il tema dal codice del file delle impostazioni (tollerante al casing
    /// e agli spazi; None se non e' uno dei quattro).
    pub fn parse(s: &str) -> Option<ThemeId> {
        match s.trim().to_ascii_lowercase().as_str() {
            "minimal" => Some(ThemeId::Minimal),
            "notturno" => Some(ThemeId::Notturno),
            "debian" => Some(ThemeId::Debian),
            "console" => Some(ThemeId::Console),
            _ => None,
        }
    }
}

/// Un tema completo: uno stile per ruolo. I campi si chiamano come le vecchie
/// funzioni del modulo (screen, fg, title, ...) perche' i ruoli sono quelli.
#[derive(Clone, Copy)]
pub struct Theme {
    /// Lo sfondo di tutto il TUI (e il testo di default).
    pub screen: Style,
    /// Lo sfondo delle finestre di dialogo (F1, F6, F7, w, t): puo' differire
    /// da quello del pannello, come nei dialoghi dell'installer Debian.
    pub dialog: Style,
    /// Testo corrente.
    pub fg: Style,
    /// Titoli dei pannelli e delle finestre.
    pub title: Style,
    /// Intestazioni di colonna e righe di stato dichiarate.
    pub header: Style,
    /// Suggerimenti e note a margine.
    pub hint: Style,
    /// I numeri dei tasti funzione nella barra in fondo.
    pub key: Style,
    /// Messaggi transitori di stato.
    pub status: Style,
    /// La riga selezionata.
    pub selected: Style,
    /// Bordo del pannello attivo (bersaglio dei comandi, TAB).
    pub border_active: Style,
    /// Bordo del pannello non attivo.
    pub border_idle: Style,
    /// La parte piena della barra di avanzamento.
    pub bar: Style,
    /// Nome di una cartella nell'anteprima.
    pub dir: Style,
    /// Nome di un file nell'anteprima.
    pub file: Style,
    /// Il primo piano del velo dell'ombra: le finestre non cancellano cio'
    /// che sta dietro, lo scoloriscono — l'effetto popup delle console vere.
    pub shadow_fg: Color,
    /// Il fondo del velo: None solo per la CLI, che non impone colori e
    /// lascia al terminale anche il fondo dell'ombra.
    pub shadow_bg: Option<Color>,
}

impl Theme {
    pub fn for_id(id: ThemeId) -> Theme {
        match id {
            ThemeId::Minimal => Theme::minimal(),
            ThemeId::Notturno => Theme::notturno(),
            ThemeId::Debian => Theme::debian(),
            ThemeId::Console => Theme::console(),
        }
    }

    /// La CLI: nessun colore imposto, i colori del terminale (su un terminale
    /// nero e' il tema nero). Selezione a video inverso. L'ombra e' un velo
    /// che tocca solo il primo piano: il fondo resta del terminale.
    fn minimal() -> Theme {
        Theme {
            screen: Style::new(),
            dialog: Style::new(),
            fg: Style::new(),
            title: Style::new().add_modifier(Modifier::BOLD),
            header: Style::new().add_modifier(Modifier::BOLD),
            hint: Style::new().add_modifier(Modifier::DIM),
            key: Style::new().add_modifier(Modifier::BOLD),
            status: Style::new().add_modifier(Modifier::BOLD),
            selected: Style::new().add_modifier(Modifier::REVERSED),
            border_active: Style::new().add_modifier(Modifier::BOLD),
            border_idle: Style::new().add_modifier(Modifier::DIM),
            bar: Style::new(),
            dir: Style::new(),
            file: Style::new().add_modifier(Modifier::DIM),
            shadow_fg: Color::DarkGray,
            shadow_bg: None,
        }
    }

    /// Notturno: blu Norton/TC — sfondo blu, testo bianco/ciano, selezione
    /// gialla a video inverso, ombra nera a velo (il classico).
    fn notturno() -> Theme {
        Theme {
            screen: Style::new().bg(Color::Blue).fg(Color::White),
            dialog: Style::new().bg(Color::Blue).fg(Color::White),
            fg: Style::new().fg(Color::White),
            title: Style::new().fg(Color::Cyan).bold(),
            header: Style::new().fg(Color::Cyan),
            hint: Style::new().fg(Color::Cyan),
            key: Style::new().fg(Color::Yellow).bold(),
            status: Style::new().fg(Color::Yellow),
            selected: Style::new().fg(Color::Black).bg(Color::Yellow),
            border_active: Style::new().fg(Color::Yellow),
            border_idle: Style::new().fg(Color::White),
            bar: Style::new().fg(Color::Cyan),
            dir: Style::new().fg(Color::White),
            file: Style::new().fg(Color::Cyan),
            shadow_fg: Color::DarkGray,
            shadow_bg: Some(Color::Black),
        }
    }

    /// Lo sfondo blu dell'installer Debian (come il Norton, ma con i colori
    /// dei dialoghi debconf): finestre di dialogo grigio chiaro con testo
    /// nero, selezione bianco su rosso, accenti rossi. Ombra nera a velo.
    fn debian() -> Theme {
        Theme {
            screen: Style::new().bg(Color::Blue).fg(Color::White),
            dialog: Style::new().bg(Color::Gray).fg(Color::Black),
            fg: Style::new().fg(Color::Black),
            title: Style::new().fg(Color::Red).bold(),
            header: Style::new().fg(Color::Red),
            hint: Style::new().fg(Color::DarkGray),
            key: Style::new().fg(Color::Red).bold(),
            status: Style::new().fg(Color::Red),
            selected: Style::new().fg(Color::White).bg(Color::Red),
            border_active: Style::new().fg(Color::Red),
            border_idle: Style::new().fg(Color::DarkGray),
            bar: Style::new().fg(Color::Red),
            dir: Style::new().fg(Color::White),
            file: Style::new().fg(Color::Cyan),
            shadow_fg: Color::DarkGray,
            shadow_bg: Some(Color::Black),
        }
    }

    /// La console, il tema di fabbrica: nero, testo bianco, accenti
    /// gialli (lo stesso 93 dei bordi della CLI — la famiglia si riconosce),
    /// grigio per il secondario — e il selettore verde chiaro (LightGreen):
    /// il cursore delle console vere. L'ombra e' il velo: il testo dietro
    /// scolorisce in grigio scuro — l'effetto popup, non la lastra piena.
    fn console() -> Theme {
        Theme {
            screen: Style::new().bg(Color::Black).fg(Color::White),
            dialog: Style::new().bg(Color::Black).fg(Color::White),
            fg: Style::new().fg(Color::White),
            title: Style::new().fg(Color::Yellow).bold(),
            header: Style::new().fg(Color::Yellow),
            hint: Style::new().fg(Color::Gray),
            key: Style::new().fg(Color::Yellow).bold(),
            status: Style::new().fg(Color::Yellow),
            selected: Style::new().fg(Color::Black).bg(Color::LightGreen),
            border_active: Style::new().fg(Color::Yellow),
            border_idle: Style::new().fg(Color::Gray),
            bar: Style::new().fg(Color::Yellow),
            dir: Style::new().fg(Color::White),
            file: Style::new().fg(Color::Gray),
            shadow_fg: Color::DarkGray,
            shadow_bg: Some(Color::Black),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i_quattro_temi_si_costruiscono() {
        for id in ThemeId::ALL {
            let _ = Theme::for_id(id);
        }
    }

    #[test]
    fn il_default_e_la_console() {
        assert_eq!(ThemeId::DEFAULT, ThemeId::Console);
    }

    #[test]
    fn i_quattro_temi_sono_diversi() {
        let m = Theme::for_id(ThemeId::Minimal);
        let n = Theme::for_id(ThemeId::Notturno);
        let d = Theme::for_id(ThemeId::Debian);
        let c = Theme::for_id(ThemeId::Console);
        assert_eq!(n.screen, d.screen); // stesso blu: le differenze sono altrove
        assert_ne!(n.selected, d.selected);
        assert_ne!(d.selected, c.selected);
        assert_ne!(m.screen, n.screen);
        assert_ne!(d.dialog, n.dialog);
    }
}
