//! Il disegno: schermata di scansione, schermata principale a due pannelli,
//! finestre (aiuto, warning, menu ordinamento, menu tema, prompt del filtro)
//! col loro velo d'ombra: il testo dietro scolorisce, l'effetto popup. Solo
//! qui si tocca ratatui; la logica sta in app.rs, i colori in theme.rs.
//! Le funzioni di formato (fmt_bytes, group, barra) sono le stesse della CLI:
//! duplicazione voluta — il core non formatta, la forma e' dei frontend — ma
//! la barra e' un marchio di prodotto e resta identica ovunque.

use crate::app::{ord_key, scroll_top, App, Focus, HitRows, Hits, Mode, ORDERS, PANE_ROWS};
use crate::theme::{Theme, ThemeId};
use mfd_core::lang::Catalog;
use mfd_core::{Filter, Progress, Row as CoreRow, Tree, Warning, WarningKind};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, Paragraph, Row as TRow, Table, TableState, Wrap,
};
use ratatui::Frame;
use std::time::Duration;

pub fn draw(f: &mut Frame, app: &mut App) {
    // le zone del click raccontano quest'ultimo frame: si azzera e si
    // riscrive — cio' che un frame non disegna non si clicca
    app.hits = Hits::default();
    match &app.mode {
        Mode::Scanning => draw_scanning(f, app),
        _ => {
            draw_main(f, app);
            // il tema in copia (pochi Style, tutti Copy): cosi' i menu
            // possono prendersi app mutabile — scrivono le zone del click —
            // senza prestiti in conflitto
            let t = app.theme;
            match &app.mode {
                Mode::Help => draw_help(f, app, &t),
                Mode::Warnings => draw_warnings(f, app, &t),
                Mode::SortMenu => draw_sort_menu(f, app, &t),
                Mode::ThemeMenu => draw_theme_menu(f, app, &t),
                Mode::FilterPrompt => draw_filter_prompt(f, app, &t),
                Mode::LangMenu => draw_lang_menu(f, app, &t),
                _ => {}
            }
        }
    }
}

/// La schermata di scansione: il box centrale del design, vestito come il
/// loader della CLI (spinner braille, ruota delle tinte, barra arcobaleno) —
/// la riga live della CLI e' questa schermata ridotta a una riga, ora si
/// somigliano. La percentuale e' una stima onesta (cartelle chiuse su chiuse
/// + ancora aperte): il totale non e' noto a priori e il walker va in
/// profondita', cosi' le aperte restano poche e la stima satura presto —
/// puo' restare al 99% per quasi tutta la scansione. Non e' un bug, e' il
/// prezzo di non raccontare percentuali inventate; a tenere vivo lo schermo
/// ci pensano l'orologio (spinner, tempo passato, velocita') e i contatori.
fn draw_scanning(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = f.area();
    paint_bg(f, area, t);
    let w = ((area.width as usize * 3) / 5).max(44).min(area.width as usize);
    let h = 10usize;
    let x = ((area.width as usize - w) / 2) as u16;
    let y = ((area.height as usize - h) / 2) as u16;
    let rect = Rect::new(x, y, w as u16, h as u16);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_active)
        .title(Span::styled(format!(" {} ", app.cat.t("scan.titolo")), t.title))
        .style(t.screen);
    shadow(f, rect, t);
    f.render_widget(Clear, rect);
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let (pct, files, dirs, bytes) = match &app.last_progress {
        Some(p) => (scan_pct(p), p.files, p.dirs, p.bytes),
        None => (0, 0, 0, 0),
    };
    // l'animazione vive di orologio: frame dello spinner, tempo passato e
    // velocita' camminano da soli, anche quando i contatori riposano
    let elapsed = app.scan_start.elapsed();
    let frame = spin_frame(elapsed);
    // la barra si prende il box meno il posto della percentuale ("100%" in
    // quattro colonne piu' lo spazio che la separa)
    let bw = (inner.width as usize).saturating_sub(5).max(10);
    let mut bar = scan_bar(pct, bw);
    bar.push(Span::raw(" "));
    bar.push(Span::styled(format!("{:>3}%", pct), t.title));
    let lines = vec![
        Line::from(vec![
            Span::styled(
                spin_char(frame).to_string(),
                Style::new().fg(spin_color(frame)),
            ),
            Span::raw(" "),
            Span::styled(
                shorten(
                    &app.root.display().to_string(),
                    (inner.width as usize).saturating_sub(2),
                ),
                t.fg,
            ),
        ]),
        Line::from(""),
        Line::from(bar),
        Line::from(format!(
            "{} · {}",
            app.cat.tf("scan.conti", &[&group(files), &group(dirs)]),
            fmt_bytes(bytes)
        ))
        .style(t.fg),
        Line::from(format!(
            "{} · {}",
            fmt_elapsed(elapsed),
            app.cat.tf("scan.rate", &[&group(file_rate(files, elapsed))])
        ))
        .style(t.fg),
        Line::from(""),
        Line::from(if app.cancelling {
            app.cat.t("scan.annulla").to_string()
        } else {
            app.cat.t("scan.esc").to_string()
        })
        .style(t.hint),
    ];
    f.render_widget(Paragraph::new(lines).style(t.screen), inner);
}

/// Il fondo del tema su tutta l'area (piatto, non trasparente).
fn paint_bg(f: &mut Frame, area: Rect, t: &Theme) {
    f.render_widget(Block::default().style(t.screen), area);
}

/// I dieci frame dello spinner braille della scansione: gli stessi del
/// loader della CLI (theme::loader) — i due vestiti sono gemelli, la riga
/// live della CLI e' questa schermata ridotta a una riga.
const SPIN: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// La ruota delle tinte dello spinner: dodici passi attorno alle sei
/// famiglie ANSI, ciascuna prima accesa e poi spenta — la tinta scorre e
/// respira, un ciclo pieno e' poco piu' di un secondo.
const WHEEL: [Color; 12] = [
    Color::LightBlue,
    Color::Blue,
    Color::LightCyan,
    Color::Cyan,
    Color::LightGreen,
    Color::Green,
    Color::LightYellow,
    Color::Yellow,
    Color::LightRed,
    Color::Red,
    Color::LightMagenta,
    Color::Magenta,
];

/// Lo spettro della barra di scansione: le sei tinte accese in ordine
/// d'arcobaleno. La parte piena le percorre cella per cella: la barra si
/// accende dal rosso e arriva al magenta man mano che la scansione chiude
/// cartelle — il gradiente e' spaziale, non un lampeggio (come il loader
/// della CLI).
const SPECTRUM: [Color; 6] = [
    Color::LightRed,
    Color::LightYellow,
    Color::LightGreen,
    Color::LightCyan,
    Color::LightBlue,
    Color::LightMagenta,
];

/// Il frame corrente dello spinner: un frame ogni 100 ms, la cadenza del
/// loader della CLI. L'animazione vive di orologio, non di eventi di
/// avanzamento: gira anche quando i contatori riposano (disco lento) o la
/// stima onesta satura al 99%.
fn spin_frame(elapsed: Duration) -> usize {
    (elapsed.as_millis() / 100) as usize
}

/// Il glifo dello spinner per quel frame.
fn spin_char(frame: usize) -> char {
    SPIN[frame % SPIN.len()]
}

/// La tinta dello spinner per quel frame.
fn spin_color(frame: usize) -> Color {
    WHEEL[frame % WHEEL.len()]
}

/// La stima onesta della percentuale: cartelle chiuse su chiuse + ancora
/// aperte. Non e' un fatto (il totale non e' noto a priori) e il walker va
/// in profondita', cosi' le aperte restano poche e la stima satura: puo'
/// stare al 99% per quasi tutta la scansione. Non le si fa dire cio' che
/// non sa — a tenere vivo lo schermo ci pensano spinner e orologio.
fn scan_pct(p: &Progress) -> u16 {
    let tot = p.closed + p.open;
    if tot > 0 {
        (p.closed * 100 / tot) as u16
    } else {
        0
    }
}

/// La barra arcobaleno della scansione: largo quanto il box meno il posto
/// della percentuale, stesso alfabeto # e . della griglia (la barra e' un
/// marchio di prodotto), tinta per cella lungo lo spettro man mano che si
/// riempie, vuoto in grigio scuro.
fn scan_bar(pct: u16, w: usize) -> Vec<Span<'static>> {
    let filled = ((pct as usize) * w / 100).min(w);
    let mut out = Vec::with_capacity(w);
    for i in 0..w {
        let (cell, c) = if i < filled {
            ("#", SPECTRUM[i * SPECTRUM.len() / w.max(SPECTRUM.len())])
        } else {
            (".", Color::DarkGray)
        };
        out.push(Span::styled(cell, Style::new().fg(c)));
    }
    out
}

/// Il tempo trascorso in colonna: minuti e secondi da sempre ("0:07"), le
/// ore quando servono ("1:02:03") — la riga si muove da sola, un tick al
/// secondo, anche quando tutto il resto riposa.
fn fmt_elapsed(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// La velocita' media della scansione: file chiusi al secondo, dall'inizio.
/// E' una media onesta, non un istantaneo che balla; a tempo zero il
/// denominatore non scende sotto il millisecondo.
fn file_rate(files: i64, elapsed: Duration) -> i64 {
    let secs = elapsed.as_secs_f64().max(0.001);
    (files as f64 / secs) as i64
}

/// Il percorso della scansione accorciato al budget: puntini in testa e
/// coda integra — come il loader della CLI (shorten di theme::loader), la
/// coda e' il nome che dice dove si sta.
fn shorten(path: &str, budget: usize) -> String {
    let n = path.chars().count();
    if n <= budget {
        return path.to_string();
    }
    let tail: String = path.chars().skip(n + 3 - budget).collect();
    format!("...{}", tail)
}

/// Barra proporzionale della griglia: 24 caratteri, IDENTICA all'algoritmo
/// della CLI (mfd-cli): la barra e' un marchio di prodotto (decisione 2).
fn bar24(bytes: i64, total: i64) -> String {
    const W: usize = 24;
    let filled = if total > 0 && bytes > 0 {
        (((bytes as f64 / total as f64) * W as f64).round() as usize).min(W)
    } else {
        0
    };
    format!("{}{}", "#".repeat(filled), ".".repeat(W - filled))
}

/// Formattazione leggibile della dimensione: la stessa della CLI.
fn fmt_bytes(b: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut u = 0usize;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{} B", b)
    } else {
        format!("{:.1} {}", v, UNITS[u]).replace('.', ",")
    }
}

/// Separatore delle migliaia stile italiano: lo stesso della CLI.
fn group_digits(n: i64) -> String {
    let s = n.to_string();
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for i in 0..b.len() {
        if i > 0 && (b.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(b[i] as char);
    }
    out
}

fn group(n: i64) -> String {
    group_digits(n)
}

/// Stringa allineata a destra in `w` colonne (le celle non hanno allineamento
/// proprio: il padding e' nel testo, come faceva il buon DOS).
fn fmt_r(s: &str, w: usize) -> String {
    format!("{:>w$}", s, w = w)
}

/// Stringa centrata in `w` colonne: la colonna SIZE della classifica la
/// vuole cosi' — il numero galleggia tra i suoi muri, non sta aggrappato
/// a destra come nella griglia (li' e' una colonna di allineamento, qui
/// una lettura).
fn fmt_c(s: &str, w: usize) -> String {
    let cut: String = s.chars().take(w).collect();
    format!("{:^w$}", cut, w = w)
}

/// Stringa troncata a `w` colonne tenendo la CODA (il nome sta in coda:
/// e' la parte che identifica) — il contrario di fit_path, per i nomi
/// dell'albero che crescono a sinistra con l'indentazione.
fn trunc(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n <= w {
        format!("{:<w$}", s, w = w)
    } else if w <= 3 {
        " ".repeat(w)
    } else {
        let tail: String = s.chars().skip(n + 3 - w).collect();
        format!("...{}", tail)
    }
}

/// Una freccia dello scroll dipinta sopra il bordo di un pannello: dice
/// "qui si scorre" senza rubare una riga ai dati — gialla come le
/// intestazioni, si vede da tre metri.
fn paint_arrow(f: &mut Frame, x: u16, y: u16, c: &str, style: Style) {
    if let Some(cell) = f.buffer_mut().cell_mut(Position::new(x, y)) {
        cell.set_symbol(c).set_style(style);
    }
}

/// La schermata principale: il quadro della CLI, vivo — la fascia del
/// percorso in cima (dove si lavora, tutto largo), l'albero e il dettaglio
/// in mezzo, la classifica in fondo (F9 la mostra e la nasconde). Le note
/// stanno fuori dal quadro, come nella CLI: lo status transitorio e la
/// barra dei tasti funzione chiudono lo schermo.
fn draw_main(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let t = &app.theme;
    paint_bg(f, area, t);
    let mut cons: Vec<Constraint> = vec![
        Constraint::Length(1), // la fascia del percorso
        Constraint::Min(3),    // albero | dettaglio
    ];
    // il pannello della classifica vuole il suo spazio: su un terminale
    // basso non ci sta senza schiacciare l'albero, e la navigazione
    // viene prima — il pannello aspetta uno schermo vero
    let want_pane = app.view.show_report && area.height >= 25;
    if want_pane {
        // il respiro prima della classifica: lo stesso che tiene l'albero
        // dal dettaglio. Una riga sola, perche' le celle sono piu' alte
        // che larghe: una riga vuota pesa agli occhi quanto il buco di
        // tre colonne tra i due box sopra
        cons.push(Constraint::Length(1));
        cons.push(Constraint::Length(14)); // la classifica (bordi, intestazione, separatore, 10 righe)
    }
    cons.push(Constraint::Length(1)); // lo status transitorio
    cons.push(Constraint::Length(1)); // la barra dei tasti
    let chunks = Layout::vertical(cons).split(area);
    draw_fascia(f, app, chunks[0]);
    let status = match app.status_now() {
        Some(s) => Line::from(format!(" {}", s)).style(t.status),
        None => {
            // il cursore nella classifica dichiara il suo file: nel pannello
            // il path si accorcia, qui lo si legge per intero — la riga
            // con i puntini trova la sua risposta senza allargare nulla
            if app.focus == Focus::Report {
                app.pane
                    .get(app.pane_sel)
                    .map(|(_, p)| Line::from(format!(" {}", p.display())).style(t.hint))
                    .unwrap_or_default()
            } else {
                Line::from("")
            }
        }
    };
    f.render_widget(Paragraph::new(status).style(t.screen), chunks[chunks.len() - 2]);
    f.render_widget(Paragraph::new(footer(t, &app.cat)).style(t.screen), chunks[chunks.len() - 1]);
    // tre zone in orizzontale: l'albero, il respiro, il dettaglio. I due
    // box non si toccano mai (un muro accanto a un muro si leggerebbe
    // come un muro solo, doppio) e pesano UGUALE: due Fill gemelli si
    // dividono cio' che resta dopo il respiro, meta' e meta' esatte a
    // qualunque larghezza. Il respiro e' di due colonne: le celle sono
    // circa il doppio piu' alte che larghe, due colonne pesano come la
    // riga vuota che sta sopra la classifica — lo stesso vuoto ovunque
    let cols = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .split(chunks[1]);
    draw_list(f, app, cols[0]);
    draw_preview(f, app, cols[2]);
    if want_pane {
        // chunks: [fascia, pannelli, respiro, classifica, status, footer]
        draw_classifica(f, app, chunks[3]);
    }
}

/// La fascia del percorso: il posto d'onore della CLI, qui in cima allo
/// schermo — la cartella dove si lavora, tutto largo, in blu come i path
/// della famiglia, con il TOTALE del ramo a destra: dichiarato col suo
/// nome, cosi' il numero non e' mai orfano — la somma dei size dell'albero
/// sotto e' lui (attraverso la lente, quando c'e'). La lente sta accanto
/// al percorso. Lunga: si accorcia in testa, la coda resta — la coda e'
/// dove si e'.
fn draw_fascia(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let Some(tree) = app.tree.clone() else { return };
    let disp_total = match &app.view.filter {
        Some(fl) => tree.subtree_match(app.current, fl).0,
        None => tree.metrics(app.current).bytes,
    };
    let mut path = tree.path_of(app.current).display().to_string();
    if let Some(fl) = &app.view.filter {
        path.push(' ');
        path.push_str(&app.cat.tf("cli.filtro", &[&fl.display()]));
    }
    let tot = app.cat.tf("tui.fascia_tot", &[&fmt_bytes(disp_total)]);
    let w = area.width as usize;
    let budget = w.saturating_sub(tot.chars().count() + 4);
    let shown = shorten(&path, budget);
    // il TOT a destra: il percorso a sinistra, il totale chiuso in fondo
    // alla riga — la barra di titolo del quadro
    let pad = w.saturating_sub(shown.chars().count() + tot.chars().count() + 2);
    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(shown, t.dir),
        Span::raw(" ".repeat(pad)),
        Span::styled(tot, t.title),
    ]);
    f.render_widget(Paragraph::new(line).style(t.screen), area);
}

/// La barra dei tasti funzione, alla Total Commander. Le scritte si
/// traducono, i numeri restano: F1-F12 e' la geografia della tastiera —
/// anche la stampa (F11) e la lingua (F12) sono gemelle F, e la barra le
/// dichiara come le altre. Il passo e' minimo, come in TC: le etichette
/// dei vocabolari sono corte apposta e su un terminale stretto la coda
/// si taglia da sola — F1 resta al suo posto.
fn footer<'a>(t: &Theme, cat: &'a Catalog) -> Line<'a> {
    const KEYS: [&str; 12] = [
        "footer.f1",
        "footer.f2",
        "footer.f3",
        "footer.f4",
        "footer.f5",
        "footer.f6",
        "footer.f7",
        "footer.f8",
        "footer.f9",
        "footer.f10",
        "footer.f11",
        "footer.f12",
    ];
    let mut spans: Vec<Span<'a>> = Vec::with_capacity(KEYS.len() * 2);
    for (i, k) in KEYS.iter().enumerate() {
        // il numero e' fisso a due colonne (" 1".."12"): la geografia non
        // si traduce, si numerano solo i posti
        spans.push(Span::styled(format!("{:2}", i + 1), t.key));
        spans.push(Span::raw(format!(" {} ", cat.t(k))));
    }
    Line::from(spans)
}

/// Il pannello di sinistra: L'ALBERO dei figli-cartella del nodo corrente
/// (decisione 3: i file vivono nel dettaglio), i rami aperti (x / F4)
/// indentati sotto il padre. Il vestito e' quello della griglia della
/// CLI: le colonne con i loro muri verticali, le giunzioni sugli angoli
/// — l'albero e' la griglia che si naviga. La selezione a video inverso
/// con la freccia `>`; le frecce dello scroll (▲ ▼) vivono sul bordo e
/// dicono che sopra o sotto c'e' ancora roba: una lista lunga non si
/// nasconde piu' dietro un pannello che sembra finito.
fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let Some(tree) = app.tree.clone() else { return };
    let cur = app.current;
    // il totale della cartella corrente attraverso la lente: la base di
    // percentuali e barre — lo dichiara la fascia in cima, qui lo usano i conti
    let disp_total = match &app.view.filter {
        Some(fl) => tree.subtree_match(cur, fl).0,
        None => tree.metrics(cur).bytes,
    };
    // il bordo attivo dice dov'e' il cursore: l'albero naviga, il resto si legge
    let border = if app.focus == Focus::List {
        t.border_active
    } else {
        t.border_idle
    };
    // la geometria: le colonne numeriche (SIZE 10, % 4, BARRA 24) coi
    // loro muri; i nomi vivono nello spazio aperto oltre l'ultimo muro,
    // fino al bordo pieno del pannello. Su un terminale stretto la barra
    // si mette via da sola: il nome vale piu' della barra (la % resta a
    // dire la proporzione)
    let inner_w = (area.width as usize).saturating_sub(2);
    let show_bar = app.view.show_bar && inner_w >= 10 + 1 + 4 + 1 + 24 + 1 + 8;
    let name_w = inner_w.saturating_sub(if show_bar { 41 } else { 16 });
    // la finestra delle righe: muro, intestazione, gradino, righe, muro
    let vis = (area.height as usize).saturating_sub(4);
    let top = scroll_top(app.sel, vis);
    let sopra = top > 0;
    let sotto = vis > 0 && top + vis < app.list.len();
    // il pannello intero e' cliccabile: click = cursore all'albero
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: inner_w as u16,
        height: area.height.saturating_sub(2),
    };
    app.hits.list_pane = Some(inner);

    let mut lines: Vec<Line> = Vec::new();
    // --- il muro di sopra: ┌───┬───┬───┬───┐ (con la ▲ se sopra c'e' altro)
    let mut spans: Vec<Span> = vec![Span::styled(
        format!(
            "┌{}┬{}{}┬",
            "─".repeat(10),
            "─".repeat(4),
            if show_bar {
                format!("┬{}", "─".repeat(24))
            } else {
                String::new()
            }
        ),
        border,
    )];
    if sopra && name_w >= 8 {
        spans.push(Span::styled("─".repeat(name_w - 5), border));
        spans.push(Span::styled("▲", t.header));
        spans.push(Span::styled("─".repeat(4), border));
    } else {
        spans.push(Span::styled("─".repeat(name_w), border));
    }
    spans.push(Span::styled("┐", border));
    lines.push(spans.into());
    // --- l'intestazione: le etichette numeriche centrate nelle loro
    // colonne (sono didascalie, non allineamenti), la NOME a sinistra
    // come i suoi dati — su una colonna larga un'etichetta centrata
    // galleggerebbe staccata
    let mut hdr: Vec<(String, Style)> = vec![
        (fmt_c(app.cat.t("lista.dim"), 10), t.header),
        ("│".into(), border),
        (fmt_c("%", 4), t.header),
        ("│".into(), border),
    ];
    if show_bar {
        // la barra non ha etichetta: e' la sua stessa etichetta — la
        // grammatica della famiglia (la CLI non la etichetta, la cella
        // resta muta tra i suoi muri, come ogni colonna grafica che sia)
        hdr.push((" ".repeat(24), t.header));
        hdr.push(("│".into(), border));
    }
    hdr.push((
        trunc(&format!("  {}", app.cat.t("lista.nome")), name_w),
        t.header,
    ));
    let mut spans: Vec<Span> = vec![Span::styled("│", border)];
    spans.extend(hdr.into_iter().map(|(s, st)| Span::styled(s, st)));
    spans.push(Span::styled("│", border));
    lines.push(spans.into());
    // --- il separatore dell'intestazione: ├─┼─┼─┼─┤
    lines.push(Line::from(Span::styled(
        format!(
            "├{}┼{}{}┼{}┤",
            "─".repeat(10),
            "─".repeat(4),
            if show_bar {
                format!("┼{}", "─".repeat(24))
            } else {
                String::new()
            },
            "─".repeat(name_w),
        ),
        border,
    )));
    // --- le righe: dalla finestra (top) per quante ne stanno (vis)
    if app.list.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(
                "│{}│",
                fmt_c(app.cat.t("lista.vuota"), inner_w.saturating_sub(2))
            ),
            t.hint,
        )));
    } else {
        let last = (top + vis).min(app.list.len());
        for i in top..last {
            let r = &app.list[i];
            let name = tree.name(r.id).to_string_lossy().into_owned();
            // il ramo: due spazi di indentazione per livello, il + / -
            // davanti al nome (solo le cartelle con figli) — ma
            // l'indentazione non mangia il nome: su un pannello stretto
            // si comprime prima lei
            let cap = name_w.saturating_sub(6) / 2;
            let indent = "  ".repeat(r.depth.min(cap));
            let toggle = if r.has_subdirs {
                if app.expanded.contains(&r.id) {
                    "- "
                } else {
                    "+ "
                }
            } else {
                "  "
            };
            // la percentuale: relativa alla cartella corrente (il parent),
            // che riga non e' — come la CLI: il 100% non dice niente, la
            // fascia dichiara il parent e la TOTALE ne tiene i numeri
            let pct = if disp_total > 0 { r.size * 100 / disp_total } else { 0 };
            let mark = if i == app.sel { ">" } else { " " };
            // la riga radice veste da TOTALE: la taglia in tinta titolo,
            // % e barra VUOTE (non corre con i figli — la grammatica della
            // TOTALE della CLI), il nome e' il punto della shell: dove sei
            let (data_st, pct_cell, bar_cell, name_cell) = if r.is_root {
                (
                    t.title,
                    " ".repeat(4),
                    " ".repeat(24),
                    // il punto sta dove partono i nomi (mark + toggle): la
                    // colonna ha un bordo sinistro pulito, il punto non
                    // galleggia due colonne piu' in la'
                    format!("{}  .", mark),
                )
            } else {
                (
                    t.fg,
                    fmt_r(&format!("{}%", pct), 4),
                    bar24(r.size, disp_total),
                    format!("{}{}{}{}\\", mark, indent, toggle, name),
                )
            };
            let mut parts: Vec<(String, Style)> = vec![
                (fmt_r(&fmt_bytes(r.size), 10), data_st),
                ("│".into(), border),
                (pct_cell, data_st),
                ("│".into(), border),
            ];
            if show_bar {
                parts.push((bar_cell, if r.is_root { t.title } else { t.bar }));
                parts.push(("│".into(), border));
            }
            parts.push((trunc(&name_cell, name_w), data_st));
            let mut spans: Vec<Span> = vec![Span::styled("│", border)];
            spans.extend(parts.into_iter().map(|(s, st)| Span::styled(s, st)));
            spans.push(Span::styled("│", border));
            // la riga selezionata si inverte per intero: ogni span veste il
            // selected (gli span con stile proprio vincerebbero la riga)
            let line = if i == app.sel {
                Line::from(
                    spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, t.selected))
                        .collect::<Vec<_>>(),
                )
            } else {
                Line::from(spans)
            };
            lines.push(line);
        }
    }
    // --- i muri laterali continuano finche' il pannello e' alto: le righe
    // mancanti sono righe vuote tra i muri, il fondo sta sempre in fondo
    while lines.len() + 1 < area.height as usize {
        lines.push(Line::from(vec![
            Span::styled("│", border),
            Span::raw(" ".repeat(inner_w)),
            Span::styled("│", border),
        ]));
    }
    // --- il muro di sotto: └───┴───┴───┴───┘ (con la ▼ se sotto c'e' altro)
    let mut spans: Vec<Span> = vec![Span::styled(
        format!(
            "└{}┴{}{}┴",
            "─".repeat(10),
            "─".repeat(4),
            if show_bar {
                format!("┴{}", "─".repeat(24))
            } else {
                String::new()
            }
        ),
        border,
    )];
    if sotto && name_w >= 8 {
        spans.push(Span::styled("─".repeat(name_w - 5), border));
        spans.push(Span::styled("▼", t.header));
        spans.push(Span::styled("─".repeat(4), border));
    } else {
        spans.push(Span::styled("─".repeat(name_w), border));
    }
    spans.push(Span::styled("┘", border));
    lines.push(spans.into());

    f.render_widget(Paragraph::new(lines).style(t.screen), area);
    // le righe cliccabili: sotto intestazione e separatore, dalla finestra
    // corrente — il click fa lo stesso conto del render o sbaglierebbe riga
    if inner.height > 2 {
        app.hits.list = Some(HitRows {
            rect: Rect {
                y: inner.y + 2,
                height: inner.height - 2,
                ..inner
            },
            top,
            stride: 1,
        });
    }
}

/// La classifica in fondo: i 10 file piu' grandi del ramo selezionato,
/// ricorsivi — il report della CLI, vivo: scorri l'albero e la risposta
/// ti segue (F9 la mostra e la nasconde). Il vestito e' la sezione report
/// del quadro della CLI: due colonne col muro in mezzo, la SIZE centrata
/// nella sua (li' galleggia, non sta aggrappata a destra), il titolo
/// nell'intestazione accanto al PARENTE in blu — di chi sono questi dieci
/// (il path si accorcia in testa se serve: la coda e' il nome, la parte
/// che parla). TAB ci porta il cursore: le frecce la leggono riga per
/// riga, la status dice il path per intero di quella puntata.
fn draw_classifica(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let Some(tree) = app.tree.clone() else { return };
    let focused = app.focus == Focus::Report;
    let border = if focused { t.border_active } else { t.border_idle };
    // la geometria: SIZE centrata in 12, il path in tutto il resto — un
    // muro in mezzo, come la sezione report della CLI
    let inner_w = (area.width as usize).saturating_sub(2);
    let size_w = 12usize;
    let path_w = inner_w.saturating_sub(size_w + 1);
    let vis = (area.height as usize).saturating_sub(4).max(PANE_ROWS);
    let top = app.pane_top.min(app.pane.len().saturating_sub(vis));
    let sopra = top > 0;
    let sotto = vis > 0 && top + vis < app.pane.len();
    // il pannello e' cliccabile (click e rotella portano qui il cursore)
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: inner_w as u16,
        height: area.height.saturating_sub(2),
    };
    app.hits.report_pane = Some(area);

    let mut lines: Vec<Line> = Vec::new();
    // --- il muro di sopra (▲ se sopra c'e' altro)
    let mut spans: Vec<Span> = vec![Span::styled(format!("┌{}┬", "─".repeat(size_w)), border)];
    if sopra && path_w >= 8 {
        spans.push(Span::styled("─".repeat(path_w - 5), border));
        spans.push(Span::styled("▲", t.header));
        spans.push(Span::styled("─".repeat(4), border));
    } else {
        spans.push(Span::styled("─".repeat(path_w), border));
    }
    spans.push(Span::styled("┐", border));
    lines.push(spans.into());
    // --- l'intestazione: la colonna dichiarata, il titolo centrato, il
    // parente del ramo a destra — la classifica segue la selezione
    let base = app.selected_id().unwrap_or(app.current);
    let parente = tree.path_of(base).display().to_string();
    // il titolo dice il taglio CHIESTO: le cifre della navigazione lo
    // riscrivono al volo (25 file, 100...) e il singolare ha la sua parola
    let n_str = app.report_n.to_string();
    let titolo = if app.report_n == 1 {
        app.cat.t("cli.top_file").to_string()
    } else {
        app.cat.tf("cli.top_files", &[&n_str])
    };
    let destro = format!(" {} ", shorten(&parente, path_w / 3));
    let mid = path_w.saturating_sub(titolo.chars().count() + destro.chars().count());
    let lpad = mid / 2;
    let hdr: Vec<(String, Style)> = vec![
        (fmt_c(app.cat.t("lista.dim"), size_w), t.header),
        ("│".into(), border),
        (
            format!(
                "{}{}{}",
                " ".repeat(lpad),
                titolo,
                " ".repeat(mid - lpad)
            ),
            t.header,
        ),
        (destro, t.dir),
    ];
    let mut spans: Vec<Span> = vec![Span::styled("│", border)];
    spans.extend(hdr.into_iter().map(|(s, st)| Span::styled(s, st)));
    spans.push(Span::styled("│", border));
    lines.push(spans.into());
    // --- il separatore: ├────┼────┤
    lines.push(Line::from(Span::styled(
        format!("├{}┼{}┤", "─".repeat(size_w), "─".repeat(path_w)),
        border,
    )));
    // --- le righe: size centrata, il path accorciato in testa (il nome
    // sta in coda); la riga del cursore si inverte quando TAB e' qui.
    // Lo scroll orizzontale (frecce SX/DX col cursore TAB) sposta la
    // finestra di lettura lungo il path: i puntini davanti dicono che
    // c'e' altro a sinistra
    let h = app.pane_hscroll;
    let last = (top + vis).min(app.pane.len());
    for i in top..last {
        let (s, p) = &app.pane[i];
        let sel = focused && i == app.pane_sel;
        let full = p.display().to_string();
        let path_cell = if h == 0 {
            trunc(&fit_path(p, path_w), path_w)
        } else {
            // la finestra scorre: salta i primi h caratteri e mostra
            // i successivi path_w, coi puntini che dicono "prima c'era"
            let chars: Vec<char> = full.chars().collect();
            if h >= chars.len() {
                " ".repeat(path_w)
            } else {
                let shown: String = chars[h..].iter().take(path_w.saturating_sub(3)).collect();
                format!("...{}{}", shown, " ".repeat(path_w.saturating_sub(3).saturating_sub(shown.chars().count())))
            }
        };
        let parts: Vec<(String, Style)> = vec![
            (fmt_c(&fmt_bytes(*s), size_w), t.fg),
            ("│".into(), border),
            (path_cell, t.dir),
        ];
        let mut spans: Vec<Span> = vec![Span::styled("│", border)];
        spans.extend(parts.into_iter().map(|(s, st)| Span::styled(s, st)));
        spans.push(Span::styled("│", border));
        let line = if sel {
            Line::from(
                spans
                    .into_iter()
                    .map(|s| Span::styled(s.content, t.selected))
                    .collect::<Vec<_>>(),
            )
        } else {
            Line::from(spans)
        };
        lines.push(line);
    }
    // --- i muri laterali fino al fondo (il pannello e' alto fisso)
    while lines.len() + 1 < area.height as usize {
        lines.push(Line::from(vec![
            Span::styled("│", border),
            Span::raw(" ".repeat(inner_w)),
            Span::styled("│", border),
        ]));
    }
    // --- il muro di sotto (▼ se sotto c'e' altro)
    let mut spans: Vec<Span> = vec![Span::styled(format!("└{}┴", "─".repeat(size_w)), border)];
    if sotto && path_w >= 8 {
        spans.push(Span::styled("─".repeat(path_w - 5), border));
        spans.push(Span::styled("▼", t.header));
        spans.push(Span::styled("─".repeat(4), border));
    } else {
        spans.push(Span::styled("─".repeat(path_w), border));
    }
    spans.push(Span::styled("┘", border));
    lines.push(spans.into());

    f.render_widget(Paragraph::new(lines).style(t.screen), area);
    // le righe cliccabili: sotto intestazione e separatore, dalla finestra
    if !app.pane.is_empty() && inner.height > 2 {
        app.hits.report = Some(HitRows {
            rect: Rect {
                y: inner.y + 2,
                height: inner.height - 2,
                ..inner
            },
            top,
            stride: 1,
        });
    }
}

/// Il pannello di destra: il contenuto della voce selezionata attraverso la
/// lente unica (la stessa della lista, dichiarata in testa). Con TAB prende
/// il cursore in LETTURA (video inverso, le frecce scorrono, INVIO non
/// entra), albero aperto o chiuso: chi naviga sta nell'albero; senza focus
/// e' la lente che si legge.
fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let Some(tree) = app.tree.clone() else { return };
    let sel_name = app
        .list
        .get(app.sel)
        .map(|r| {
            // la riga radice si chiama punto, come nel disegno: il nome
            // del nodo radice e' una stringa vuota e il titolo non può
            // restare muto
            if r.is_root {
                ".".to_string()
            } else {
                tree.name(r.id).to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| "-".to_string());
    // il bordo dice dov'e' il cursore: col cursore a destra si scorre il
    // dettaglio in lettura, l'albero resta il posto dove si naviga
    let focused = app.focus == Focus::Detail;
    let border = if focused { t.border_active } else { t.border_idle };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Span::styled(
            format!(" {} ", app.cat.tf("dettaglio.titolo", &[&sel_name])),
            t.title,
        ))
        .style(t.screen);
    let inner = block.inner(area);
    // il pannello del dettaglio e' cliccabile ovunque, bordi compresi:
    // click = cursore in lettura, come TAB
    app.hits.detail_pane = Some(area);
    f.render_widget(block, area);
    // il fondo riserva la riga del warning SOLO quando c'e' un warning da
    // dire: senza, le righe del dettaglio hanno tutto il pannello — una
    // lista lunga non si ferma due righe prima facendo credere che sia
    // finita li' (regressione vista dall'utente)
    let n_warn = app
        .selected_id()
        .map(|sid| tree.warnings().count_under(&tree.path_of(sid)))
        .unwrap_or(0);
    let mut cons = vec![
        Constraint::Length(1), // i totali della selezione: il dato piu' importante
        Constraint::Length(1), // la lente dichiarata
        Constraint::Length(1), // separatore
        Constraint::Min(1),    // le righe
    ];
    if n_warn > 0 {
        cons.push(Constraint::Length(1)); // il warning del ramo, quando c'e'
    }
    let rows_area = Layout::vertical(cons).split(inner);
    // il totale delle dimensioni di quello che sta sotto: e' il dato piu'
    // importante del dettaglio, quindi apre il pannello, non lo chiude
    if let Some(sid) = app.selected_id() {
        let totals = detail_totals(&tree, sid, app.view.filter.as_ref(), &app.cat);
        f.render_widget(
            Paragraph::new(Line::from(totals).style(t.title)).style(t.screen),
            rows_area[0],
        );
    }
    let mut lens = format!(
        " {}: {}",
        app.cat.t("lens.ord"),
        app.cat.t(ord_key(app.view.order))
    );
    if let Some(fl) = &app.view.filter {
        lens.push_str(&format!("   {}: {}", app.cat.t("lens.fil"), fl.display()));
    }
    f.render_widget(
        Paragraph::new(Line::from(lens).style(t.header)).style(t.screen),
        rows_area[1],
    );
    let sep = "-".repeat(inner.width.saturating_sub(2) as usize);
    f.render_widget(
        Paragraph::new(Line::from(sep).style(t.border_idle)).style(t.screen),
        rows_area[2],
    );
    // il top delle righe cliccabili segue il render: col cursore si scrolla
    // su dsel, senza si parte dall'alto (il click che da' il focus trovera'
    // le righe dove le vede)
    let top = if focused {
        scroll_top(app.dsel, rows_area[3].height as usize)
    } else {
        0
    };
    // si costruiscono SOLO le righe visibili: il dettaglio di una cartella
    // grossa (WinSxS: ~1.800 righe) ricostruito a ogni frame mangiava
    // mezzo processore da fermo — la finestra che si vede e' tutto cio'
    // che serve al render
    let vis = rows_area[3].height as usize;
    let last = (top + vis).min(app.preview.len());
    let prow: Vec<TRow> = app.preview[top..last]
        .iter()
        .enumerate()
        .map(|(i, p)| {
            // il cursore del dettaglio: la freccia `>` come nella lista,
            // a video inverso quando il focus e' qui (l'indice e' assoluto:
            // la fetta parte da top)
            let abs = top + i;
            let mark = if focused && abs == app.dsel { ">" } else { " " };
            let (cells, kind) = match &p.row {
                CoreRow::Dir(j) => (
                    vec![
                        Cell::from(format!("{}{}\\", mark, tree.name(*j).to_string_lossy())),
                        Cell::from(fmt_r(&fmt_bytes(p.size), 10)),
                    ],
                    t.dir,
                ),
                CoreRow::Leaf(_, n) => (
                    vec![
                        Cell::from(format!("{}{}", mark, n.to_string_lossy())),
                        Cell::from(fmt_r(&fmt_bytes(p.size), 10)),
                    ],
                    t.file,
                ),
            };
            if focused && abs == app.dsel {
                TRow::new(cells).style(t.selected)
            } else {
                TRow::new(cells).style(kind)
            }
        })
        .collect();
    let table = Table::new(prow, [Constraint::Min(6), Constraint::Length(10)]).column_spacing(1);
    if focused {
        // il cursore esiste solo col focus: e' lui che scrolla il pannello.
        // La fetta parte da top, quindi il selezionato e' relativo a lei
        let mut state = TableState::default()
            .with_selected(Some(app.dsel.saturating_sub(top)));
        f.render_stateful_widget(table, rows_area[3], &mut state);
    } else {
        f.render_widget(table, rows_area[3]);
    }
    if !app.preview.is_empty() {
        app.hits.detail = Some(HitRows {
            rect: rows_area[3],
            top,
            stride: 1,
        });
    }
    // il fondo: solo i warning del ramo — i totali e la lente in cifre stanno
    // in cima al pannello, dove si guardano per primi
    if n_warn > 0 {
        let msg = if n_warn == 1 {
            app.cat.t("dettaglio.warn1").to_string()
        } else {
            app.cat.tf("dettaglio.warnN", &[&group(n_warn as i64)])
        };
        f.render_widget(
            Paragraph::new(Line::from(msg).style(t.hint)).style(t.screen),
            rows_area[4],
        );
    }
    // le frecce dello scroll sul bordo: sopra o sotto c'e' altro — il
    // pannello non sembra finito quando non lo e'
    let vis = rows_area[3].height as usize;
    if vis > 0 && top > 0 {
        paint_arrow(f, area.x + area.width.saturating_sub(3), area.y, "▲", t.header);
    }
    if vis > 0 && top + vis < app.preview.len() {
        paint_arrow(
            f,
            area.x + area.width.saturating_sub(3),
            area.y + area.height.saturating_sub(1),
            "▼",
            t.header,
        );
    }
}

/// La riga di totali in cima al DETTAGLIO: totale, file e cartelle della
/// selezione — il dato piu' importante, la somma delle righe elencate sotto.
/// Con un filtro attivo conta solo cio' che la lente lascia passare (le
/// cartelle non si filtrano: si attraversano; i file matchanti invece si).
pub fn detail_totals(tree: &Tree, sid: u32, filter: Option<&Filter>, cat: &Catalog) -> String {
    match filter {
        Some(fl) => {
            let (b, n) = tree.subtree_match(sid, fl);
            cat.tf(
                "dettaglio.totale_fil",
                &[&fmt_bytes(b), &group(n), &fl.display().to_string()],
            )
        }
        None => {
            let m = tree.metrics(sid);
            cat.tf(
                "dettaglio.totale",
                &[&fmt_bytes(m.bytes), &group(m.files), &group(m.dirs)],
            )
        }
    }
}

/// Un rettangolo centrato, in percentuale dell'area totale.
fn centered(area: Rect, pw: u16, ph: u16) -> Rect {
    let w = area.width * pw / 100;
    let h = area.height * ph / 100;
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

/// L'ombra della finestra: il rettangolo spostato di una cella in basso a
/// destra non viene cancellato ma velato — i caratteri che stanno dietro
/// restano e scoloriscono (testo in grigio scuro, fondo scuro dove il tema
/// lo impone): e' l'effetto popup delle console vere, non la lastra di
/// colore che copre lo sfondo. Restare dentro lo schermo e' garantito dal
/// taglio sull'area del frame.
fn shadow(f: &mut Frame, rect: Rect, t: &Theme) {
    let fa = f.area();
    let sx = rect.x.saturating_add(1);
    let sy = rect.y.saturating_add(1);
    let ex = ((rect.x as u32 + rect.width as u32 + 1).min(fa.x as u32 + fa.width as u32)) as u16;
    let ey = ((rect.y as u32 + rect.height as u32 + 1).min(fa.y as u32 + fa.height as u32)) as u16;
    if sx < ex && sy < ey {
        for y in sy..ey {
            for x in sx..ex {
                if let Some(cell) = f.buffer_mut().cell_mut((x, y)) {
                    cell.set_fg(t.shadow_fg);
                    if let Some(bg) = t.shadow_bg {
                        cell.set_bg(bg);
                    }
                }
            }
        }
    }
}

/// Una finestra bordata centrata: ombra sotto (la sovrapposizione), pulizia,
/// fondo e titolo del tema. Ritorna l'area interna dove va il contenuto.
fn window(f: &mut Frame, t: &Theme, title: &str, pw: u16, ph: u16) -> Rect {
    let rect = centered(f.area(), pw, ph);
    shadow(f, rect, t);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_active)
        .title(Span::styled(format!(" {} ", title), t.title))
        .style(t.dialog);
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    inner
}

/// Una riga tasto -> azione per l'aiuto.
fn kline(t: &Theme, key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<20}", key), t.key),
        Span::raw(desc.to_string()),
    ])
}

/// L'aiuto (F1 o ?): la tabella dei tasti della sezione 5 del design. Le
/// 24 voci e le 13 righe di coda arrivano dal vocabolario (aiuto.k01..k24,
/// d01..d24, h01..h13): la tabella si traduce intera, niente resta cablato
/// — e la voce Shift+<lettera> e' l'ultima arrivata, col suo posto in fila.
fn draw_help(f: &mut Frame, app: &App, t: &Theme) {
    let inner = window(f, t, app.cat.t("aiuto.titolo"), 72, 96);
    let mut lines: Vec<Line> = Vec::new();
    for i in 1..=24 {
        let k = format!("aiuto.k{:02}", i);
        let d = format!("aiuto.d{:02}", i);
        lines.push(kline(t, app.cat.t(&k), app.cat.t(&d)));
    }
    lines.push(Line::from(""));
    for i in 1..=13 {
        let h = format!("aiuto.h{:02}", i);
        lines.push(Line::from(app.cat.t(&h).to_string()).style(t.hint));
    }
    lines.shrink_to_fit();
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).style(t.dialog),
        inner,
    );
}

/// La finestra dei warning (w o F8): il blocco note della CLI, navigabile.
/// Si spiega da sola: cosa sono le segnalazioni, perche' non sono errori e
/// cosa comportano nei totali (il motivo per cui vale guardarla).
fn draw_warnings(f: &mut Frame, app: &App, t: &Theme) {
    let Some(tree) = app.tree.clone() else { return };
    let n = tree.warnings().len();
    let title = if n == 0 {
        app.cat.t("warn.titolo0").to_string()
    } else if n == 1 {
        app.cat.t("warn.titolo1").to_string()
    } else {
        app.cat.tf("warn.titoloN", &[&group(n as i64)])
    };
    let inner = window(f, t, &title, 78, 70);
    let mut lines: Vec<Line> = Vec::new();
    if n == 0 {
        for k in ["warn.e1a", "warn.e1b"] {
            lines.push(Line::from(app.cat.t(k).to_string()).style(t.fg));
        }
        lines.push(Line::from(""));
        for k in ["warn.e2a", "warn.e2b", "warn.e2c"] {
            lines.push(Line::from(app.cat.t(k).to_string()).style(t.hint));
        }
    } else {
        for k in ["warn.p1", "warn.p2", "warn.p3", "warn.p4", "warn.p5"] {
            lines.push(Line::from(app.cat.t(k).to_string()).style(t.fg));
        }
        lines.push(Line::from(""));
        for w in tree.warnings().iter().skip(app.warn_top) {
            lines.push(
                Line::from(format!("  - {}", render_warning(w, &app.cat))).style(t.fg),
            );
        }
    }
    // i file di lingua scoperti ma non leggibili si dichiarano qui, dove
    // finiscono le cose che non si sono potute leggere (DESIGN-I18N)
    if !app.lang_errors.is_empty() {
        lines.push(Line::from(""));
        let msg = app.cat.tf("warn.lang_ko", &[&app.lang_errors.join(", ")]);
        lines.push(Line::from(msg).style(t.hint));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(app.cat.t("warn.chiudi").to_string()).style(t.hint));
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).style(t.dialog),
        inner,
    );
}

/// Il testo di una segnalazione: stessi prefissi del blocco note della CLI
/// (coerenza di prodotto; la forma e' dei frontend, quindi qui una copia).
fn render_warning(w: &Warning, cat: &Catalog) -> String {
    let key = match w.kind {
        WarningKind::OpenDir => "warn.k_opendir",
        WarningKind::EnumEntry => "warn.k_enum",
        WarningKind::EntryType => "warn.k_type",
        WarningKind::Metadata => "warn.k_meta",
    };
    let what = cat.tf(key, &[&w.path.display().to_string()]);
    format!("{}: {}", what, w.message)
}


/// Il path del report nella larghezza che resta: se non ci sta si tiene la
/// coda (il nome del file e' la parte per cui si guarda) e si dichiara il
/// taglio con "..." — tagliare a destra perderebbe il nome.
fn fit_path(p: &std::path::Path, w: usize) -> String {
    let s = p.display().to_string();
    let len = s.chars().count();
    if len <= w {
        return s;
    }
    let keep = w.saturating_sub(3);
    let tail: String = s.chars().skip(len - keep).collect();
    format!("...{}", tail)
}

/// Il menu dell'ordinamento (F6): le otto combinazioni di -o con il codice.
fn draw_sort_menu(f: &mut Frame, app: &mut App, t: &Theme) {
    let inner = window(f, t, app.cat.t("menu.ordina"), 48, 50);
    // le voci del menu sono cliccabili: una riga l'una, dall'alto
    app.hits.menu = Some(HitRows {
        rect: inner,
        top: 0,
        stride: 1,
    });
    let mut lines: Vec<Line> = Vec::new();
    for (i, o) in ORDERS.iter().enumerate() {
        let sel = i == app.menu_sel;
        let marker = if sel { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}{} ", marker, o.code()),
                if sel { t.selected } else { t.key },
            ),
            Span::styled(
                app.cat.t(ord_key(*o)).to_string(),
                if sel { t.selected } else { t.fg },
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(app.cat.t("menu.invio_esc").to_string()).style(t.hint));
    f.render_widget(Paragraph::new(lines).style(t.dialog), inner);
}

/// Il menu dei temi (t o F3): i quattro temi della sezione 6 del design, col
/// contrassegno su quello in uso. INVIO applica, ESC annulla.
fn draw_theme_menu(f: &mut Frame, app: &mut App, t: &Theme) {
    let inner = window(f, t, app.cat.t("menu.tema"), 58, 36);
    // le voci del menu sono cliccabili: DUE righe l'una (nome e blurb) —
    // cliccare il blurb e' cliccare la voce
    app.hits.menu = Some(HitRows {
        rect: inner,
        top: 0,
        stride: 2,
    });
    let mut lines: Vec<Line> = Vec::new();
    for (i, id) in ThemeId::ALL.iter().enumerate() {
        let sel = i == app.menu_sel;
        let in_use = *id == app.theme_id;
        let marker = if sel { "> " } else { "  " };
        let tag = if in_use {
            format!("   {}", app.cat.t("tema.in_uso"))
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}{}{}", marker, app.cat.t(id.key()), tag),
                if sel { t.selected } else { t.fg },
            ),
        ]));
        lines.push(
            Line::from(format!("   {}", app.cat.t(id.blurb_key()))).style(t.hint),
        );
    }
    lines.push(Line::from(""));
    lines.push(Line::from(app.cat.t("menu.invio_esc").to_string()).style(t.hint));
    f.render_widget(Paragraph::new(lines).style(t.dialog), inner);
}

/// La finestra del filtro (f o F7): il campo di immissione in cima e,
/// sotto, istruzioni ed esempi delle due sintassi (glob e REG(...)). Una
/// finestra e non una riga in fondo: lo schermo e' grande e la sintassi
/// della regex merita di stare davanti agli occhi mentre si scrive.
fn draw_filter_prompt(f: &mut Frame, app: &App, t: &Theme) {
    let inner = window(f, t, app.cat.t("filtro.titolo"), 78, 90);
    let mut lines: Vec<Line> = Vec::new();
    // il campo: l'input e il cursore in evidenza (il colore della selezione)
    lines.push(Line::from(vec![
        Span::styled(app.cat.t("filtro.campo").to_string(), t.title),
        Span::styled(app.filter_input.clone(), t.selected),
        Span::styled("_", t.selected), // il cursore
    ]));
    // l'ultimo errore di compilazione, se c'e': resta finche' non si corregge
    if let Some(e) = &app.prompt_error {
        lines.push(Line::from(format!(" {}", e)).style(t.status));
    }
    lines.push(Line::from(""));
    // la semantica di -f, detta a parole
    for k in ["filtro.sem1", "filtro.sem2", "filtro.sem3", "filtro.sem4"] {
        lines.push(Line::from(app.cat.t(k).to_string()).style(t.fg));
    }
    lines.push(Line::from(""));
    // primo modo: glob
    lines.push(Line::from(app.cat.t("filtro.glob_t").to_string()).style(t.title));
    for k in ["filtro.glob1", "filtro.glob2", "filtro.glob3", "filtro.glob4"] {
        lines.push(Line::from(app.cat.t(k).to_string()).style(t.hint));
    }
    lines.push(Line::from(""));
    // secondo modo: REG(...)
    lines.push(Line::from(app.cat.t("filtro.reg_t").to_string()).style(t.title));
    lines.push(
        Line::from(app.cat.t("filtro.reg_ancora").to_string()).style(t.title),
    );
    for k in ["filtro.reg1", "filtro.reg2", "filtro.reg3"] {
        lines.push(Line::from(app.cat.t(k).to_string()).style(t.hint));
    }
    // il promemoria dei pezzi di regex che servono davvero
    for k in ["filtro.mem1", "filtro.mem2"] {
        lines.push(Line::from(app.cat.t(k).to_string()).style(t.hint));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(app.cat.t("filtro.chiudi").to_string()).style(t.status));
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).style(t.dialog),
        inner,
    );
}

/// Il menu delle lingue (l o F12): l'inglese incorporato piu' quelle
/// scoperte nella cartella del profilo, il contrassegno su quella in uso
/// e — per le traduzioni generate da AI — il marchio [AI] col suo
/// disclaimer in fondo: chi sceglie sa cosa sta scegliendo.
fn draw_lang_menu(f: &mut Frame, app: &mut App, t: &Theme) {
    let inner = window(f, t, app.cat.t("menu.lingua"), 58, 42);
    // le voci sono cliccabili: una riga l'una, dall'alto
    app.hits.menu = Some(HitRows {
        rect: inner,
        top: 0,
        stride: 1,
    });
    let mut lines: Vec<Line> = Vec::new();
    let in_use_code = app.cat.active.code.clone();
    let any_ai = app.langs.iter().any(|l| l.ai);
    for (i, l) in app.langs.iter().enumerate() {
        let sel = i == app.menu_sel;
        let marker = if sel { "> " } else { "  " };
        let mut spans = vec![Span::styled(
            if l.code == in_use_code {
                format!("{}{}   {}", marker, l.name, app.cat.t("tema.in_uso"))
            } else {
                format!("{}{}", marker, l.name)
            },
            if sel { t.selected } else { t.fg },
        )];
        if l.ai {
            spans.push(Span::styled(
                format!(" {}", app.cat.t("lang.ai_tag")),
                t.hint,
            ));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));
    if any_ai {
        lines.push(Line::from(app.cat.t("lang.ai_nota").to_string()).style(t.hint));
    }
    lines.push(Line::from(app.cat.t("menu.invio_esc").to_string()).style(t.hint));
    f.render_widget(Paragraph::new(lines).style(t.dialog), inner);
}


#[cfg(test)]
mod tests {
    //! Il rendering vero e proprio: con TestBackend ratatui disegna in memoria
    //! e da qui si legge il testo del buffer. Regressione di un bug reale
    //! trovato dall'utente: la tabella del dettaglio finiva nell'area da una
    //! riga del separatore (pannello destro con UNA sola riga visibile) e la
    //! riga della lente copriva i totali in cima al pannello.

    use super::*;
    use mfd_core::{scan, ScanOptions};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Instant;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// La stessa fixture dei test di app.rs, ridotta al minimo che serve qui:
    /// la selezione di default e' alpha (2 file: data.dat e lib.dll), quindi
    /// il DETTAGLIO ha 2 righe da mostrare, non una.
    fn fixture(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mfd-ui-test-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("alpha")).unwrap();
        std::fs::create_dir_all(dir.join("beta/sub")).unwrap();
        std::fs::write(dir.join("alpha/lib.dll"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.join("alpha/data.dat"), vec![0u8; 2000]).unwrap();
        std::fs::write(dir.join("beta/note.txt"), vec![0u8; 50]).unwrap();
        dir
    }

    /// Il testo del buffer come un'unica stringa (le righe compresse).
    fn screen_text(terminal: &Terminal<TestBackend>) -> String {
        let mut out = String::new();
        for cell in terminal.backend().buffer().content() {
            out.push_str(cell.symbol());
        }
        out
    }

    /// Lo specchio: stampa la schermata intera su file per leggerla con
    /// occhi propri prima di farla guardare a chiunque (`cargo test
    /// debug_dump -- --ignored`). Ignorato dalla CI: e' un attrezzo
    /// diagnostico, non un contratto
    #[test]
    #[ignore]
    fn debug_dump_schermata() {
        let fx = fixture("dump");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // 123x42: la larghezza del terminale dell'utente
        let mut terminal = Terminal::new(TestBackend::new(123, 42)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..42 {
            for x in 0..123 {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        let p = std::env::temp_dir().join("mfd-schermata.txt");
        std::fs::write(&p, out).unwrap();
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn il_dettaglio_mostra_totali_e_piu_di_una_riga() {
        let fx = fixture("righe");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // i totali in cima al DETTAGLIO restano visibili: la riga della lente
        // (ord/fil) sta sulla riga sotto, non sopra di loro. Selezione di
        // default: la riga radice — il dettaglio racconta la cartella
        assert!(screen.contains("DETTAGLIO: ."), "titolo: {screen:?}");
        assert!(
            screen.contains("3,0 KB - 3 file - 3 cartelle"),
            "totali: {screen:?}"
        );
        // il dettaglio elenca TUTTE le sue righe: il bug ne mostrava una sola
        // (la tabella finiva nell'area da 1 riga riservata al separatore)
        assert!(screen.contains("alpha\\"), "prima riga: {screen:?}");
        assert!(screen.contains("beta\\"), "seconda riga: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn il_popup_del_filtro_mostra_istruzioni_ed_esempi() {
        let fx = fixture("fil-popup");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        app.on_key(key(KeyCode::Char('F')));
        for c in "*.dll".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // il campo di immissione mostra cio' che si sta scrivendo
        assert!(screen.contains("*.dll"), "campo: {screen:?}");
        // le due sintassi, spiegate e con gli esempi (la richiesta che ha
        // fatto nascere il popup: capire subito come funziona REG)
        assert!(screen.contains("GLOB"), "glob: {screen:?}");
        assert!(screen.contains("REG(...)"), "reg: {screen:?}");
        assert!(screen.contains("REG(\\.(zip|7z)$)"), "esempio reg: {screen:?}");
        assert!(screen.contains("INVIO applica"), "tasti: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn la_finestra_dei_warning_si_spiega_da_sola() {
        let fx = fixture("warn-fx");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        app.on_key(key(KeyCode::Char('W')));
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // la fixture non genera intoppi: la finestra lo dice senza giri
        assert!(screen.contains("nessuna segnalazione"), "titolo: {screen:?}");
        // e dice a cosa serve (il feedback che ha fatto nascere il testo:
        // F8 non era capito)
        assert!(screen.contains("conteggio"), "scopo: {screen:?}");
        assert!(screen.contains("ESC o Shift+W chiude"), "tasti: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn l_albero_disegna_rami_indentati_con_più_meno() {
        // la stessa struttura dei test di app.rs serve qui: un ramo chiuso
        // (alpha), uno con figli (gamma/x), un fratello in coda (beta)
        let dir = std::env::temp_dir().join(format!(
            "mfd-ui-test-{}-albero",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("alpha")).unwrap();
        std::fs::create_dir_all(dir.join("beta/sub")).unwrap();
        std::fs::create_dir_all(dir.join("gamma/x")).unwrap();
        std::fs::write(dir.join("readme.txt"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("alpha/lib.dll"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.join("alpha/data.dat"), vec![0u8; 2000]).unwrap();
        std::fs::write(dir.join("beta/note.txt"), vec![0u8; 50]).unwrap();
        std::fs::write(dir.join("beta/sub/altro.log"), vec![0u8; 70]).unwrap();
        std::fs::write(dir.join("gamma/x/y.dll"), vec![0u8; 300]).unwrap();
        std::fs::write(dir.join("gamma/z.tmp"), vec![0u8; 10]).unwrap();
        let tree = Arc::new(scan(
            &dir,
            &ScanOptions {
                depth: usize::MAX,
                filter: None,
            },
        ));
        let mut app = App::with_tree(dir.clone(), tree);
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma
        app.on_key(key(KeyCode::Char('X'))); // apre il ramo
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // la riga radice in testa, col suo punto
        assert!(screen.contains(" ."), "radice: {screen:?}");
        // il ramo chiuso col + (beta ha una sotto-cartella), quello aperto
        // col - sulla selezione (gamma: freccia + trattino), il figlio
        // indentato (x, foglia dell'albero: due spazi, niente simbolo)
        assert!(screen.contains("+ beta\\"), "chiuso: {screen:?}");
        assert!(screen.contains(">- gamma\\"), "aperto: {screen:?}");
        assert!(screen.contains("  x\\"), "figlio indentato: {screen:?}");
        assert!(screen.contains("   alpha\\"), "foglia dell'albero: {screen:?}");
        // e il footer dichiara il nuovo tasto funzione
        assert!(screen.contains("Espandi"), "footer: {screen:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn il_dettaglio_col_cursore_mostra_la_freccia() {
        let fx = fixture("dettaglio-cursore");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // TAB porta in lettura anche con l'albero chiuso
        app.on_key(key(KeyCode::Down)); // alpha: dettaglio a 2 righe
        app.on_key(key(KeyCode::Tab)); // il cursore passa al dettaglio
        app.on_key(key(KeyCode::Down)); // seconda riga: lib.dll
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // la freccia sta sulla riga puntata dal cursore del dettaglio...
        assert!(screen.contains(">lib.dll"), "cursore: {screen:?}");
        // ...e NON su quella sopra (il marchio e' lo spazio)
        assert!(screen.contains(" data.dat"), "non puntata: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn la_classifica_disegna_taglia_e_nome_dei_big() {
        let fx = fixture("report-render");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // la riga radice e' selezionata: la classifica in fondo e' il
        // report dell'intera cartella — il globale, alla radice dello scan
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // il titolo dichiara il taglio chiesto (10 di fabbrica)...
        assert!(screen.contains("I 10 FILE PIU' GRANDI"), "titolo: {screen:?}");
        // ...e le righe sono in ordine: il piu' grande in testa
        assert!(screen.contains("2,0 KB"), "taglia: {screen:?}");
        assert!(screen.contains("data.dat"), "primo: {screen:?}");
        assert!(screen.contains("lib.dll"), "secondo: {screen:?}");
        // e il footer dichiara il tasto che la nasconde
        assert!(screen.contains("Report"), "footer: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }

    /// Il contratto del click: le zone che il disegno dichiara sono le righe
    /// che il disegno mostra. Lista lunga (15 cartelle) in un pannello corto
    /// (7 righe visibili): la selezione in fondo scrolla, e il click sulla
    /// prima riga visibile deve prendere la riga che si vede lì — non la
    /// prima dei dati.
    #[test]
    fn le_zone_del_click_seguono_il_disegno() {
        let dir = std::env::temp_dir().join(format!(
            "mfd-ui-test-{}-click-zone",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for i in 1..=15 {
            std::fs::create_dir_all(dir.join(format!("f{:02}", i))).unwrap();
            std::fs::write(dir.join(format!("f{:02}/x.bin", i)), vec![0u8; i * 100]).unwrap();
        }
        let tree = Arc::new(scan(&dir, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(dir.clone(), tree);
        app.sel = 15; // l'ultima riga (f01): la radice ha preso la prima
        app.refresh();
        let mut terminal = Terminal::new(TestBackend::new(120, 12)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // la lista e' scrollata e il render lo dichiara: la prima riga
        // visibile e' f06 (la fascia del percorso si e' presa una riga:
        // una in meno da scorrere), la selezione f01 sta in fondo con la
        // freccia
        let z = app.hits.list.expect("la zona della lista c'e'");
        assert!(z.top > 0, "la lista scrolla: top={}", z.top);
        // sotto la fascia, il bordo del pannello, l'intestazione e il suo
        // separatore (il vestito della griglia ha una riga in piu'):
        // le righe dati partono a y=4
        assert_eq!(z.rect.y, 4);
        assert!(screen.contains("f05"), "prima riga visibile: {screen:?}");
        assert!(screen.contains(">  f01"), "selezione visibile in fondo: {screen:?}");
        // il click sulla prima riga visibile seleziona la riga che si vede
        app.on_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: z.rect.y,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.sel, z.top);
        let name = app
            .tree
            .as_ref()
            .unwrap()
            .name(app.list[app.sel].id)
            .to_string_lossy()
            .into_owned();
        assert_eq!(name, "f05", "il click prende la riga disegnata");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tab_arriva_anche_alla_classifica_e_le_frecce_la_leggono() {
        let fx = fixture("tab-report");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        // due TAB: albero -> dettaglio -> classifica (il pannello c'e',
        // l'ultimo frame l'ha disegnato: 30 righe lo tengono)
        app.on_key(key(KeyCode::Tab));
        app.on_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Report, "due TAB e si legge la classifica");
        // le frecce muovono il cursore della classifica, non la selezione
        let sel = app.sel;
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.pane_sel, 1, "il cursore della classifica scende");
        assert_eq!(app.sel, sel, "l'albero non si tocca");
        // il terzo TAB chiude il giro: si torna all'albero
        app.on_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::List, "il giro e' completo");
        // F9 spegne il pannello: il cursore non resta senza casa
        app.on_key(key(KeyCode::Tab));
        app.on_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Report);
        app.on_key(key(KeyCode::F(9)));
        assert_eq!(app.focus, Focus::List, "senza pannello si torna all'albero");
        assert!(!app.view.show_report);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn la_classifica_veste_il_box_della_cli_con_la_size_centrata() {
        let fx = fixture("classifica-box");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // la size galleggia al centro della sua colonna, tra i due muri —
        // non sta aggrappata a destra come nella griglia
        assert!(screen.contains("│   2,0 KB   │"), "size centrata: {screen:?}");
        // il muro tra le colonne e la giunzione del separatore: il vestito
        // della sezione report della CLI
        assert!(screen.contains('┼'), "giunzioni: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn le_frecce_dello_scroll_si_vedono_sul_bordo() {
        // dodici cartelle in un terminale basso (25 righe: il minimo che
        // tiene la classifica col suo respiro): il pannello SX mostra la
        // ▼ (sotto c'e' altro), e dopo un po' di frecce compare la ▲ —
        // una lista lunga non sembra finita dove il pannello finisce
        let dir = std::env::temp_dir().join(format!(
            "mfd-ui-test-scroll-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for i in 0..12 {
            std::fs::create_dir_all(dir.join(format!("d{:02}", i))).unwrap();
            std::fs::write(dir.join(format!("d{:02}/f.bin", i)), vec![0u8; 100 * (i + 1)])
                .unwrap();
        }
        let tree = Arc::new(scan(&dir, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(dir.clone(), tree);
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        assert!(screen.contains('▼'), "freccia giu' in fondo: {screen:?}");
        assert!(!screen.contains('▲'), "in cima non c'e' niente sopra: {screen:?}");
        // tre giu': la finestra scorre, adesso sopra c'e' altro
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        assert!(screen.contains('▲'), "freccia su dopo lo scroll: {screen:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn il_menu_delle_lingue_marca_le_traduzioni_ai() {
        let fx = fixture("lang-ai");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // inglese (la referenza) piu' un tedesco _ai: il marchio e il
        // disclaimer si devono vedere MENTRE si sceglie — e' la richiesta
        // che ha fatto nascere il meta _ai
        let de = mfd_core::lang::Lang::from_json_str(
            "de",
            "{\"_name\": \"Deutsch\", \"_ai\": \"KI-generierte Uebersetzung\"}",
        )
        .unwrap();
        app.langs = vec![mfd_core::lang::Lang::english(), de];
        app.on_key(key(KeyCode::Char('L')));
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        assert!(screen.contains("English"), "inglese in lista: {screen:?}");
        assert!(screen.contains("Deutsch"), "tedesco in lista: {screen:?}");
        assert!(screen.contains("[AI]"), "marchio AI: {screen:?}");
        // il disclaimer in fondo: chi sceglie sa cosa sta scegliono
        assert!(
            screen.contains("tradotte da AI"),
            "nota AI in italiano (il test e' italiano): {screen:?}"
        );
        assert!(screen.contains("INVIO applica"), "tasti: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn lo_spinner_della_scansione_gira_col_tempo() {
        // un frame ogni 100 ms, la cadenza del loader della CLI: l'animazione
        // vive di orologio, non di eventi di avanzamento — cosi' gira anche
        // quando la stima onesta satura al 99% e i contatori riposano
        assert_eq!(spin_frame(Duration::from_millis(0)), 0);
        assert_eq!(spin_frame(Duration::from_millis(99)), 0);
        assert_eq!(spin_frame(Duration::from_millis(100)), 1);
        // il braille avanza di un glifo...
        assert_eq!(spin_char(0), '⠋');
        assert_eq!(spin_char(1), '⠙');
        // ...e la ruota con lui: tinta accesa, poi spenta della stessa
        // famiglia, finche' al giro dei dodici passi ricomincia
        assert_eq!(spin_color(0), Color::LightBlue);
        assert_eq!(spin_color(1), Color::Blue);
        assert_eq!(spin_color(12), Color::LightBlue);
    }

    #[test]
    fn la_barra_arcobaleno_si_riempie_seguendo_lo_spettro() {
        // meta' piena: i # percorrono lo spettro rosso->magenta, i . restano
        // in grigio scuro — il gradiente e' spaziale, la tinta dice quanto
        // manca, non un lampeggio
        let spans = scan_bar(50, 10);
        let chars: String = spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(chars, "#####.....");
        assert_eq!(spans[0].style, Style::new().fg(Color::LightRed));
        assert_eq!(spans[4].style, Style::new().fg(Color::LightGreen));
        assert_eq!(spans[5].style, Style::new().fg(Color::DarkGray));
    }

    #[test]
    fn la_percentuale_della_scansione_resta_la_stima_onesta() {
        let p = |closed: i64, open: i64| Progress {
            files: 0,
            dirs: 0,
            bytes: 0,
            errors: 0,
            closed,
            open,
        };
        // a scansione appena nata non si sa niente: zero
        assert_eq!(scan_pct(&p(0, 0)), 0);
        assert_eq!(scan_pct(&p(3, 1)), 75);
        // il famoso 99 del walker in profondita': onesto, ci pensa lo
        // spinner a tenere vivo lo schermo
        assert_eq!(scan_pct(&p(99, 1)), 99);
    }

    #[test]
    fn tempo_e_velocita_della_scansione_si_leggono_subito() {
        assert_eq!(fmt_elapsed(Duration::from_secs(7)), "0:07");
        assert_eq!(fmt_elapsed(Duration::from_secs(42)), "0:42");
        assert_eq!(fmt_elapsed(Duration::from_secs(3723)), "1:02:03");
        // la velocita' e' una media onesta dall'inizio, non un istantaneo
        // che balla; a tempo zero non divide mai per zero
        assert_eq!(file_rate(100, Duration::from_secs(10)), 10);
        assert_eq!(file_rate(0, Duration::from_secs(0)), 0);
    }

    #[test]
    fn il_percorso_della_scansione_si_accorcia_coi_puntini() {
        let lungo = "a".repeat(40);
        let s = shorten(&lungo, 28);
        assert_eq!(s.chars().count(), 28);
        assert!(s.starts_with("..."));
        assert!(s.ends_with(&"a".repeat(25)));
        assert_eq!(shorten("corto", 28), "corto");
    }

    #[test]
    fn la_schermata_di_scansione_veste_spinner_barra_e_orologio() {
        let fx = fixture("scan-vestito");
        let tree =
            Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // si entra a mano nella scansione, con una fotografia quasi a fine
        // coda e l'orologio che cammina da due minuti: il disegno guarda lo
        // stato, non il canale
        app.mode = Mode::Scanning;
        app.last_progress = Some(Progress {
            files: 1000,
            dirs: 12,
            bytes: 3530,
            errors: 0,
            closed: 99,
            open: 1,
        });
        app.scan_start = Instant::now() - Duration::from_secs(120);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);
        // lo spinner c'e' e gira (quale glifo dipende dal millisecondo del
        // frame: uno dei dieci, non uno fisso)
        assert!(
            SPIN.iter().any(|c| screen.contains(*c)),
            "spinner: {screen:?}"
        );
        // la barra quasi piena e la percentuale onesta del 99
        assert!(screen.contains("99%"), "percentuale: {screen:?}");
        assert!(screen.contains("#####"), "barra: {screen:?}");
        // i contatori di sempre: file, cartelle e ora anche i byte
        assert!(screen.contains("1.000 file - 12 cartelle"), "conti: {screen:?}");
        assert!(screen.contains("3,4 KB"), "byte: {screen:?}");
        // la riga nuova: tempo passato e velocita' media — si muovono da
        // sole, un tick al secondo, anche a contatori fermi
        assert!(screen.contains("2:00"), "tempo: {screen:?}");
        assert!(screen.contains("8 file/s"), "velocita: {screen:?}");
        assert!(screen.contains("ESC annulla"), "tasti: {screen:?}");
        let _ = std::fs::remove_dir_all(&fx);
    }
}
