//! Le lingue del prodotto (docs/DESIGN-I18N.md — il contratto): l'inglese
//! incorporato nel binario, ogni altra lingua un file JSON che la community
//! puo' aggiungere senza toccare una riga di codice. Niente dipendenze: il
//! lettore e' interno, minimale, e si prova in coppia con lo scrittore di
//! json.rs (il progetto possiede entrambe le meta', scrittura e lettura).
//!
//! Il formato dei file: un oggetto piatto chiave -> valore, tutte e due
//! stringhe. Le chiavi che iniziano con "_" sono meta, non vocabolario:
//! `_name` e' come la lingua si chiama nel menu, `_ai` dichiara una
//! traduzione generata da AI (il menu la marca col suo disclaimer — chi
//! sceglie sa cosa sta scegliendo). La degradazione e' per chiave, non per
//! file: una lingua incompleta mostra inglese solo dove manca la
//! traduzione, e una chiave che manca anche all'inglese stampa se stessa
//! (un gap si vede subito e si sistema in un minuto).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Il vocabolario inglese, incorporato nel binario: il fallback garantito,
/// sempre disponibile anche senza un solo file sul disco. E' anche la
/// referenza dei traduttori — chiavi e valori inglesi sono il vocabolario
/// base (lo dice DESIGN-I18N).
const EN_JSON: &str = include_str!("../lang/en.json");

/// Una lingua: il codice (per i file scoperti e' lo stem del nome), il nome
/// nel menu (`_name`, il codice se manca) e il dizionario. `ai` e' vero
/// quando il file porta `_ai`: la traduzione e' stata generata da AI e il
/// menu la dichiara (potrebbe essere imprecisa o errata).
#[derive(Clone, Debug)]
pub struct Lang {
    pub code: String,
    pub name: String,
    pub ai: bool,
    dict: HashMap<String, String>,
}

impl Lang {
    /// L'inglese incorporato. Il file sta nel binario: se non va e' un bug
    /// di build, non di runtime — il panic qui e' la risposta giusta.
    pub fn english() -> Lang {
        Lang::from_json_str("en", EN_JSON)
            .expect("en.json incorporato: valido per costruzione (gate di parita')")
    }

    /// Una lingua dal suo testo JSON (il codice lo dichiara chi chiama: per
    /// i file scoperti e' lo stem del nome). Err se il testo non e' un
    /// oggetto piatto di stringhe: la lingua non si carica — non e' un
    /// errore dell'app, si salta il file e lo si dice.
    pub fn from_json_str(code: &str, text: &str) -> Result<Lang, String> {
        let dict = parse_flat(text)?;
        let name = dict
            .get("_name")
            .cloned()
            .unwrap_or_else(|| code.to_string());
        let ai = dict.contains_key("_ai");
        Ok(Lang {
            code: code.to_string(),
            name,
            ai,
            dict,
        })
    }

    /// Una lingua dal file: il codice e' lo stem del nome (il patto con la
    /// community: una lingua, un file, il nome e' il codice).
    pub fn from_file(path: &Path) -> Result<Lang, String> {
        let code = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        Lang::from_json_str(code, &text)
    }

    /// Il valore di una chiave, meta comprese (`_name`, `_ai`: chi vuole il
    /// disclaimer AI lo legge dal file stesso).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.dict.get(key).map(|s| s.as_str())
    }

    /// Le chiavi del vocabolario (meta escluse), in ordine: per il gate di
    /// parita' — tutte le lingue di lang/ devono avere le stesse chiavi di
    /// en.json, e la completezza non si revisiona a mano.
    pub fn keys(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .dict
            .keys()
            .filter(|k| !k.starts_with('_'))
            .map(|k| k.as_str())
            .collect();
        v.sort();
        v
    }
}

/// La lingua attiva col fallback inglese incorporato. La catena di ricerca
/// per chiave: attiva -> inglese -> la chiave stessa. La attiva si cambia a
/// runtime (menu l / F12 del TUI): il fallback resta l'inglese del binario.
#[derive(Clone, Debug)]
pub struct Catalog {
    pub active: Lang,
    pub fallback: Lang,
}

impl Catalog {
    /// Il catalogo di fabbrica: inglese attivo, inglese di riserva.
    pub fn english() -> Catalog {
        let en = Lang::english();
        Catalog {
            fallback: en.clone(),
            active: en,
        }
    }

    /// Una lingua attiva qualsiasi, con l'inglese incorporato di riserva.
    pub fn with(active: Lang) -> Catalog {
        Catalog {
            fallback: Lang::english(),
            active,
        }
    }

    /// Il codice della lingua attiva (per il menu, per il profilo).
    pub fn code(&self) -> &str {
        &self.active.code
    }

    /// Una stringa del vocabolario: attiva, poi inglese, poi la chiave.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.active
            .get(key)
            .or_else(|| self.fallback.get(key))
            .unwrap_or(key)
    }

    /// La stringa con gli argomenti al posto dei segnaposto `{}`, in ordine.
    /// Niente macro format! sulle traduzioni: una grafa sbagliata in un
    /// file community non puo' far panicare niente — un segnaposto senza
    /// argomento resta vuoto, un argomento di piu' si ignora.
    pub fn tf(&self, key: &str, args: &[&str]) -> String {
        let pattern = self.t(key);
        let mut out = String::with_capacity(pattern.len());
        let mut i = 0usize;
        for (n, part) in pattern.split("{}").enumerate() {
            if n > 0 {
                if let Some(a) = args.get(i) {
                    out.push_str(a);
                }
                i += 1;
            }
            out.push_str(part);
        }
        out
    }
}
/// La cartella delle lingue del profilo utente: %APPDATA%\mfd\lang su
/// Windows, ~/.config/mfd/lang altrove (la stessa scelta dei tool da
/// terminale). Porta il nome del prodotto, non del frontend: le lingue le
/// usera' anche la CLI (fase 2 di DESIGN-I18N) e un solo posto le trova
/// tutte. L'utente droppa un file → la lingua compare nel menu al
/// prossimo avvio, zero codice toccato.
pub fn profile_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("mfd").join("lang"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(|d| PathBuf::from(d).join(".config").join("mfd").join("lang"))
    }
}

/// Le lingue disponibili: l'inglese incorporato piu' quelle scoperte nella
/// cartella data. Un file scoperto con lo stesso codice di uno incorporato
/// lo sopravanza (il profilo vince sul fabbrica); un file illeggibile non
/// e' un errore — la lingua non compare e il suo nome finisce tra gli
/// illeggibili, per la segnalazione (finestra w del TUI, stderr nella CLI:
/// mai un crash). L'inglese sta in testa, le altre in ordine di codice:
/// il menu le mostra cosi'.
pub fn registry_in(dir: Option<&Path>) -> (Vec<Lang>, Vec<String>) {
    let mut langs = vec![Lang::english()];
    let mut unreadable = Vec::new();
    if let Some(dir) = dir {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut found = Vec::new();
            for e in entries.flatten() {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) != Some("json") {
                    continue;
                }
                match Lang::from_file(&path) {
                    Ok(l) => found.push(l),
                    Err(_) => unreadable.push(
                        path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    ),
                }
            }
            found.sort_by(|a, b| a.code.cmp(&b.code));
            for l in found {
                if let Some(slot) = langs.iter_mut().find(|x| x.code == l.code) {
                    *slot = l;
                } else {
                    langs.push(l);
                }
            }
        }
    }
    (langs, unreadable)
}

/// Le lingue disponibili, dalla cartella del profilo: l'inglese incorporato
/// piu' quelle che l'utente ha droppato (il menu l / F12 le elenca).
pub fn registry() -> (Vec<Lang>, Vec<String>) {
    registry_in(profile_dir().as_deref())
}

// ---------------------------------------------------------------------------
// Il lettore: un oggetto JSON piatto di stringhe, niente di piu'. Severo
// dove serve (cio' che non e' una stringa si rifiuta, la roba dopo l'oggetto
// pure) e tollerante dove il mondo lo e' (BOM, a-capo CRLF, caratteri non
// ASCII scritti raw o come escape \uXXXX — deve rileggere entrambe le
// grafie, perche' una e' quella dei file fatti a mano e l'altra e' quella
// che json.rs scrive).
// ---------------------------------------------------------------------------

/// Toglie il BOM che gli editor di Windows a volte mettono: il file della
/// lingua si deve poter salvare con qualunque editor ragionevole.
fn parse_flat(text: &str) -> Result<HashMap<String, String>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = P {
        s: text,
        i: 0,
        line: 1,
    };
    p.ws();
    p.expect('{')?;
    let mut dict = HashMap::new();
    loop {
        p.ws();
        if p.peek() == Some('}') {
            p.bump();
            break;
        }
        let key = p.string()?;
        p.ws();
        p.expect(':')?;
        p.ws();
        let val = p.string()?;
        dict.insert(key, val);
        p.ws();
        match p.bump() {
            Some(',') => {
                // la virgola finale non si perdona: la coppia deve esserci
                p.ws();
                if p.peek() == Some('}') {
                    return Err(p.err("virgola finale: la coppia dopo manca"));
                }
            }
            Some('}') => break,
            Some(c) => {
                return Err(p.err(&format!("atteso ',' o '}}', trovato '{}'", c)));
            }
            None => return Err(p.err("l'oggetto finisce prima di '}'")),
        }
    }
    p.ws();
    if p.i != p.s.len() {
        return Err(p.err("roba dopo la chiusura dell'oggetto"));
    }
    Ok(dict)
}

/// Il cursore sul testo: posizione (byte) e riga, per gli errori che
/// indicano dove il file smette di farsi capire.
struct P<'a> {
    s: &'a str,
    i: usize,
    line: usize,
}

impl<'a> P<'a> {
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    /// Gli spazi bianchi fra i pezzi (a-capo compreso, LF e CRLF).
    fn ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
            self.bump();
        }
    }

    fn err(&self, msg: &str) -> String {
        format!("riga {}: {}", self.line, msg)
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        match self.bump() {
            Some(x) if x == c => Ok(()),
            Some(x) => Err(self.err(&format!("atteso '{}', trovato '{}'", c, x))),
            None => Err(self.err(&format!("atteso '{}', il file finisce", c))),
        }
    }
    /// Una stringa JSON fra doppi apici, con tutti gli escape standard (le
    /// coppie surrogate \uXXXX\uXXXX incluse: servono per i caratteri fuori
    /// dal BMP, e il file lo scrive anche json.rs).
    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.err("stringa senza chiusura")),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let cp = self.hex4()?;
                        let c = if (0xd800..0xdc00).contains(&cp) {
                            // alta senza la bassa non e' un carattere: la
                            // coppia si completa o si sostituisce
                            if self.peek() == Some('\\') {
                                self.bump();
                                if self.peek() == Some('u') {
                                    self.bump();
                                    let lo = self.hex4()?;
                                    if (0xdc00..0xe000).contains(&lo) {
                                        let n = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00);
                                        char::from_u32(n).unwrap_or('\u{fffd}')
                                    } else {
                                        '\u{fffd}'
                                    }
                                } else {
                                    '\u{fffd}'
                                }
                            } else {
                                '\u{fffd}'
                            }
                        } else {
                            char::from_u32(cp).unwrap_or('\u{fffd}')
                        };
                        out.push(c);
                    }
                    Some(c) => {
                        return Err(self.err(&format!("escape sconosciuto '\\{}'", c)));
                    }
                    None => return Err(self.err("escape senza fine")),
                },
                Some(c) if (c as u32) < 0x20 => {
                    return Err(self.err("carattere di controllo dentro la stringa"));
                }
                Some(c) => out.push(c),
            }
        }
    }

    /// Le quattro cifre di \uXXXX.
    fn hex4(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self
                .bump()
                .ok_or_else(|| self.err("\\u vuole quattro cifre, il file finisce"))?;
            let d = c
                .to_digit(16)
                .ok_or_else(|| self.err(&format!("'{}' non e' esadecimale", c)))?;
            v = v * 16 + d;
        }
        Ok(v)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_oggetto_piatto_si_legge() {
        let d = parse_flat(r#"{"a": "uno", "b": "due"}"#).unwrap();
        assert_eq!(d.get("a").map(|s| s.as_str()), Some("uno"));
        assert_eq!(d.get("b").map(|s| s.as_str()), Some("due"));
    }

    #[test]
    fn il_bom_e_gli_a_capo_di_windows_si_tollerano() {
        let text = "\u{feff}{\r\n  \"a\": \"uno\",\r\n  \"b\": \"due\"\r\n}\r\n";
        let d = parse_flat(text).unwrap();
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn gli_escape_si_leggono_e_non_ascii_anche_raw() {
        // la stessa stringa nelle due grafie: raw UTF-8 e \uXXXX (quella
        // che scrive json.rs, ASCII puro) devono dare lo stesso risultato
        let raw = parse_flat("{\"k\": \"caffè 日\"}").unwrap();
        let esc = parse_flat("{\"k\": \"caff\\u00e8 \\u65e5\"}").unwrap();
        assert_eq!(raw.get("k"), esc.get("k"));
        // e gli escape classici pure
        let d = parse_flat("{\"k\": \"a\\\"b\\\\c\\nd\\te\"}").unwrap();
        assert_eq!(d.get("k").map(|s| s.as_str()), Some("a\"b\\c\nd\te"));
    }

    #[test]
    fn le_coppie_surrogate_si_compongono() {
        // U+1F5C2 (fatto di surrogate) diventa se stesso, non due sostituti
        let d = parse_flat("{\"k\": \"\\ud83d\\uddc2\"}").unwrap();
        assert_eq!(d.get("k").map(|s| s.as_str()), Some("\u{1f5c2}"));
    }

    #[test]
    fn il_malformato_si_rifiuta_e_dice_dove() {
        assert!(parse_flat("{").is_err()); // non finisce
        assert!(parse_flat("{\"a\" \"b\"}").is_err()); // manca ':'
        assert!(parse_flat("{\"a\": 1}").is_err()); // non e' una stringa
        assert!(parse_flat("{\"a\": \"b\",}").is_err()); // virgola finale
        assert!(parse_flat("{\"a\": \"b\"} xy").is_err()); // roba dopo
        assert!(parse_flat("[\"a\"]").is_err()); // non e' un oggetto
        let e = parse_flat("{\"a\": \"b\"\n\"c\": \"d\"}").unwrap_err();
        assert!(e.contains("riga 2"), "dice la riga: {}", e);
    }

    #[test]
    fn round_trip_con_lo_scrittore_di_json() {
        // il progetto possiede scrittura e lettura e le si prova in coppia:
        // json.rs scrive il dizionario col suo escape (ASCII puro), lang.rs
        // lo rilegge — identico
        let dict = [
            ("vuota", ""),
            ("virgolette", "un \" nel mezzo"),
            ("backslash", "C:\\Users\\prova"),
            ("a_capo", "prima\ndopo"),
            ("tab", "a\tb"),
            ("non_ascii", "caffè, 日本語, emoji \u{1f5c2}"),
            ("chiave strana", "valore"),
        ];
        let mut text = String::from("{");
        for (i, (k, v)) in dict.iter().enumerate() {
            if i > 0 {
                text.push(',');
            }
            text.push_str(&format!(
                "\n  \"{}\": \"{}\"",
                crate::json::esc(k),
                crate::json::esc(v)
            ));
        }
        text.push_str("\n}");
        let back = parse_flat(&text).unwrap();
        for (k, v) in &dict {
            assert_eq!(back.get(*k).map(|s| s.as_str()), Some(*v), "chiave {}", k);
        }
    }

    #[test]
    fn il_catalogo_cade_in_inglese_e_poi_nella_chiave() {
        let it = Lang::from_json_str("it", "{\"_name\": \"Italiano\", \"footer.f1\": \"Aiuto\"}").unwrap();
        assert_eq!(it.name, "Italiano");
        assert!(!it.ai);
        let cat = Catalog::with(it);
        assert_eq!(cat.t("footer.f1"), "Aiuto"); // la attiva
        assert_eq!(cat.t("footer.f2"), "Save"); // l'inglese incorporato
        // la chiave che manca dappertutto stampa se stessa: un gap si vede
        assert_eq!(cat.t("chiave.che.non.esiste"), "chiave.che.non.esiste");
    }

    #[test]
    fn i_segnaposto_si_riempiono_senza_panic() {
        let cat = Catalog::english();
        assert_eq!(cat.tf("x{}y{}z", &["1", "2"]), "x1y2z");
        assert_eq!(cat.tf("x{}y", &["1"]), "x1y");
        assert_eq!(cat.tf("x{}y", &[]), "xy"); // manca: resta vuoto
        assert_eq!(cat.tf("xy", &["1"]), "xy"); // nessun segnaposto
        assert_eq!(cat.tf("x{}y", &["1", "2"]), "x1y"); // uno di piu': si ignora
    }

    #[test]
    fn il_meta_ai_si_dichiara() {
        let l = Lang::from_json_str(
            "de",
            "{\"_name\": \"Deutsch\", \"_ai\": \"KI-generierte Uebersetzung\", \"k\": \"v\"}",
        )
        .unwrap();
        assert!(l.ai);
        assert_eq!(l.get("_ai"), Some("KI-generierte Uebersetzung"));
        // il disclaimer e' meta, non vocabolario: la parita' non lo guarda
        assert_eq!(l.keys(), vec!["k"]);
    }

    #[test]
    fn la_scoperta_salta_gli_illeggibili_e_non_json() {
        let dir = std::env::temp_dir().join(format!("mfd-lang-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("xx.json"), "{\"_name\": \"Xx\", \"k\": \"v\"}").unwrap();
        std::fs::write(dir.join("rotto.json"), "{non json").unwrap();
        std::fs::write(dir.join("annoti.txt"), "non e' una lingua").unwrap();
        let (langs, unreadable) = registry_in(Some(&dir));
        assert_eq!(langs.len(), 2); // en incorporato + xx
        assert_eq!(langs[1].code, "xx");
        assert_eq!(langs[1].name, "Xx");
        assert_eq!(unreadable, vec!["rotto.json".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn il_profilo_sopravanza_l_incorporato() {
        // un en.json nel profilo prende il posto dell'inglese di fabbrica:
        // il menu parte da quello scoperto
        let dir = std::env::temp_dir().join(format!("mfd-lang-override-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("en.json"), "{\"_name\": \"English (custom)\"}").unwrap();
        let (langs, unreadable) = registry_in(Some(&dir));
        assert!(unreadable.is_empty());
        assert_eq!(langs[0].code, "en");
        assert_eq!(langs[0].name, "English (custom)");
        // e le chiavi dell'inglese custom cadono nell'inglese incorporato
        let cat = Catalog::with(langs[0].clone());
        assert_eq!(cat.t("footer.f1"), "Help"); // dal fallback
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn la_parita_delle_chiavi_e_il_gate_delle_lingue() {
        // tutte le lingue di lang/ hanno lo stesso vocabolario di en.json:
        // e' il gate automatico delle PR — la completezza non si
        // revisiona a mano. Le prime sette: en, it, es, pt-BR, de, ja,
        // zh-CN (le traduzioni AI di partenza; _ai su tutte tranne le
        // prime due: l'inglese e' la referenza, l'italiano e' del
        // manutentore)
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("lang");
        let en = Lang::english();
        let mut viste = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let l = Lang::from_file(&path)
                .unwrap_or_else(|e| panic!("{} non si legge: {}", path.display(), e));
            viste += 1;
            assert_eq!(
                l.keys(),
                en.keys(),
                "{}: le chiavi devono essere quelle di en.json",
                path.display()
            );
            assert!(
                !l.get("_name").unwrap_or("").trim().is_empty(),
                "{}: _name manca",
                path.display()
            );
        }
        assert!(viste >= 7, "le sette lingue di partenza (trovate: {})", viste);
    }
}