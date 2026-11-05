use reqwest::Client;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
    pub phonetic: Option<String>,
}

impl LyricLine {
    pub fn new(time_ms: u64, text: impl Into<String>) -> Self {
        let text = text.into();
        let phonetic = crate::transliteration::transliterate(&text);
        Self { time_ms, text, phonetic }
    }
}

pub fn parse_lrc(raw: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for (idx, raw_line) in raw.lines().enumerate() {
        let s = raw_line.trim();
        if s.is_empty() { continue; }
        if let Some(close) = s.find(']') {
            if s.starts_with('[') {
                let tag = &s[1..close];
                let text = s[close + 1..].trim();
                if let Some((m_str, s_str)) = tag.split_once(':') {
                    if let (Ok(mins), Ok(secs)) = (m_str.parse::<u64>(), s_str.parse::<f64>()) {
                        let time_ms = mins * 60_000 + (secs * 1000.0) as u64;
                        let val = if text.is_empty() { "•••" } else { text };
                        if val != "•••" || lines.last().map(|l: &LyricLine| l.text != "•••").unwrap_or(true) {
                            lines.push(LyricLine::new(time_ms, val));
                        }
                        continue;
                    }
                }
            }
        }
        lines.push(LyricLine::new(idx as u64 * 4000, s));
    }
    if lines.first().map(|l| l.time_ms >= 3000).unwrap_or(false) {
        lines.insert(0, LyricLine::new(0, "•••"));
    }
    lines
}

pub async fn fetch_lrclib_lyrics(
    http: &Client,
    title: &str,
    artist: &str,
) -> Vec<LyricLine> {
    let clean_title = title.split(['(', '[']).next().unwrap_or(title)
        .split(" - ").next().unwrap_or(title).trim();

    let url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        urlencoding::encode(clean_title),
        urlencoding::encode(artist)
    );

    if let Ok(resp) = http.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(synced) = json.get("syncedLyrics").and_then(|v| v.as_str()) {
                    let parsed = parse_lrc(synced);
                    if !parsed.is_empty() {
                        return parsed;
                    }
                }
                if let Some(plain) = json.get("plainLyrics").and_then(|v| v.as_str()) {
                    let parsed = parse_lrc(plain);
                    if !parsed.is_empty() {
                        return parsed;
                    }
                }
            }
        }
    }
    Vec::new()
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for b in s.bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(b as char);
                }
                b' ' => encoded.push_str("%20"),
                _ => encoded.push_str(&format!("%{:02X}", b)),
            }
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc() {
        let lrc = "[00:10.50] Primera línea\n[00:15.00] Segunda línea";
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 3); // Includes leading •••
        assert_eq!(parsed[1].text, "Primera línea");
        assert_eq!(parsed[1].time_ms, 10500);
    }
}
