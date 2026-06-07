use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;
use zeroize::Zeroize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenMode {
    Chars,
    Passphrase,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenOptions {
    pub mode: GenMode,
    pub length: usize,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub number: bool,
}

const AMBIGUOUS: &[u8] = b"0O1lI";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.?/";

// EFF Large Wordlist (7776 words). CC-BY-3.0-US. https://www.eff.org/dice
static WORDLIST: &str = include_str!("eff_large_wordlist.txt");

pub fn generate(opts: &GenOptions) -> String {
    match opts.mode {
        GenMode::Chars => gen_chars(opts),
        GenMode::Passphrase => gen_passphrase(opts),
    }
}

fn gen_chars(opts: &GenOptions) -> String {
    let length = opts.length.clamp(8, 128);
    let mut alphabet: Vec<u8> =
        (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'0'..=b'9').collect();
    if opts.symbols {
        alphabet.extend_from_slice(SYMBOLS);
    }
    if opts.avoid_ambiguous {
        alphabet.retain(|c| !AMBIGUOUS.contains(c));
    }
    let mut rng = rand::thread_rng();
    let pw: String = (0..length)
        .map(|_| *alphabet.choose(&mut rng).expect("alphabet non-empty") as char)
        .collect();
    alphabet.zeroize();
    pw
}

fn gen_passphrase(opts: &GenOptions) -> String {
    let count = opts.words.clamp(3, 12);
    let list: Vec<&str> = WORDLIST.lines().collect();
    let sep = if opts.separator.is_empty() { "-" } else { opts.separator.as_str() };
    let mut rng = rand::thread_rng();
    let mut parts: Vec<String> = (0..count)
        .map(|_| {
            let w = *list.choose(&mut rng).expect("wordlist non-empty");
            if opts.capitalize {
                capitalize(w)
            } else {
                w.to_string()
            }
        })
        .collect();
    let mut pass = parts.join(sep);
    if opts.number {
        pass.push(char::from(b'0' + rng.gen_range(2..=9)));
    }
    parts.zeroize();
    pass
}

fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars_opts() -> GenOptions {
        GenOptions {
            mode: GenMode::Chars,
            length: 20,
            symbols: true,
            avoid_ambiguous: false,
            words: 5,
            separator: "-".into(),
            capitalize: true,
            number: false,
        }
    }

    fn phrase_opts() -> GenOptions {
        GenOptions { mode: GenMode::Passphrase, ..chars_opts() }
    }

    #[test]
    fn chars_respects_length_and_clamps() {
        assert_eq!(generate(&GenOptions { length: 20, ..chars_opts() }).chars().count(), 20);
        assert_eq!(generate(&GenOptions { length: 4, ..chars_opts() }).chars().count(), 8);
        assert_eq!(generate(&GenOptions { length: 999, ..chars_opts() }).chars().count(), 128);
    }

    #[test]
    fn chars_without_symbols_is_alphanumeric() {
        let pw = generate(&GenOptions { symbols: false, ..chars_opts() });
        assert!(pw.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn chars_avoid_ambiguous_excludes_confusables() {
        let pw = generate(&GenOptions { avoid_ambiguous: true, length: 128, ..chars_opts() });
        assert!(!pw.contains(['0', 'O', '1', 'l', 'I']));
    }

    #[test]
    fn passphrase_has_word_count() {
        let pw = generate(&GenOptions { words: 5, number: false, ..phrase_opts() });
        assert_eq!(pw.split('-').count(), 5);
    }

    #[test]
    fn passphrase_capitalizes_each_word() {
        let pw = generate(&GenOptions { words: 4, number: false, capitalize: true, ..phrase_opts() });
        assert!(pw.split('-').all(|w| w.chars().next().is_some_and(|c| c.is_uppercase())));
    }

    #[test]
    fn passphrase_number_appends_a_safe_digit() {
        let pw = generate(&GenOptions { number: true, ..phrase_opts() });
        let last = pw.chars().last().unwrap();
        assert!(('2'..='9').contains(&last));
    }

    #[test]
    fn wordlist_is_the_eff_large_list() {
        assert_eq!(WORDLIST.lines().count(), 7776);
        assert_eq!(WORDLIST.lines().next().unwrap(), "abacus");
        assert_eq!(WORDLIST.lines().last().unwrap(), "zoom");
    }

    #[test]
    fn two_generations_differ() {
        assert_ne!(generate(&chars_opts()), generate(&chars_opts()));
    }
}
