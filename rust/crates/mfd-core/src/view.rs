//! Le viste sull'albero: righe della ripartizione (sotto-cartelle e foglie)
//! e il loro ordinamento. E' il lessico di -o della CLI, riusato dal TUI:
//! un solo posto decide cosa vuol dire "ordinato per taglia decrescente".

use crate::tree::Tree;
use std::ffi::{OsStr, OsString};

/// Ordinamento delle righe (opzione -o/--order della CLI, tasto s del TUI):
/// T = taglia (dimensione ricorsiva), A = alfanumerico (nome), F = file
/// (conteggio ricorsivo), C = cartelle (conteggio ricorsivo); direzione
/// D = decrescente, C = crescente. Default T.D: la riga piu' grande in cima.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Order {
    TD,
    TC,
    AC,
    AD,
    FD,
    FC,
    CD,
    CC,
}

impl Order {
    /// Accetta la forma puntata ("T.D") o compatta ("td"), in ogni casing.
    pub fn parse(s: &str) -> Option<Order> {
        match s.to_ascii_uppercase().replace('.', "").as_str() {
            "TD" => Some(Order::TD),
            "TC" => Some(Order::TC),
            "AD" => Some(Order::AD),
            "AC" => Some(Order::AC),
            "FD" => Some(Order::FD),
            "FC" => Some(Order::FC),
            "CD" => Some(Order::CD),
            "CC" => Some(Order::CC),
            _ => None,
        }
    }

    /// Etichetta del criterio per titoli e menù (il lessico di -o, senza forma).
    pub fn label(&self) -> &'static str {
        match self {
            Order::TD => "taglia decrescente",
            Order::TC => "taglia crescente",
            Order::AD => "nome decrescente",
            Order::AC => "nome crescente",
            Order::FD => "file decrescente",
            Order::FC => "file crescente",
            Order::CD => "cartelle decrescente",
            Order::CC => "cartelle crescente",
        }
    }

    /// Il codice del criterio, come lo scrive -o ("T.D"): per menù e titoli.
    pub fn code(&self) -> &'static str {
        match self {
            Order::TD => "T.D",
            Order::TC => "T.C",
            Order::AD => "A.D",
            Order::AC => "A.C",
            Order::FD => "F.D",
            Order::FC => "F.C",
            Order::CD => "C.D",
            Order::CC => "C.C",
        }
    }

    /// Il criterio successivo nel ciclo del tasto s (T/A/F/C x D/C).
    pub fn next(self) -> Order {
        match self {
            Order::TD => Order::TC,
            Order::TC => Order::AC,
            Order::AC => Order::AD,
            Order::AD => Order::FD,
            Order::FD => Order::FC,
            Order::FC => Order::CD,
            Order::CD => Order::CC,
            Order::CC => Order::TD,
        }
    }
}

/// Riga di una vista sulla ripartizione: sotto-cartella (nodo dell'albero) o
/// file diretto (foglia del nodo della sua cartella).
#[derive(Clone)]
pub enum Row {
    Dir(u32),
    Leaf(i64, OsString),
}

/// Metriche di una riga per l'ordinamento -o: dal nodo (totali ricorsivi)
/// oppure, per le foglie, dal file stesso (una foglia e' 1 file, 0 cartelle,
/// la sua taglia).
pub struct RowMetrics<'a> {
    pub bytes: i64,
    pub files: i64,
    pub dirs: i64,
    pub name: &'a OsStr,
}

impl Tree {
    /// Le righe dei figli di `id`: sotto-cartelle e file diretti insieme,
    /// nell'ordine di scoperta (prima di qualsiasi ordinamento).
    pub fn rows_of(&self, id: u32) -> Vec<Row> {
        let mut rows: Vec<Row> = self.children(id).iter().map(|&i| Row::Dir(i)).collect();
        for l in self.leaves(id) {
            rows.push(Row::Leaf(l.size, l.name));
        }
        rows
    }

    /// Metriche di una riga ai fini dell'ordinamento.
    pub fn row_metrics<'a>(&'a self, r: &'a Row) -> RowMetrics<'a> {
        match r {
            Row::Dir(i) => {
                let m = self.metrics(*i);
                RowMetrics {
                    bytes: m.bytes,
                    files: m.files,
                    dirs: m.dirs,
                    name: self.name(*i),
                }
            }
            Row::Leaf(s, name) => RowMetrics {
                bytes: *s,
                files: 1,
                dirs: 0,
                name,
            },
        }
    }

    /// Ordina le righe secondo `order` (stesso comparatore della griglia -d:
    /// prima la metrica, poi il nome case-insensitive, poi la taglia).
    pub fn sort_rows(&self, rows: &mut [Row], order: Order) {
        rows.sort_by(|a, b| {
            let ma = self.row_metrics(a);
            let mb = self.row_metrics(b);
            let ka = name_key(ma.name);
            let kb = name_key(mb.name);
            match order {
                // T/F/C: prima la metrica (D = la grande in cima), poi il nome, poi la taglia
                Order::TD => mb.bytes.cmp(&ma.bytes).then_with(|| ka.cmp(&kb)),
                Order::TC => ma.bytes.cmp(&mb.bytes).then_with(|| ka.cmp(&kb)),
                Order::FD => mb
                    .files
                    .cmp(&ma.files)
                    .then_with(|| ka.cmp(&kb))
                    .then_with(|| mb.bytes.cmp(&ma.bytes)),
                Order::FC => ma
                    .files
                    .cmp(&mb.files)
                    .then_with(|| ka.cmp(&kb))
                    .then_with(|| mb.bytes.cmp(&ma.bytes)),
                Order::CD => mb
                    .dirs
                    .cmp(&ma.dirs)
                    .then_with(|| ka.cmp(&kb))
                    .then_with(|| mb.bytes.cmp(&ma.bytes)),
                Order::CC => ma
                    .dirs
                    .cmp(&mb.dirs)
                    .then_with(|| ka.cmp(&kb))
                    .then_with(|| mb.bytes.cmp(&ma.bytes)),
                // A: nome senza maiuscole/minuscole, poi nome esatto, poi taglia
                Order::AC => ka
                    .cmp(&kb)
                    .then_with(|| ma.name.cmp(&mb.name))
                    .then_with(|| mb.bytes.cmp(&ma.bytes)),
                Order::AD => kb
                    .cmp(&ka)
                    .then_with(|| mb.name.cmp(&ma.name))
                    .then_with(|| mb.bytes.cmp(&ma.bytes)),
            }
        });
    }
}

/// Chiave case-insensitive del nome per l'ordinamento alfanumerico (-o A.*):
/// a parita' di chiave decide il confronto esatto del nome.
fn name_key(n: &OsStr) -> String {
    n.to_string_lossy().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_forme_puntate_e_compatte() {
        assert_eq!(Order::parse("T.D"), Some(Order::TD));
        assert_eq!(Order::parse("ac"), Some(Order::AC));
        assert_eq!(Order::parse("f.d"), Some(Order::FD));
        assert_eq!(Order::parse("X.Y"), None);
        assert_eq!(Order::parse("TDD"), None);
    }

    #[test]
    fn ciclo_del_tasto_s_copre_tutte_le_combinazioni() {
        let mut o = Order::TD;
        let mut visti = vec![o];
        for _ in 0..7 {
            o = o.next();
            visti.push(o);
        }
        assert_eq!(o.next(), Order::TD);
        let mut unici = visti.clone();
        unici.dedup();
        assert_eq!(visti.len(), unici.len());
        assert_eq!(visti.len(), 8);
    }
}
