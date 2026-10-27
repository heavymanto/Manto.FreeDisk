//! Il registro delle segnalazioni I/O (il "blocco note" della CLI): gli
//! accessi negati e simili sono warning, non errori — lo scan prosegue e
//! finisce con exit code 0.

use std::path::Path;

/// Il tipo di operazione fallita: la CLI lo renderizza col prefisso storico
/// ("apertura cartella ...", "enumerazione in ...", ...), il TUI a modo suo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarningKind {
    /// read_dir della cartella fallita.
    OpenDir,
    /// singola voce dell'enumerazione non leggibile.
    EnumEntry,
    /// file_type() di una voce fallito.
    EntryType,
    /// metadata() di un file fallito.
    Metadata,
}

/// Una segnalazione: dove (path completo), che operazione, il testo per
/// l'utente (gia' privo del suffisso tecnico "(os error N)", vedi scan::io_msg).
#[derive(Clone, Debug)]
pub struct Warning {
    pub path: std::path::PathBuf,
    pub kind: WarningKind,
    pub message: String,
}

/// Insieme delle segnalazioni di uno scan; l'ordine e' quello di scoperta.
#[derive(Default)]
pub struct WarningLog {
    entries: Vec<Warning>,
}

impl WarningLog {
    pub fn push(&mut self, w: Warning) {
        self.entries.push(w);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Warning> {
        self.entries.iter()
    }

    /// Quante segnalazioni cadono dentro il sottoalbero di `base` (per
    /// l'anteprima TUI: "N warning nel ramo"). Confronto per componenti del
    /// path, non per prefisso di stringa.
    pub fn count_under(&self, base: &Path) -> usize {
        self.entries.iter().filter(|w| w.path.starts_with(base)).count()
    }
}
