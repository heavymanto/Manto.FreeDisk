//! Filtro sui nomi dei file (-f/--filter della CLI, lente delle viste TUI).

use regex::{Regex, RegexBuilder};
use std::ffi::OsStr;

/// Filtro sui nomi dei file: i file che non matchano sono invisibili ai totali
/// (byte, contatori, foglie e loro errori), mentre le cartelle si attraversano
/// comunque tutte. Due sintassi: glob sul nome (jolly * e ?, es. "*.exe")
/// oppure regex nella forma REG(...) non ancorata (es. "REG(\.zip$)").
/// Case-insensitive come il filesystem Windows; `display` conserva il testo
/// originale per mostrarlo nel riepilogo.
pub struct Filter {
    re: Regex,
    display: String,
}

impl Filter {
    /// Compila la specifica (glob o REG(...)); Err riporta il testo per il
    /// messaggio d'errore del frontend.
    pub fn build(spec: &str) -> Result<Filter, String> {
        let pattern = if let Some(inner) = spec.strip_prefix("REG(").and_then(|r| r.strip_suffix(")")) {
            inner.to_string()
        } else {
            // glob -> regex: * e ? restano jolly, tutto il resto e' letterale;
            // il glob copre il nome per intero (regex ancorata)
            let mut re = String::from("^");
            for c in spec.chars() {
                match c {
                    '*' => re.push_str(".*"),
                    '?' => re.push('.'),
                    c => re.push_str(&regex::escape(&c.to_string())),
                }
            }
            re.push('$');
            re
        };
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(true)
            .build()
            .map_err(|e| format!("regex '{}' non valida: {}", pattern, e))?;
        Ok(Filter {
            re,
            display: spec.to_string(),
        })
    }

    /// Il nome matcha il filtro?
    pub fn is_match(&self, name: &OsStr) -> bool {
        self.re.is_match(&name.to_string_lossy())
    }

    /// La specifica originale, come scritta dall'utente (per i riepiloghi).
    pub fn display(&self) -> &str {
        &self.display
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_copre_il_nome_per_intero() {
        let f = Filter::build("*.dll").unwrap();
        assert!(f.is_match(std::ffi::OsStr::new("lib.DLL"))); // case-insensitive
        assert!(f.is_match(std::ffi::OsStr::new("a.dll")));
        assert!(!f.is_match(std::ffi::OsStr::new("a.dll.bak"))); // non suffisso
        assert!(!f.is_match(std::ffi::OsStr::new("dll")));
    }

    #[test]
    fn glob_jolly_singolo_carattere() {
        let f = Filter::build("log?.txt").unwrap();
        assert!(f.is_match(std::ffi::OsStr::new("log1.txt")));
        assert!(!f.is_match(std::ffi::OsStr::new("log12.txt")));
    }

    #[test]
    fn reg_non_ancorata() {
        let f = Filter::build("REG(\\.(zip|7z)$)").unwrap();
        assert!(f.is_match(std::ffi::OsStr::new("archivio.7z")));
        assert!(!f.is_match(std::ffi::OsStr::new("archivio.zip.exe")));
    }

    #[test]
    fn regex_non_valida_si_rifiuta() {
        // nota: "REG([" senza parentesi finale NON e' un errore: manca il
        // suffisso ")" e la specifica ricade nel ramo glob (nome letterale)
        assert!(Filter::build("REG([)").is_err());
    }

    #[test]
    fn display_conserva_la_specifica() {
        assert_eq!(Filter::build("*.exe").unwrap().display(), "*.exe");
    }
}
