use kakasi;

pub fn transliterate(text: &str) -> Option<String> {
    // 1. Coreano (Hangul)
    if text.chars().any(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c)) {
        return Some(transliterate_korean(text));
    }

    // 2. Japonés (Hiragana, Katakana, Kanji)
    if text.chars().any(|c| matches!(c, '\u{3041}'..='\u{3096}' | '\u{30A1}'..='\u{30FA}' | '\u{4E00}'..='\u{9FFF}')) {
        let res = kakasi::convert(text);
        let romaji = res.romaji.trim();
        if !romaji.is_empty() {
            return Some(romaji.to_string());
        }
    }

    None
}

fn transliterate_korean(text: &str) -> String {
    const CHOSUNG: [&str; 19] = ["g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t", "p", "h"];
    const JUNGSUNG: [&str; 21] = ["a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo", "we", "wi", "yu", "eu", "ui", "i"];
    const JONGSUNG: [&str; 28] = ["", "k", "k", "ks", "n", "nj", "nh", "t", "l", "lg", "lm", "lb", "ls", "lt", "lp", "lh", "m", "p", "ps", "t", "t", "ng", "t", "t", "k", "t", "p", "h"];

    let mut result = String::new();
    for c in text.chars() {
        if ('\u{AC00}'..='\u{D7A3}').contains(&c) {
            let index = (c as u32 - 0xAC00) as usize;
            let chosung_idx = index / (21 * 28);
            let jungsung_idx = (index % (21 * 28)) / 28;
            let jongsung_idx = index % 28;

            if let Some(chosung) = CHOSUNG.get(chosung_idx) {
                result.push_str(chosung);
            }
            if let Some(jungsung) = JUNGSUNG.get(jungsung_idx) {
                result.push_str(jungsung);
            }
            result.push_str(JONGSUNG[jongsung_idx]);
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_korean_hangul() {
        assert_eq!(transliterate("안녕하세요"), Some("annyeonghaseyo".to_string()));
    }

    #[test]
    fn test_japanese_kana() {
        assert_eq!(transliterate("あした"), Some("ashita".to_string()));
    }

    #[test]
    fn test_japanese_kanji_and_kana() {
        let res = transliterate("甘くぬかるんだ眼差し").unwrap();
        assert_eq!(res, "amaku nukarunda manazashi");
    }

    #[test]
    fn test_ascii_returns_none() {
        assert_eq!(transliterate("Hello, world!"), None);
    }
}