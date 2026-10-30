// mfd-tui — l'interfaccia interattiva di Manto.FreeDisk (fase 2).
// Il design approvato vive in docs/DESIGN-TUI.md: pannello doppio (lista a
// sinistra + anteprima a destra, lente unica: s e f valgono per entrambi),
// barre #/. come la CLI, solo cartelle nella lista, drill-down con INVIO,
// niente eliminazione (mfd misura, non muta), quattro temi (Notturno, Debian,
// Console — il default — e CLI, nera: tasto t).
// Il mouse si veste da tasto: la rotella scorre come le frecce (un detent =
// una riga; inclinata a sinistra/destra e' <- e ->), il click seleziona la
// riga, il doppio click entra come INVIO, il destro apre il ramo (x). Il
// drag resta al terminale (Shift seleziona il testo mentre il capture e'
// acceso); i bottoni laterali (Indietro/Avanti) non attraversano il
// terminale: crossterm non li vede.
// F2 salva le impostazioni — tema, lente (ordinamento e filtro), barra,
// lingua — nel profilo utente e il prossimo avvio le riprende (settings.rs);
// F5 riscansiona; p / F11 e' la stampa: la fotografia JSON della
// cartella corrente come la si vede — i rami aperti, la lente (il
// contratto di --json, mfd-core) — su file nella cartella di lancio;
// l / F12 e' la lingua: il menu di quelle trovate (inglese incorporato
// piu' i file del profilo, mfd-core::lang — docs/DESIGN-I18N.md). La
// catena: --lang batte MFD_LANG che batte la riga lingua= di F2.
// Il motore e' mfd-core: la scansione gira sui thread del walker e l'event
// loop resta reattivo (contatori via canale, ESC annulla senza risultati
// parziali). Uso: mfd-tui <path> - senza argomenti: cartella corrente (naked = .).

mod app;
mod settings;
mod theme;
mod ui;

use app::App;
use crate::settings::Settings;
use mfd_core::lang::Catalog;
use std::time::Duration;

/// La lingua con cui il TUI parte — e con cui parlano -h e gli errori
/// pre-TUI: --lang della riga di comando, poi MFD_LANG, poi la riga
/// lingua= del profilo (la scrive F2), poi l'inglese incorporato. Ritorna
/// anche il codice chiesto e mancato, per l'avviso: non e' un errore, si
/// parte dove dice il resto della catena.
fn resolve_lang(cli: Option<String>) -> (Catalog, Option<String>) {
    let env = std::env::var("MFD_LANG")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let file = Settings::default_path()
        .and_then(|p| Settings::load(&p))
        .and_then(|s| s.lang);
    let asked = cli.or(env).or(file);
    let (langs, _) = mfd_core::lang::registry();
    match asked {
        Some(code) => match langs.iter().find(|l| l.code == code) {
            Some(l) => (Catalog::with(l.clone()), None),
            None => (Catalog::english(), Some(code)),
        },
        None => (Catalog::english(), None),
    }
}

/// L'aiuto parla la lingua della stessa catena del TUI (docs/DESIGN-I18N):
/// il vocabolario arriva da lang/, il blocco intero e' una chiave sola.
fn print_help(cat: &Catalog) {
    println!("{}", cat.tf("tui.tagline", &[env!("CARGO_PKG_VERSION")]));
    print!("{}", cat.t("tui.help"));
}

/// Un path della riga di comando in forma assoluta e pulita: i relativi si
/// risolvono, i . e .. si sciolgono — chi li ha digitati sa dov'e', chi
/// legge la fascia merita il percorso intero, c:\lll\kkkk\kkk come lo
/// scriverebbe chiunque. canonicalize su Windows mette il prefisso
/// verbatim \\?\ : si toglie, il percorso deve restare leggibile
fn assoluta(p: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(p);
    let pulita = std::fs::canonicalize(&path).unwrap_or(path);
    if let Some(s) = pulita.to_str() {
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return std::path::PathBuf::from(stripped);
        }
    }
    pulita
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // --lang <codice> (o --lang=codice): la lingua della sessione, che
    // batte tutto il resto della catena. Gli altri argomenti restano
    // posizionali: il TUI ha un solo path, niente fioriture
    let mut cli_lang: Option<String> = None;
    let mut pos: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--lang" {
            if i + 1 >= args.len() {
                // --lang senza valore: lo si dice nella lingua che si
                // sarebbe usata, e si esce con la sindrome del -h rotto
                let (cat0, _) = resolve_lang(None);
                eprintln!("{}", cat0.t("tui.err_lang"));
                std::process::exit(2);
            }
            cli_lang = Some(args[i + 1].clone());
            i += 1;
        } else if let Some(v) = a.strip_prefix("--lang=") {
            cli_lang = Some(v.to_string());
        } else {
            pos.push(args[i].clone());
        }
        i += 1;
    }
    // il vocabolario di -h e degli errori pre-TUI: la stessa catena
    // dell'app (--lang > MFD_LANG > lingua= del profilo > inglese)
    let (cat, missing) = resolve_lang(cli_lang.clone());
    if let Some(code) = &missing {
        // codice chiesto e inesistente: avviso e si va dove dice il resto
        // della catena (mai un errore per una lingua che non c'e')
        let (langs, _) = mfd_core::lang::registry();
        let disp = langs
            .iter()
            .map(|l| l.code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("{}", cat.tf("tui.lang_ko", &[code, &disp]));
    }
    if pos.iter().any(|a| a == "-h" || a == "--help" || a == "/?") {
        print_help(&cat);
        return;
    }
    if pos.iter().any(|a| a == "-V" || a == "--version") {
        println!("mfd-tui {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let root = match pos.as_slice() {
        [p] => assoluta(p),
        // naked: la cartella corrente, in forma assoluta — la fascia dice
        // dove sei, e "." non lo dice. E' quello che tutti si aspettano
        // (du, ncdu, dua), senza il puntino addosso
        [] => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        _ => {
            eprintln!("{}", cat.t("tui.err_path"));
            std::process::exit(2);
        }
    };
    if !root.is_dir() {
        let dov = root.display().to_string();
        eprintln!("{}", cat.tf("tui.err_cartella", &[&dov]));
        std::process::exit(2);
    }
    let mut app = App::new(root, cli_lang);
    let mut terminal = ratatui::init();
    // il capture del mouse: senza non arrivano nemmeno rotella e click. Se
    // non parte, il TUI gira senza — le frecce restano — e lo si dice uscendo
    let mouse = enable_mouse();
    let res = event_loop(&mut terminal, &mut app);
    disable_mouse();
    ratatui::restore();
    if let Err(e) = mouse {
        let perche = e.to_string();
        eprintln!("{}", app.cat.tf("tui.err_mouse", &[&perche]));
    }
    if let Err(e) = res {
        let perche = e.to_string();
        eprintln!("{}", app.cat.tf("tui.err_generico", &[&perche]));
        std::process::exit(1);
    }
    // la prima scansione e' stata annullata con ESC: lo diciamo uscendo puliti
    if app.cancelled_first() {
        println!("{}", app.cat.t("tui.annullata"));
    }
}

/// Accende il capture del mouse: e' il prezzo per ricevere rotella e click
/// (crossterm non li manda senza). Mentre e' acceso il terminale manda
/// all'applicazione anche il drag, che qui non si usa — per
/// selezionare il testo del terminale si tiene Shift (i terminali moderni
/// cedono il passo all'utente). L'hook del panic si incatena a quello che
/// ratatui::init() ha gia' installato (alternate screen, raw mode): il
/// nostro spegne il mouse per primo, poi lascia a lui il terminale —
/// cosi' nemmeno un crash lo lascia rotto.
fn enable_mouse() -> std::io::Result<()> {
    use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = ratatui::crossterm::execute!(std::io::stdout(), DisableMouseCapture);
        prev(info);
    }));
    ratatui::crossterm::execute!(std::io::stdout(), EnableMouseCapture)
}

/// Il capture si spegne all'uscita pulita; sul panic ci pensa l'hook che
/// enable_mouse ha incatenato.
fn disable_mouse() {
    use ratatui::crossterm::event::DisableMouseCapture;
    let _ = ratatui::crossterm::execute!(std::io::stdout(), DisableMouseCapture);
}

/// L'event loop: disegna, legge un tasto (max ogni 40 ms), drena il canale
/// di progresso. Il TUI non si blocca mai sullo scan (regola di design).
fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        if ratatui::crossterm::event::poll(Duration::from_millis(40))? {
            match ratatui::crossterm::event::read()? {
                ratatui::crossterm::event::Event::Key(k) => {
                    // su Windows i tasti arrivano anche come Release: si agisce solo sul Press
                    if k.kind == ratatui::crossterm::event::KeyEventKind::Press {
                        app.on_key(k);
                    }
                }
                // il mouse intero va all'app: rotella e click si vestono
                // da tasti (on_mouse); il drag no — resta al terminale
                ratatui::crossterm::event::Event::Mouse(m) => app.on_mouse(m),
                _ => {}
            }
        }
        app.pump();
        if app.should_quit() {
            break;
        }
    }
    Ok(())
}
