use std::path::PathBuf;
use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserType {
    Chromium(PathBuf),
    Firefox(PathBuf),
    WebKit(PathBuf),
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let filename = if cfg!(windows) && !name.ends_with(".exe") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let full = dir.join(&filename);
            if full.is_file() { Some(full) } else { None }
        })
    })
}

fn find_candidate(names: &[&str]) -> Option<PathBuf> {
    for name in names {
        if let Some(p) = find_in_path(name) {
            return Some(p);
        }
    }
    None
}

pub fn discover_browser() -> Option<BrowserType> {
    if let Ok(val) = env::var("SPORE_BROWSER_BIN").or_else(|_| env::var("BROWSER")) {
        let low = val.to_lowercase();
        if low.contains("safari") || low.contains("webkit") || low.contains("orion") || low.contains("epiphany") {
            return Some(BrowserType::WebKit(PathBuf::from(val)));
        }
        if low.contains("firefox") || low.contains("librewolf") || low.contains("zen")
            || low.contains("waterfox") || low.contains("floorp")
        {
            return Some(BrowserType::Firefox(PathBuf::from(val)));
        }
        return Some(BrowserType::Chromium(PathBuf::from(val)));
    }

    let chromium_names = [
        "google-chrome", "chrome", "chromium", "brave-browser", "brave",
        "msedge", "microsoft-edge", "opera", "vivaldi", "arc"
    ];
    if let Some(path) = find_candidate(&chromium_names) {
        return Some(BrowserType::Chromium(path));
    }

    let firefox_names = [
        "firefox", "librewolf", "zen", "zen-browser", "waterfox", "floorp", "mullvad-browser"
    ];
    if let Some(path) = find_candidate(&firefox_names) {
        return Some(BrowserType::Firefox(path));
    }

    #[cfg(target_os = "windows")]
    {
        for base in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            if let Ok(dir) = env::var(base) {
                let pb = PathBuf::from(dir);
                let candidates = [
                    pb.join(r"Microsoft\Edge\Application\msedge.exe"),
                    pb.join(r"Google\Chrome\Application\chrome.exe"),
                    pb.join(r"BraveSoftware\Brave-Browser\Application\brave.exe"),
                    pb.join(r"Mozilla Firefox\firefox.exe"),
                ];
                for p in candidates {
                    if p.is_file() {
                        if p.to_string_lossy().to_lowercase().contains("firefox") {
                            return Some(BrowserType::Firefox(p));
                        }
                        return Some(BrowserType::Chromium(p));
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        for p in ["Google Chrome.app/Contents/MacOS/Google Chrome", "Brave Browser.app/Contents/MacOS/Brave Browser",
                  "Microsoft Edge.app/Contents/MacOS/Microsoft Edge", "Arc.app/Contents/MacOS/Arc"] {
            let pb = PathBuf::from("/Applications").join(p);
            if pb.is_file() { return Some(BrowserType::Chromium(pb)); }
        }
        let mac_webkit = ["/Applications/Safari.app/Contents/MacOS/Safari", "/Applications/Orion.app/Contents/MacOS/Orion"];
        if let Some(p) = mac_webkit.iter().map(PathBuf::from).find(|p| p.is_file()) {
            return Some(BrowserType::WebKit(p));
        }
    }

    #[cfg(target_os = "linux")]
    if let Some(p) = find_in_path("epiphany") {
        return Some(BrowserType::WebKit(p));
    }

    None
}

pub fn find_gecko_cookie_databases() -> Vec<PathBuf> {
    let mut base_dirs = Vec::new();

    #[cfg(target_os = "windows")]
    if let Ok(appdata) = env::var("APPDATA") {
        let p = PathBuf::from(appdata);
        for sub in [r"Mozilla\Firefox", "LibreWolf", "Zen", "Waterfox", "Floorp"] {
            base_dirs.push(p.join(sub));
        }
    }

    #[cfg(not(target_os = "windows"))]
    if let Ok(home) = env::var("HOME") {
        let h = PathBuf::from(home);
        for sub in [".mozilla/firefox", ".librewolf", ".zen", ".waterfox", ".floorp",
            "Library/Application Support/Firefox", "Library/Application Support/LibreWolf", "Library/Application Support/Zen",
            ".var/app/org.mozilla.firefox/.mozilla/firefox", ".var/app/io.gitlab.librewolf-community/.librewolf", ".var/app/app.zen_browser.zen/.zen"] {
            base_dirs.push(h.join(sub));
        }
    }

    let mut dbs = Vec::new();
    for base in base_dirs {
        if !base.is_dir() { continue; }
        let profiles_dir = if base.join("Profiles").is_dir() { base.join("Profiles") } else { base };
        if let Ok(entries) = std::fs::read_dir(profiles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let cookie_file = path.join("cookies.sqlite");
                    if cookie_file.is_file() {
                        dbs.push(cookie_file);
                    }
                }
            }
        }
    }

    dbs.sort_by_key(|p| {
        std::fs::metadata(p).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    dbs.reverse();
    dbs
}
