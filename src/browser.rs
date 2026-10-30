use std::path::PathBuf;
use std::process::Command;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{
    actions, div, font, px, rgb, rgba, App, Bounds, Context, Div, FocusHandle, FontWeight,
    KeyBinding, KeyDownEvent, Point, Render, SharedString, Size, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowOptions,
};

use crate::components::Theme;
use crate::icons::{self, SfSymbol};

actions!(
    alpenglow_browser,
    [
        BrowserNewTab,
        BrowserCloseTab,
        BrowserReload,
        BrowserFocusUrl,
        BrowserToggleCmdBar,
        BrowserGoBack,
        BrowserGoForward,
        BrowserZoomIn,
        BrowserZoomOut,
        BrowserResetZoom,
        BrowserToggleBookmark,
        BrowserCopyUrl,
        BrowserNextTab,
        BrowserPrevTab,
        BrowserSelectTab1,
        BrowserSelectTab2,
        BrowserSelectTab3,
        BrowserSelectTab4,
        BrowserSelectTab5,
        BrowserSelectTab6,
        BrowserSelectTab7,
        BrowserSelectTab8,
        BrowserSelectTab9,
        BrowserEscape,
    ]
);

pub fn bind_browser_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-t", BrowserNewTab, Some("alpenglow_browser")),
        KeyBinding::new("cmd-w", BrowserCloseTab, Some("alpenglow_browser")),
        KeyBinding::new("cmd-r", BrowserReload, Some("alpenglow_browser")),
        KeyBinding::new("cmd-l", BrowserFocusUrl, Some("alpenglow_browser")),
        KeyBinding::new("cmd-k", BrowserToggleCmdBar, Some("alpenglow_browser")),
        KeyBinding::new("cmd-space", BrowserToggleCmdBar, Some("alpenglow_browser")),
        KeyBinding::new("cmd-[", BrowserGoBack, Some("alpenglow_browser")),
        KeyBinding::new("cmd-left", BrowserGoBack, Some("alpenglow_browser")),
        KeyBinding::new("cmd-]", BrowserGoForward, Some("alpenglow_browser")),
        KeyBinding::new("cmd-right", BrowserGoForward, Some("alpenglow_browser")),
        KeyBinding::new("cmd-=", BrowserZoomIn, Some("alpenglow_browser")),
        KeyBinding::new("cmd-+", BrowserZoomIn, Some("alpenglow_browser")),
        KeyBinding::new("cmd--", BrowserZoomOut, Some("alpenglow_browser")),
        KeyBinding::new("cmd-0", BrowserResetZoom, Some("alpenglow_browser")),
        KeyBinding::new("cmd-d", BrowserToggleBookmark, Some("alpenglow_browser")),
        KeyBinding::new("cmd-shift-c", BrowserCopyUrl, Some("alpenglow_browser")),
        KeyBinding::new("ctrl-tab", BrowserNextTab, Some("alpenglow_browser")),
        KeyBinding::new("ctrl-shift-tab", BrowserPrevTab, Some("alpenglow_browser")),
        KeyBinding::new("cmd-1", BrowserSelectTab1, Some("alpenglow_browser")),
        KeyBinding::new("cmd-2", BrowserSelectTab2, Some("alpenglow_browser")),
        KeyBinding::new("cmd-3", BrowserSelectTab3, Some("alpenglow_browser")),
        KeyBinding::new("cmd-4", BrowserSelectTab4, Some("alpenglow_browser")),
        KeyBinding::new("cmd-5", BrowserSelectTab5, Some("alpenglow_browser")),
        KeyBinding::new("cmd-6", BrowserSelectTab6, Some("alpenglow_browser")),
        KeyBinding::new("cmd-7", BrowserSelectTab7, Some("alpenglow_browser")),
        KeyBinding::new("cmd-8", BrowserSelectTab8, Some("alpenglow_browser")),
        KeyBinding::new("cmd-9", BrowserSelectTab9, Some("alpenglow_browser")),
        KeyBinding::new("escape", BrowserEscape, Some("alpenglow_browser")),
    ]);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserEngineStatus {
    pub engine_name: String,
    pub installed: bool,
    pub package_manager: &'static str,
    pub install_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalBrowserPage {
    pub url: String,
    pub title: String,
    pub summary_blocks: Vec<String>,
    pub links: Vec<(String, String)>,
    pub webapp_installed: bool,
    pub status_banner: Option<String>,
}

/// Detects the Linux package manager and whether a native WebKit/system browser engine is available.
pub fn detect_browser_engine() -> BrowserEngineStatus {
    let pkg_mgr = if PathBuf::from("/usr/bin/pacman").exists() {
        "pacman"
    } else if PathBuf::from("/usr/bin/apt-get").exists() {
        "apt"
    } else if PathBuf::from("/usr/bin/dnf").exists() {
        "dnf"
    } else if PathBuf::from("/sbin/apk").exists() {
        "apk"
    } else if PathBuf::from("/usr/bin/flatpak").exists() {
        "flatpak"
    } else {
        "system"
    };

    let install_command = match pkg_mgr {
        "pacman" => "pkexec pacman -S --noconfirm webkit2gtk-4.1 epiphany".to_string(),
        "apt" => "pkexec apt-get install -y libwebkit2gtk-4.1-0 epiphany-browser".to_string(),
        "dnf" => "pkexec dnf install -y webkit2gtk4.1 epiphany".to_string(),
        "apk" => "pkexec apk add webkit2gtk epiphany".to_string(),
        "flatpak" => "flatpak install -y flathub org.gnome.Epiphany".to_string(),
        _ => "xdg-settings get default-web-browser".to_string(),
    };

    let webkit_present = [
        "/usr/lib/libwebkit2gtk-4.1.so",
        "/usr/lib64/libwebkit2gtk-4.1.so",
        "/usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.1.so.0",
        "/System/Library/Frameworks/WebKit.framework",
    ]
    .iter()
    .any(|p| PathBuf::from(p).exists());

    let engine_name = if webkit_present {
        "WebKit Native".to_string()
    } else {
        "Built-in Reader".to_string()
    };

    BrowserEngineStatus {
        engine_name,
        installed: webkit_present,
        package_manager: pkg_mgr,
        install_command,
    }
}

pub fn slugify_domain(url: &str) -> String {
    let stripped = url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let host = stripped.split('/').next().unwrap_or("webapp");
    let slug: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let clean = slug.trim_matches('-').to_string();
    if clean.is_empty() {
        "webapp".to_string()
    } else {
        clean
    }
}

pub fn webapp_desktop_path(url: &str) -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    let slug = slugify_domain(url);
    base.join("applications")
        .join(format!("alpenglow-webapp-{slug}.desktop"))
}

/// Installs the given URL as a native Freedesktop `.desktop` web application in `~/.local/share/applications/`.
pub fn install_web_app(title: &str, url: &str) -> std::io::Result<PathBuf> {
    let path = webapp_desktop_path(url);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let clean_title = if title.trim().is_empty() {
        slugify_domain(url)
    } else {
        title.trim().to_string()
    };
    let desktop_entry = format!(
        "[Desktop Entry]\n\
         Version=1.0\n\
         Type=Application\n\
         Name={clean_title}\n\
         Comment=Web Application ({url})\n\
         Exec=alpenglowed open {url}\n\
         Icon=web-browser-symbolic\n\
         Terminal=false\n\
         Categories=Network;WebBrowser;\n\
         StartupWMClass=alpenglow-webapp-{}\n",
        slugify_domain(url)
    );
    std::fs::write(&path, desktop_entry)?;
    Ok(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserTabDisplay {
    pub title: String,
    pub url: String,
    pub is_active: bool,
}

fn extract_page_links(url: &str, html: &str) -> Vec<(String, String)> {
    let mut links = Vec::new();
    let lower = html.to_lowercase();
    let base_host = if let Some(slash_idx) = url.get(8..).and_then(|r| r.find('/')) {
        &url[..8 + slash_idx]
    } else {
        url
    };
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find("<a ") {
        let tag_start = cursor + rel;
        let Some(tag_end_rel) = lower[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_rel;
        let attrs = &html[tag_start..tag_end];
        let Some(close_rel) = lower[tag_end..].find("</a>") else {
            break;
        };
        let text = strip_html_tags(&html[tag_end..tag_end + close_rel]);
        cursor = tag_end + close_rel + 4;

        if let Some(href_pos) = attrs.find("href=\"").or_else(|| attrs.find("href='")) {
            let quote = attrs.as_bytes()[href_pos + 5] as char;
            let rest = &attrs[href_pos + 6..];
            if let Some(end_quote) = rest.find(quote) {
                let href = &rest[..end_quote];
                let full_url = if href.starts_with("http://") || href.starts_with("https://") {
                    href.to_string()
                } else if href.starts_with('/') {
                    format!("{base_host}{href}")
                } else {
                    continue;
                };
                let clean_text = collapse_whitespace(&text);
                if clean_text.len() >= 3
                    && clean_text.len() <= 36
                    && !links.iter().any(|(t, _)| t == &clean_text)
                {
                    links.push((clean_text, full_url));
                    if links.len() >= 6 {
                        break;
                    }
                }
            }
        }
    }
    if links.is_empty() {
        default_quicklinks()
    } else {
        links
    }
}

/// Strips <script>...</script> and <style>...</style> blocks to prevent JS/CSS leaking into readable reader text.
fn strip_script_and_style(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let lower = html.to_lowercase();
    let mut cursor = 0;
    while cursor < html.len() {
        let script_pos = lower[cursor..].find("<script");
        let style_pos = lower[cursor..].find("<style");
        let (tag, next_rel) = match (script_pos, style_pos) {
            (Some(s), Some(st)) => {
                if s < st {
                    ("script", s)
                } else {
                    ("style", st)
                }
            }
            (Some(s), None) => ("script", s),
            (None, Some(st)) => ("style", st),
            (None, None) => {
                out.push_str(&html[cursor..]);
                break;
            }
        };
        let tag_start = cursor + next_rel;
        out.push_str(&html[cursor..tag_start]);
        let close_tag = format!("</{tag}>");
        if let Some(close_rel) = lower[tag_start..].find(&close_tag) {
            cursor = tag_start + close_rel + close_tag.len();
        } else {
            break;
        }
    }
    out
}

/// Extracts readable text blocks and title from raw HTML.
pub fn parse_html_to_page(url: &str, html: &str) -> InternalBrowserPage {
    let cleaned_html = strip_script_and_style(html);
    let title = extract_tag_content(&cleaned_html, "title").unwrap_or_else(|| domain_title(url));
    let mut blocks = Vec::new();
    for tag in ["h1", "h2", "h3", "p", "li"] {
        for extracted in extract_all_tag_contents(&cleaned_html, tag) {
            let clean = collapse_whitespace(&extracted);
            if clean.len() >= 12
                && blocks.len() < 6
                && !blocks.contains(&clean)
                && !clean.contains("function(")
                && !clean.contains("var ")
                && !clean.contains("window.addEventListener")
            {
                blocks.push(clean);
            }
        }
    }
    if blocks.is_empty() {
        blocks = curated_blocks_for_url(url);
    }
    let links = extract_page_links(url, html);
    let webapp_installed = webapp_desktop_path(url).exists();
    InternalBrowserPage {
        url: url.to_string(),
        title,
        summary_blocks: blocks,
        links,
        webapp_installed,
        status_banner: None,
    }
}

pub fn load_internal_page(url: &str) -> InternalBrowserPage {
    let normalized = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url.trim())
    };

    // Live fetch via curl with fast 3-second timeout
    if let Ok(out) = Command::new("curl")
        .args([
            "-sL",
            "--max-time",
            "3",
            "-A",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            &normalized,
        ])
        .output()
    {
        if out.status.success() && !out.stdout.is_empty() {
            let html = String::from_utf8_lossy(&out.stdout);
            let page = parse_html_to_page(&normalized, &html);
            if !page.summary_blocks.is_empty() {
                return page;
            }
        }
    }

    InternalBrowserPage {
        url: normalized.clone(),
        title: domain_title(&normalized),
        summary_blocks: curated_blocks_for_url(&normalized),
        links: default_quicklinks(),
        webapp_installed: webapp_desktop_path(&normalized).exists(),
        status_banner: None,
    }
}

fn domain_title(url: &str) -> String {
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let host = stripped.split('/').next().unwrap_or(url);
    match host {
        "github.com" => "GitHub · Software Development & Repositories".to_string(),
        "docs.rs" => "Docs.rs · Rust Crate Documentation".to_string(),
        "crates.io" => "crates.io · Rust Package Registry".to_string(),
        "wiki.archlinux.org" => "ArchWiki · Linux Documentation".to_string(),
        "duckduckgo.com" => "DuckDuckGo · Private Web Search".to_string(),
        "en.wikipedia.org" => "Wikipedia · The Free Encyclopedia".to_string(),
        _ => host.to_string(),
    }
}

fn curated_blocks_for_url(url: &str) -> Vec<String> {
    if url.contains("github.com") {
        vec![
            "Explore repositories, pull requests, issues, and releases directly from the internal browser.".to_string(),
            "Tip: Type !gh <query> in the search bar to jump straight to matching repositories.".to_string(),
            "Click 'Pin App' above to pin this site as a native Linux desktop launcher entry.".to_string(),
        ]
    } else if url.contains("docs.rs") || url.contains("crates.io") {
        vec![
            "Search Rust standard library and crate API documentation with instant keyboard navigation.".to_string(),
            "Tip: Use !rs <crate> or !crates <name> from the main bar to open crate docs directly.".to_string(),
        ]
    } else if url.contains("archlinux.org") {
        vec![
            "Wayland compositor, DRM/KMS seat configuration, XDG desktop portal, and package guides.".to_string(),
            "Install WebKit2GTK or pin ArchWiki as a standalone desktop web app with one click.".to_string(),
        ]
    } else {
        vec![
            format!("Viewing {url} in the built-in reader view."),
            "Use 'Pin App' to create a native ~/.local/share/applications/*.desktop shortcut, or 'Open' to launch in default browser.".to_string(),
        ]
    }
}

fn default_quicklinks() -> Vec<(String, String)> {
    vec![
        ("GitHub".to_string(), "https://github.com".to_string()),
        ("Rust Docs".to_string(), "https://docs.rs".to_string()),
        (
            "Arch Wiki".to_string(),
            "https://wiki.archlinux.org".to_string(),
        ),
        (
            "DuckDuckGo".to_string(),
            "https://duckduckgo.com".to_string(),
        ),
    ]
}

fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}>");
    let start_tag = lower.find(&open_pat)?;
    let gt = lower[start_tag..].find('>')? + start_tag + 1;
    let end_tag = lower[gt..].find(&close_pat)? + gt;
    Some(strip_html_tags(&html[gt..end_tag]))
}

fn extract_all_tag_contents(html: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let lower = html.to_lowercase();
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}>");
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find(&open_pat) {
        let start_tag = cursor + rel;
        let Some(gt_rel) = lower[start_tag..].find('>') else {
            break;
        };
        let gt = start_tag + gt_rel + 1;
        let Some(end_rel) = lower[gt..].find(&close_pat) else {
            break;
        };
        let end_tag = gt + end_rel;
        let text = strip_html_tags(&html[gt..end_tag]);
        if !text.is_empty() {
            results.push(text);
        }
        cursor = end_tag + close_pat.len();
        if results.len() >= 8 {
            break;
        }
    }
    results
}

fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(target_os = "macos")]
pub mod macos_webview {
    use std::ffi::{c_char, c_void, CString};
    use std::sync::Mutex;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct CGSize {
        pub width: f64,
        pub height: f64,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct CGRect {
        pub origin: CGPoint,
        pub size: CGSize,
    }

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
    }

    #[link(name = "WebKit", kind = "framework")]
    unsafe extern "C" {}

    pub struct NativeWebView {
        pub raw: *mut c_void,
        pub current_url: String,
    }

    unsafe impl Send for NativeWebView {}
    unsafe impl Sync for NativeWebView {}

    static GLOBAL_WEBVIEW: Mutex<Option<NativeWebView>> = Mutex::new(None);

    pub fn ensure_regular_activation_policy() {
        unsafe {
            let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
            if ns_app_cls.is_null() {
                return;
            }
            let shared_app: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let app = shared_app(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
            if app.is_null() {
                return;
            }
            // NSApplicationActivationPolicyRegular = 0
            let set_policy: extern "C" fn(*mut c_void, *mut c_void, isize) -> bool =
                std::mem::transmute(objc_msgSend as *const ());
            set_policy(app, sel_registerName(c"setActivationPolicy:".as_ptr()), 0);
        }
    }

    pub fn attach_or_navigate(url: &str, x: f64, y: f64, width: f64, height: f64) {
        unsafe {
            let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
            if ns_app_cls.is_null() {
                return;
            }
            let shared_app: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let app = shared_app(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
            if app.is_null() {
                return;
            }

            // Target the Browser window if available, otherwise keyWindow
            let mut win: *mut c_void = std::ptr::null_mut();
            let windows_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let windows = windows_fn(app, sel_registerName(c"windows".as_ptr()));
            if !windows.is_null() {
                let count_fn: extern "C" fn(*mut c_void, *mut c_void) -> usize =
                    std::mem::transmute(objc_msgSend as *const ());
                let count = count_fn(windows, sel_registerName(c"count".as_ptr()));
                let obj_at_fn: extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let title_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let utf8_fn: extern "C" fn(*mut c_void, *mut c_void) -> *const c_char =
                    std::mem::transmute(objc_msgSend as *const ());

                for i in 0..count {
                    let w = obj_at_fn(windows, sel_registerName(c"objectAtIndex:".as_ptr()), i);
                    if !w.is_null() {
                        let t_obj = title_fn(w, sel_registerName(c"title".as_ptr()));
                        if !t_obj.is_null() {
                            let utf8 = utf8_fn(t_obj, sel_registerName(c"UTF8String".as_ptr()));
                            if !utf8.is_null() {
                                let title_str = std::ffi::CStr::from_ptr(utf8).to_string_lossy();
                                if title_str.contains("Browser") {
                                    win = w;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if win.is_null() {
                let key_window: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                win = key_window(app, sel_registerName(c"keyWindow".as_ptr()));
            }
            if win.is_null() && !windows.is_null() {
                let first_obj: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                win = first_obj(windows, sel_registerName(c"firstObject".as_ptr()));
            }
            if win.is_null() {
                return;
            }

            let content_view_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let content_view = content_view_fn(win, sel_registerName(c"contentView".as_ptr()));
            if content_view.is_null() {
                return;
            }

            let is_flipped_fn: extern "C" fn(*mut c_void, *mut c_void) -> bool =
                std::mem::transmute(objc_msgSend as *const ());
            let is_flipped = is_flipped_fn(content_view, sel_registerName(c"isFlipped".as_ptr()));

            let bounds_fn: extern "C" fn(*mut c_void, *mut c_void) -> CGRect =
                std::mem::transmute(objc_msgSend as *const ());
            let bounds = bounds_fn(content_view, sel_registerName(c"bounds".as_ptr()));
            let top_bar_height = 38.0;
            let actual_y = if y > 0.0 {
                y
            } else if is_flipped {
                top_bar_height
            } else {
                0.0
            };
            let w = if width > 0.0 {
                width
            } else {
                bounds.size.width
            };
            let h = if height > 0.0 {
                height
            } else {
                (bounds.size.height - top_bar_height).max(50.0)
            };

            let mut guard = GLOBAL_WEBVIEW.lock().unwrap();
            let frame = CGRect {
                origin: CGPoint { x, y: actual_y },
                size: CGSize {
                    width: w,
                    height: h,
                },
            };

            let webview_ptr = if let Some(ref mut wv) = *guard {
                let set_frame: extern "C" fn(*mut c_void, *mut c_void, CGRect) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_frame(wv.raw, sel_registerName(c"setFrame:".as_ptr()), frame);
                let set_hidden: extern "C" fn(*mut c_void, *mut c_void, bool) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_hidden(wv.raw, sel_registerName(c"setHidden:".as_ptr()), false);
                wv.raw
            } else {
                let wk_cls = objc_getClass(c"WKWebView".as_ptr());
                if wk_cls.is_null() {
                    return;
                }
                let alloc_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let allocated = alloc_fn(wk_cls, sel_registerName(c"alloc".as_ptr()));

                let init_with_frame: extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    CGRect,
                ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
                let wv = init_with_frame(
                    allocated,
                    sel_registerName(c"initWithFrame:".as_ptr()),
                    frame,
                );
                if wv.is_null() {
                    return;
                }

                let set_autoresize: extern "C" fn(*mut c_void, *mut c_void, usize) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_autoresize(wv, sel_registerName(c"setAutoresizingMask:".as_ptr()), 18);

                let set_wants_layer: extern "C" fn(*mut c_void, *mut c_void, bool) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_wants_layer(wv, sel_registerName(c"setWantsLayer:".as_ptr()), true);

                let layer_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let layer = layer_fn(wv, sel_registerName(c"layer".as_ptr()));
                if !layer.is_null() {
                    let set_corner_radius: extern "C" fn(*mut c_void, *mut c_void, f64) =
                        std::mem::transmute(objc_msgSend as *const ());
                    set_corner_radius(layer, sel_registerName(c"setCornerRadius:".as_ptr()), 0.0);
                    let set_masks: extern "C" fn(*mut c_void, *mut c_void, bool) =
                        std::mem::transmute(objc_msgSend as *const ());
                    set_masks(layer, sel_registerName(c"setMasksToBounds:".as_ptr()), true);
                }

                let add_subview: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const ());
                add_subview(content_view, sel_registerName(c"addSubview:".as_ptr()), wv);

                *guard = Some(NativeWebView {
                    raw: wv,
                    current_url: String::new(),
                });
                wv
            };

            let should_load = guard
                .as_ref()
                .map(|wv| wv.current_url != url)
                .unwrap_or(true);

            if should_load {
                let ns_str_cls = objc_getClass(c"NSString".as_ptr());
                let str_with_utf8: extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    *const c_char,
                ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
                let c_url = CString::new(url).unwrap_or_default();
                let ns_url_str = str_with_utf8(
                    ns_str_cls,
                    sel_registerName(c"stringWithUTF8String:".as_ptr()),
                    c_url.as_ptr(),
                );

                let ns_url_cls = objc_getClass(c"NSURL".as_ptr());
                let url_with_str: extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    *mut c_void,
                ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
                let ns_url = url_with_str(
                    ns_url_cls,
                    sel_registerName(c"URLWithString:".as_ptr()),
                    ns_url_str,
                );

                if !ns_url.is_null() {
                    let ns_req_cls = objc_getClass(c"NSURLRequest".as_ptr());
                    let req_with_url: extern "C" fn(
                        *mut c_void,
                        *mut c_void,
                        *mut c_void,
                    ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
                    let req = req_with_url(
                        ns_req_cls,
                        sel_registerName(c"requestWithURL:".as_ptr()),
                        ns_url,
                    );

                    let load_req: extern "C" fn(
                        *mut c_void,
                        *mut c_void,
                        *mut c_void,
                    ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
                    load_req(webview_ptr, sel_registerName(c"loadRequest:".as_ptr()), req);
                }

                if let Some(ref mut wv) = *guard {
                    wv.current_url = url.to_string();
                }
            }
        }
    }

    pub fn detach() {
        if let Ok(mut guard) = GLOBAL_WEBVIEW.lock() {
            if let Some(wv) = guard.take() {
                unsafe {
                    let remove: extern "C" fn(*mut c_void, *mut c_void) =
                        std::mem::transmute(objc_msgSend as *const ());
                    remove(wv.raw, sel_registerName(c"removeFromSuperview".as_ptr()));
                }
            }
        }
    }

    pub fn set_hidden(hidden: bool) {
        if let Ok(guard) = GLOBAL_WEBVIEW.lock() {
            if let Some(ref wv) = *guard {
                unsafe {
                    let set_hidden_fn: extern "C" fn(*mut c_void, *mut c_void, bool) =
                        std::mem::transmute(objc_msgSend as *const ());
                    set_hidden_fn(wv.raw, sel_registerName(c"setHidden:".as_ptr()), hidden);
                }
            }
        }
    }

    pub fn go_back() {
        if let Ok(guard) = GLOBAL_WEBVIEW.lock() {
            if let Some(ref wv) = *guard {
                unsafe {
                    let go_back_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    go_back_fn(wv.raw, sel_registerName(c"goBack".as_ptr()));
                }
            }
        }
    }

    pub fn go_forward() {
        if let Ok(guard) = GLOBAL_WEBVIEW.lock() {
            if let Some(ref wv) = *guard {
                unsafe {
                    let go_fwd_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    go_fwd_fn(wv.raw, sel_registerName(c"goForward".as_ptr()));
                }
            }
        }
    }

    pub fn reload() {
        if let Ok(guard) = GLOBAL_WEBVIEW.lock() {
            if let Some(ref wv) = *guard {
                unsafe {
                    let reload_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    reload_fn(wv.raw, sel_registerName(c"reload".as_ptr()));
                }
            }
        }
    }

    pub fn set_zoom(zoom: f64) {
        if let Ok(guard) = GLOBAL_WEBVIEW.lock() {
            if let Some(ref wv) = *guard {
                unsafe {
                    let set_zoom_fn: extern "C" fn(*mut c_void, *mut c_void, f64) =
                        std::mem::transmute(objc_msgSend as *const ());
                    set_zoom_fn(wv.raw, sel_registerName(c"setPageZoom:".as_ptr()), zoom);
                }
            }
        }
    }

    pub fn current_title() -> Option<String> {
        if let Ok(guard) = GLOBAL_WEBVIEW.lock() {
            if let Some(ref wv) = *guard {
                unsafe {
                    let title_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    let title_obj = title_fn(wv.raw, sel_registerName(c"title".as_ptr()));
                    if !title_obj.is_null() {
                        let utf8_fn: extern "C" fn(*mut c_void, *mut c_void) -> *const c_char =
                            std::mem::transmute(objc_msgSend as *const ());
                        let utf8 = utf8_fn(title_obj, sel_registerName(c"UTF8String".as_ptr()));
                        if !utf8.is_null() {
                            let title =
                                std::ffi::CStr::from_ptr(utf8).to_string_lossy().to_string();
                            if !title.is_empty() {
                                return Some(title);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

pub fn ensure_regular_activation_policy() {
    #[cfg(target_os = "macos")]
    macos_webview::ensure_regular_activation_policy();
}

pub fn detach_native_webview() {
    #[cfg(target_os = "macos")]
    macos_webview::detach();
}

pub fn set_native_webview_hidden(_hidden: bool) {
    #[cfg(target_os = "macos")]
    macos_webview::set_hidden(_hidden);
}

pub fn browser_go_back() {
    #[cfg(target_os = "macos")]
    macos_webview::go_back();
}

pub fn browser_go_forward() {
    #[cfg(target_os = "macos")]
    macos_webview::go_forward();
}

pub fn browser_reload() {
    #[cfg(target_os = "macos")]
    macos_webview::reload();
}

pub fn browser_set_zoom(_zoom: f64) {
    #[cfg(target_os = "macos")]
    macos_webview::set_zoom(_zoom);
}

pub fn browser_current_title() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        macos_webview::current_title()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn copy_to_clipboard(text: &str) {
    #[cfg(target_os = "macos")]
    {
        let child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn();
        if let Ok(mut c) = child {
            if let Some(mut stdin) = c.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "printf '%s' '{}' | wl-copy 2>/dev/null || printf '%s' '{}' | xclip -selection clipboard 2>/dev/null",
                text, text
            ))
            .spawn();
    }
}

/// Renders the minimal Internal Browser viewport.
///
/// On macOS, attaches the native WKWebView directly into the window canvas.
/// On Linux or fallback, renders a clean full-bleed web page view without clutter.
pub fn render_internal_browser(page: &InternalBrowserPage, _engine: &BrowserEngineStatus) -> Div {
    #[cfg(target_os = "macos")]
    {
        macos_webview::attach_or_navigate(&page.url, 14.0, 14.0, 722.0, 405.0);
    }

    div()
        .w_full()
        .h_full()
        .px(px(14.))
        .pt(px(4.))
        .pb(px(14.))
        .child(
            div()
                .size_full()
                .rounded(px(14.))
                .bg(rgb(0x101015))
                .border_1()
                .border_color(rgb(Theme::CARD_STROKE))
                .overflow_hidden()
                .p(px(20.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(icons::sf_icon(SfSymbol::Globe, 14.0, 0x60a5fa))
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(Theme::TEXT_PRIMARY))
                                        .child(page.title.clone()),
                                ),
                        )
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(2.))
                                .rounded(px(5.))
                                .bg(rgb(0x202028))
                                .font(font("Geist Mono"))
                                .text_size(px(10.5))
                                .text_color(rgb(Theme::TEXT_TERTIARY))
                                .child(slugify_domain(&page.url)),
                        ),
                )
                .children(page.summary_blocks.iter().take(6).map(|paragraph| {
                    div()
                        .text_size(px(13.))
                        .line_height(px(19.))
                        .text_color(rgb(Theme::TEXT_SECONDARY))
                        .child(paragraph.clone())
                })),
        )
}

/// Dedicated Browser Window hosting Safari compact-mode tabs and full-bleed WebKit webview.
pub struct StandaloneBrowserWindow {
    pub tabs: Vec<BrowserTabDisplay>,
    pub active_tab: usize,
    pub command_bar_open: bool,
    pub command_query: String,
    pub command_selected: usize,
    pub history: Vec<String>,
    pub bookmarks: Vec<String>,
    pub editing_url: bool,
    pub url_input: String,
    pub zoom: f64,
    pub copied_toast: bool,
    pub focus_handle: Option<FocusHandle>,
}

impl StandaloneBrowserWindow {
    pub fn new(initial_url: Option<String>) -> Self {
        Self::with_focus(initial_url, None)
    }

    pub fn with_context(
        initial_url: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let fh = cx.focus_handle();
        fh.focus(window);
        Self::with_focus(initial_url, Some(fh))
    }

    pub fn with_focus(initial_url: Option<String>, focus_handle: Option<FocusHandle>) -> Self {
        let initial = initial_url.unwrap_or_else(|| "https://docs.rs".to_string());
        let tabs = vec![
            BrowserTabDisplay {
                title: "Docs.rs".to_string(),
                url: initial.clone(),
                is_active: true,
            },
            BrowserTabDisplay {
                title: "GitHub".to_string(),
                url: "https://github.com".to_string(),
                is_active: false,
            },
            BrowserTabDisplay {
                title: "Rust".to_string(),
                url: "https://www.rust-lang.org".to_string(),
                is_active: false,
            },
        ];
        let command_bar_open = std::env::var_os("ALPENGLOWED_BROWSER_CMD_BAR").is_some();
        Self {
            tabs,
            active_tab: 0,
            command_bar_open,
            command_query: String::new(),
            command_selected: 0,
            history: vec![initial],
            bookmarks: vec![
                "https://docs.rs".to_string(),
                "https://github.com".to_string(),
                "https://crates.io".to_string(),
            ],
            editing_url: false,
            url_input: String::new(),
            zoom: 1.0,
            copied_toast: false,
            focus_handle,
        }
    }

    pub fn active_url(&self) -> String {
        self.tabs
            .get(self.active_tab)
            .map(|t| t.url.clone())
            .unwrap_or_else(|| "https://docs.rs".to_string())
    }

    pub fn navigate(&mut self, url: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.title = slugify_domain(&url);
            tab.url = url.clone();
        }
        if !self.history.first().map(|h| h == &url).unwrap_or(false) {
            self.history.insert(0, url);
        }
        self.editing_url = false;
        self.command_bar_open = false;
        self.copied_toast = false;
    }

    pub fn new_tab(&mut self, url: Option<String>) {
        let target = url.unwrap_or_else(|| "https://docs.rs".to_string());
        for tab in &mut self.tabs {
            tab.is_active = false;
        }
        let id = self.tabs.len() + 1;
        self.tabs.push(BrowserTabDisplay {
            title: format!("Tab {id}"),
            url: target.clone(),
            is_active: true,
        });
        self.active_tab = self.tabs.len() - 1;
        self.editing_url = false;
        self.command_bar_open = false;
        self.copied_toast = false;
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() <= 1 {
            if let Some(tab) = self.tabs.first_mut() {
                tab.title = "Docs.rs".to_string();
                tab.url = "https://docs.rs".to_string();
            }
            return;
        }
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len().saturating_sub(1);
            }
            for (i, tab) in self.tabs.iter_mut().enumerate() {
                tab.is_active = i == self.active_tab;
            }
        }
        self.copied_toast = false;
    }

    pub fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active_tab = idx;
            for (i, tab) in self.tabs.iter_mut().enumerate() {
                tab.is_active = i == idx;
            }
            self.editing_url = false;
            self.command_bar_open = false;
            self.copied_toast = false;
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            let next = (self.active_tab + 1) % self.tabs.len();
            self.switch_tab(next);
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            let prev = if self.active_tab == 0 {
                self.tabs.len().saturating_sub(1)
            } else {
                self.active_tab - 1
            };
            self.switch_tab(prev);
        }
    }

    pub fn go_back(&mut self) {
        browser_go_back();
    }

    pub fn go_forward(&mut self) {
        browser_go_forward();
    }

    pub fn reload(&mut self) {
        browser_reload();
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom + 0.1).min(2.5);
        browser_set_zoom(self.zoom);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom - 0.1).max(0.5);
        browser_set_zoom(self.zoom);
    }

    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
        browser_set_zoom(1.0);
    }

    pub fn toggle_bookmark(&mut self) {
        let url = self.active_url();
        if let Some(pos) = self.bookmarks.iter().position(|b| b == &url) {
            self.bookmarks.remove(pos);
        } else {
            self.bookmarks.push(url);
        }
    }

    pub fn is_bookmarked(&self) -> bool {
        self.bookmarks.contains(&self.active_url())
    }

    pub fn copy_url(&mut self) {
        let url = self.active_url();
        copy_to_clipboard(&url);
        self.copied_toast = true;
    }

    pub fn execute_command(&mut self, query: &str) {
        let q = query.trim();
        if q.is_empty() {
            return;
        }
        let target_url = if q.starts_with("http://") || q.starts_with("https://") {
            q.to_string()
        } else if q.starts_with('!') {
            let parts: Vec<&str> = q.splitn(2, ' ').collect();
            let bang = parts[0];
            let rest = parts.get(1).copied().unwrap_or("");
            match bang {
                "!g" => format!("https://www.google.com/search?q={rest}"),
                "!gh" => format!("https://github.com/search?q={rest}"),
                "!rs" => format!("https://docs.rs/releases/search?query={rest}"),
                "!cr" | "!crate" => format!("https://crates.io/search?q={rest}"),
                "!yt" => format!("https://www.youtube.com/results?search_query={rest}"),
                "!w" | "!wiki" => {
                    format!("https://en.wikipedia.org/wiki/Special:Search?search={rest}")
                }
                _ => format!("https://duckduckgo.com/?q={rest}"),
            }
        } else if q.contains('.') && !q.contains(' ') {
            format!("https://{q}")
        } else {
            format!("https://duckduckgo.com/?q={}", q)
        };
        self.navigate(target_url);
        self.command_bar_open = false;
        self.command_query.clear();
        self.editing_url = false;
    }

    pub fn omnibar_candidates(&self) -> Vec<OmnibarCandidate> {
        let q = self.command_query.trim();
        let mut list = Vec::new();

        if !q.is_empty() {
            if q.starts_with("http://") || q.starts_with("https://") {
                list.push(OmnibarCandidate {
                    title: "Open URL".to_string(),
                    subtitle: q.to_string(),
                    icon: SfSymbol::Globe,
                    icon_color: TC_ACCENT,
                    action: OmnibarAction::Navigate(q.to_string()),
                });
            } else if q.starts_with('!') {
                let parts: Vec<&str> = q.splitn(2, ' ').collect();
                let bang = parts[0];
                let rest = parts.get(1).copied().unwrap_or("");
                let (engine_name, target) = match bang {
                    "!g" => ("Google", format!("https://www.google.com/search?q={rest}")),
                    "!gh" => ("GitHub", format!("https://github.com/search?q={rest}")),
                    "!rs" => (
                        "Docs.rs",
                        format!("https://docs.rs/releases/search?query={rest}"),
                    ),
                    "!cr" | "!crate" => ("Crates.io", format!("https://crates.io/search?q={rest}")),
                    "!yt" => (
                        "YouTube",
                        format!("https://www.youtube.com/results?search_query={rest}"),
                    ),
                    "!w" | "!wiki" => (
                        "Wikipedia",
                        format!("https://en.wikipedia.org/wiki/Special:Search?search={rest}"),
                    ),
                    _ => ("DuckDuckGo", format!("https://duckduckgo.com/?q={rest}")),
                };
                list.push(OmnibarCandidate {
                    title: format!("Search {engine_name}"),
                    subtitle: format!("Query: '{rest}'"),
                    icon: SfSymbol::BoltFill,
                    icon_color: 0xf59e0b,
                    action: OmnibarAction::Navigate(target),
                });
            } else if q.contains('.') && !q.contains(' ') {
                list.push(OmnibarCandidate {
                    title: "Go to Website".to_string(),
                    subtitle: format!("https://{q}"),
                    icon: SfSymbol::Globe,
                    icon_color: TC_ACCENT,
                    action: OmnibarAction::Navigate(format!("https://{q}")),
                });
            } else {
                list.push(OmnibarCandidate {
                    title: "Search DuckDuckGo".to_string(),
                    subtitle: format!("Search web for '{q}'"),
                    icon: SfSymbol::MagnifyingGlass,
                    icon_color: TC_ACCENT,
                    action: OmnibarAction::Navigate(format!("https://duckduckgo.com/?q={q}")),
                });
                list.push(OmnibarCandidate {
                    title: "Search Docs.rs".to_string(),
                    subtitle: format!("Rust documentation for '{q}'"),
                    icon: SfSymbol::DocOnDoc,
                    icon_color: 0xf59e0b,
                    action: OmnibarAction::Navigate(format!(
                        "https://docs.rs/releases/search?query={q}"
                    )),
                });
                list.push(OmnibarCandidate {
                    title: "Search GitHub".to_string(),
                    subtitle: format!("Repositories for '{q}'"),
                    icon: SfSymbol::DocTextMagnifyingGlass,
                    icon_color: 0xa78bfa,
                    action: OmnibarAction::Navigate(format!("https://github.com/search?q={q}")),
                });
            }

            let q_lower = q.to_lowercase();
            for (idx, tab) in self.tabs.iter().enumerate() {
                if idx != self.active_tab
                    && (tab.title.to_lowercase().contains(&q_lower)
                        || tab.url.to_lowercase().contains(&q_lower))
                {
                    list.push(OmnibarCandidate {
                        title: format!("Switch to Tab {}: {}", idx + 1, tab.title),
                        subtitle: tab.url.clone(),
                        icon: SfSymbol::MacWindow,
                        icon_color: 0xa78bfa,
                        action: OmnibarAction::SwitchTab(idx),
                    });
                }
            }
        } else {
            list.push(OmnibarCandidate {
                title: "New Tab".to_string(),
                subtitle: "Open blank tab (⌘T)".to_string(),
                icon: SfSymbol::Plus,
                icon_color: 0x94a3b8,
                action: OmnibarAction::NewTab,
            });
            list.push(OmnibarCandidate {
                title: "Reload Page".to_string(),
                subtitle: "Refresh current page (⌘R)".to_string(),
                icon: SfSymbol::ArrowUturnBackward,
                icon_color: 0x94a3b8,
                action: OmnibarAction::Reload,
            });
            list.push(OmnibarCandidate {
                title: "Copy URL".to_string(),
                subtitle: self.active_url(),
                icon: SfSymbol::DocOnDoc,
                icon_color: TC_ACCENT,
                action: OmnibarAction::CopyUrl,
            });
            list.push(OmnibarCandidate {
                title: "Bookmark Page".to_string(),
                subtitle: "Toggle bookmark for active page (⌘D)".to_string(),
                icon: SfSymbol::Bookmark,
                icon_color: 0xf59e0b,
                action: OmnibarAction::Bookmark,
            });
            list.push(OmnibarCandidate {
                title: "Duplicate Tab".to_string(),
                subtitle: "Open copy of active tab in new tab".to_string(),
                icon: SfSymbol::DocOnDoc,
                icon_color: 0x94a3b8,
                action: OmnibarAction::DuplicateTab,
            });
            for (idx, tab) in self.tabs.iter().enumerate() {
                if idx != self.active_tab {
                    list.push(OmnibarCandidate {
                        title: format!("Switch to Tab {}: {}", idx + 1, tab.title),
                        subtitle: tab.url.clone(),
                        icon: SfSymbol::MacWindow,
                        icon_color: 0xa78bfa,
                        action: OmnibarAction::SwitchTab(idx),
                    });
                }
            }
            list.push(OmnibarCandidate {
                title: "Close Tab".to_string(),
                subtitle: "Close current active tab (⌘W)".to_string(),
                icon: SfSymbol::Xmark,
                icon_color: 0xf87171,
                action: OmnibarAction::CloseTab,
            });
            list.push(OmnibarCandidate {
                title: "Install as Web App".to_string(),
                subtitle: "Create .desktop application launcher".to_string(),
                icon: SfSymbol::ArrowDownCircle,
                icon_color: 0x34d399,
                action: OmnibarAction::InstallWebApp,
            });
        }
        list
    }

    pub fn execute_omnibar_action(&mut self, action: OmnibarAction) {
        match action {
            OmnibarAction::Navigate(url) => self.navigate(url),
            OmnibarAction::SwitchTab(idx) => self.switch_tab(idx),
            OmnibarAction::NewTab => self.new_tab(None),
            OmnibarAction::Reload => self.reload(),
            OmnibarAction::CopyUrl => self.copy_url(),
            OmnibarAction::Bookmark => self.toggle_bookmark(),
            OmnibarAction::DuplicateTab => self.new_tab(Some(self.active_url())),
            OmnibarAction::CloseTab => self.close_tab(self.active_tab),
            OmnibarAction::InstallWebApp => {
                let _ = install_web_app(&self.active_url(), &self.active_url());
            }
        }
        self.command_bar_open = false;
        self.command_query.clear();
    }
}

const TC_SCRIM: u32 = 0x161618;
const TC_PANEL_STROKE: u32 = 0x26262b;
const TC_GLASS_BG: u32 = 0x242429;
const TC_BORDER: u32 = 0x3e3e46;
const TC_CARD_FILL: u32 = 0x1f1f24;
const TC_CARD_STROKE: u32 = 0x2b2b31;
const TC_TEXT: u32 = 0xffffff;
const TC_TEXT_DIM: u32 = 0x9a9aa0;
const TC_TEXT_FAINT: u32 = 0x68686e;
const TC_ACCENT: u32 = 0x38bdf8;

#[derive(Clone, Debug)]
pub struct OmnibarCandidate {
    pub title: String,
    pub subtitle: String,
    pub icon: SfSymbol,
    pub icon_color: u32,
    pub action: OmnibarAction,
}

#[derive(Clone, Debug)]
pub enum OmnibarAction {
    Navigate(String),
    SwitchTab(usize),
    NewTab,
    Reload,
    CopyUrl,
    Bookmark,
    DuplicateTab,
    CloseTab,
    InstallWebApp,
}

impl Render for StandaloneBrowserWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(target_os = "macos")]
        let active_url = self.active_url();
        let is_bookmarked = self.is_bookmarked();
        let zoom_pct = (self.zoom * 100.0).round() as u32;

        #[cfg(target_os = "macos")]
        {
            if !self.command_bar_open {
                macos_webview::attach_or_navigate(&active_url, 0.0, 0.0, 0.0, 0.0);
                macos_webview::set_hidden(false);
            } else {
                macos_webview::set_hidden(true);
            }
        }

        let candidates = self.omnibar_candidates();

        if (self.editing_url || self.command_bar_open) && self.focus_handle.is_some() {
            if let Some(ref fh) = self.focus_handle {
                fh.focus(window);
            }
        }

        let mut container = div()
            .id("alpenglow-browser-container")
            .key_context("alpenglow_browser")
            .size_full()
            .bg(rgb(TC_SCRIM))
            .overflow_hidden()
            .flex()
            .flex_col();

        if let Some(ref fh) = self.focus_handle {
            container = container.track_focus(fh);
        }

        container
            .on_action(cx.listener(|this, _: &BrowserNewTab, _, cx| {
                this.new_tab(None);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserCloseTab, _, cx| {
                this.close_tab(this.active_tab);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserReload, _, cx| {
                this.reload();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserFocusUrl, _, cx| {
                this.editing_url = !this.editing_url;
                if this.editing_url {
                    this.url_input = this.active_url();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserToggleCmdBar, _, cx| {
                this.command_bar_open = !this.command_bar_open;
                if !this.command_bar_open {
                    this.command_query.clear();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserGoBack, _, cx| {
                this.go_back();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserGoForward, _, cx| {
                this.go_forward();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserZoomIn, _, cx| {
                this.zoom_in();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserZoomOut, _, cx| {
                this.zoom_out();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserResetZoom, _, cx| {
                this.reset_zoom();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserToggleBookmark, _, cx| {
                this.toggle_bookmark();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserCopyUrl, _, cx| {
                this.copy_url();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserNextTab, _, cx| {
                this.next_tab();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserPrevTab, _, cx| {
                this.prev_tab();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab1, _, cx| {
                this.switch_tab(0);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab2, _, cx| {
                this.switch_tab(1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab3, _, cx| {
                this.switch_tab(2);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab4, _, cx| {
                this.switch_tab(3);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab5, _, cx| {
                this.switch_tab(4);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab6, _, cx| {
                this.switch_tab(5);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab7, _, cx| {
                this.switch_tab(6);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab8, _, cx| {
                this.switch_tab(7);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserSelectTab9, _, cx| {
                this.switch_tab(8);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowserEscape, _, cx| {
                if this.command_bar_open {
                    this.command_bar_open = false;
                    this.command_query.clear();
                    cx.notify();
                } else if this.editing_url {
                    this.editing_url = false;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let key = event.keystroke.key.as_str();
                let mods = event.keystroke.modifiers;

                // Modifiers (Cmd / Ctrl)
                if mods.platform || mods.control {
                    if key == "v" {
                        if let Some(item) = cx.read_from_clipboard() {
                            if let Some(text) = item.text() {
                                if this.command_bar_open {
                                    this.command_query.push_str(&text);
                                    cx.notify();
                                    cx.stop_propagation();
                                    return;
                                } else if this.editing_url {
                                    this.url_input.push_str(&text);
                                    cx.notify();
                                    cx.stop_propagation();
                                    return;
                                }
                            }
                        }
                    }
                    if key == "backspace" {
                        if this.command_bar_open {
                            this.command_query.clear();
                            this.command_selected = 0;
                            cx.notify();
                            cx.stop_propagation();
                            return;
                        } else if this.editing_url {
                            this.url_input.clear();
                            cx.notify();
                            cx.stop_propagation();
                            return;
                        }
                    }
                    if key == "t" {
                        this.new_tab(None);
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "w" {
                        this.close_tab(this.active_tab);
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "r" {
                        this.reload();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "l" {
                        this.editing_url = !this.editing_url;
                        if this.editing_url {
                            this.url_input = this.active_url();
                        }
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "k" || key == "space" {
                        this.command_bar_open = !this.command_bar_open;
                        if !this.command_bar_open {
                            this.command_query.clear();
                        }
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "[" || key == "left" {
                        this.go_back();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "]" || key == "right" {
                        this.go_forward();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "=" || key == "+" {
                        this.zoom_in();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "-" {
                        this.zoom_out();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "0" {
                        this.reset_zoom();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "d" {
                        this.toggle_bookmark();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if mods.shift && key == "c" {
                        this.copy_url();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if let Ok(num) = key.parse::<usize>() {
                        if (1..=9).contains(&num) {
                            this.switch_tab(num - 1);
                            cx.notify();
                            cx.stop_propagation();
                            return;
                        }
                    }
                }

                if key == "escape" {
                    if this.command_bar_open {
                        this.command_bar_open = false;
                        this.command_query.clear();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if this.editing_url {
                        this.editing_url = false;
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                }

                let typed_char: Option<String> = if let Some(ch) = event.keystroke.key_char.as_deref() {
                    Some(ch.to_string())
                } else if key == "space" {
                    Some(" ".to_string())
                } else if key.chars().count() == 1 && !key.chars().all(|c| c.is_control()) {
                    Some(key.to_string())
                } else {
                    None
                };

                if this.editing_url {
                    if key == "enter" {
                        let q = this.url_input.clone();
                        this.execute_command(&q);
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "backspace" {
                        this.url_input.pop();
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if !mods.platform && !mods.control && !mods.alt {
                        if let Some(ch) = typed_char.as_deref() {
                            this.url_input.push_str(ch);
                            cx.notify();
                            cx.stop_propagation();
                            return;
                        }
                    }
                }

                if this.command_bar_open {
                    if key == "enter" {
                        let cands = this.omnibar_candidates();
                        if !cands.is_empty() {
                            let sel = this.command_selected.min(cands.len() - 1);
                            let action = cands[sel].action.clone();
                            this.execute_omnibar_action(action);
                        } else {
                            let q = this.command_query.clone();
                            this.execute_command(&q);
                        }
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "backspace" {
                        this.command_query.pop();
                        this.command_selected = 0;
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "up" {
                        this.command_selected = this.command_selected.saturating_sub(1);
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if key == "down" {
                        let max_idx = this.omnibar_candidates().len().saturating_sub(1);
                        this.command_selected = (this.command_selected + 1).min(max_idx);
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    if !mods.platform && !mods.control && !mods.alt {
                        if let Some(ch) = typed_char.as_deref() {
                            this.command_query.push_str(ch);
                            this.command_selected = 0;
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }
                }
            }))
            // 1. Slim 38px Safari Compact Mode Tab Chips Bar
            .child(
                div()
                    .w_full()
                    .h(px(38.))
                    .px(px(8.))
                    .bg(rgb(TC_SCRIM))
                    .border_b_1()
                    .border_color(rgb(TC_PANEL_STROKE))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .when(cfg!(target_os = "macos"), |r| r.pl(px(74.)))
                            // Nav Back button
                            .child(
                                div()
                                    .id("browser-back-button")
                                    .h(px(26.))
                                    .w(px(26.))
                                    .rounded(px(6.))
                                    .bg(rgb(TC_CARD_FILL))
                                    .border_1()
                                    .border_color(rgb(TC_CARD_STROKE))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.go_back();
                                        cx.notify();
                                    }))
                                    .child(icons::sf_icon(SfSymbol::ChevronLeft, 10.5, TC_TEXT_DIM)),
                            )
                            // Nav Forward button
                            .child(
                                div()
                                    .id("browser-forward-button")
                                    .h(px(26.))
                                    .w(px(26.))
                                    .rounded(px(6.))
                                    .bg(rgb(TC_CARD_FILL))
                                    .border_1()
                                    .border_color(rgb(TC_CARD_STROKE))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.go_forward();
                                        cx.notify();
                                    }))
                                    .child(icons::sf_icon(SfSymbol::ChevronRight, 10.5, TC_TEXT_DIM)),
                            )
                            // Floating Tab Chips
                            .children(self.tabs.iter().enumerate().map(|(idx, tab)| {
                                let is_active = idx == self.active_tab;
                                if is_active {
                                    // Active Tab Chip in Safari compact style: contains the current link!
                                    div()
                                        .id(SharedString::from(format!("tab-chip-active-{idx}")))
                                        .h(px(28.))
                                        .px(px(10.))
                                        .rounded(px(8.))
                                        .bg(rgb(TC_GLASS_BG))
                                        .border_1()
                                        .border_color(rgb(TC_BORDER))
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .child(icons::sf_icon(SfSymbol::LockFill, 10.0, 0x34d399))
                                        .when(self.editing_url, |chip| {
                                            chip.child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .font(font("Geist Mono"))
                                                            .text_size(px(11.5))
                                                            .text_color(rgb(if self.url_input.is_empty() {
                                                                TC_TEXT_FAINT
                                                            } else {
                                                                TC_TEXT
                                                            }))
                                                            .child(if self.url_input.is_empty() {
                                                                "Search or enter URL...".to_string()
                                                            } else {
                                                                self.url_input.clone()
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(2.))
                                                            .h(px(14.))
                                                            .rounded(px(1.))
                                                            .bg(rgb(TC_ACCENT)),
                                                    ),
                                            )
                                        })
                                        .when(!self.editing_url, |chip| {
                                            chip.child(
                                                div()
                                                    .id("active-tab-url-text")
                                                    .font(font("Geist Mono"))
                                                    .text_size(px(11.5))
                                                    .text_color(rgb(TC_TEXT))
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.editing_url = true;
                                                        this.url_input = this.active_url();
                                                        cx.notify();
                                                    }))
                                                    .child(tab.url.clone()),
                                            )
                                            // Bookmark toggle
                                            .child(
                                                div()
                                                    .id("active-tab-bookmark-toggle")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_bookmark();
                                                        cx.notify();
                                                    }))
                                                    .child(icons::sf_icon(
                                                        SfSymbol::Bookmark,
                                                        9.5,
                                                        if is_bookmarked {
                                                            0xf59e0b
                                                        } else {
                                                            TC_TEXT_FAINT
                                                        },
                                                    )),
                                            )
                                            // Reload button
                                            .child(
                                                div()
                                                    .id("active-tab-reload-button")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.reload();
                                                        cx.notify();
                                                    }))
                                                    .child(icons::sf_icon(
                                                        SfSymbol::ArrowUturnBackward,
                                                        9.5,
                                                        TC_TEXT_DIM,
                                                    )),
                                            )
                                        })
                                        // Close tab button
                                        .child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "close-active-tab-{idx}"
                                                )))
                                                .text_size(px(11.))
                                                .text_color(rgb(TC_TEXT_FAINT))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.close_tab(idx);
                                                    cx.notify();
                                                }))
                                                .child("×"),
                                        )
                                } else {
                                    // Inactive Tab Chip
                                    div()
                                        .id(SharedString::from(format!("tab-chip-{idx}")))
                                        .h(px(26.))
                                        .px(px(8.))
                                        .rounded(px(6.))
                                        .bg(rgb(TC_CARD_FILL))
                                        .border_1()
                                        .border_color(rgb(TC_CARD_STROKE))
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.switch_tab(idx);
                                            cx.notify();
                                        }))
                                        .child(icons::sf_icon(SfSymbol::Globe, 10.0, TC_TEXT_FAINT))
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(rgb(TC_TEXT_DIM))
                                                .child(tab.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .id(SharedString::from(format!("close-chip-{idx}")))
                                                .text_size(px(10.5))
                                                .text_color(rgb(TC_TEXT_FAINT))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.close_tab(idx);
                                                    cx.notify();
                                                }))
                                                .child("×"),
                                        )
                                }
                            }))
                            // Plus chip for new tab
                            .child(
                                div()
                                    .id("tab-add-chip")
                                    .h(px(26.))
                                    .w(px(26.))
                                    .rounded(px(6.))
                                    .bg(rgb(TC_CARD_FILL))
                                    .border_1()
                                    .border_color(rgb(TC_CARD_STROKE))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.new_tab(None);
                                        cx.notify();
                                    }))
                                    .child(icons::sf_icon(SfSymbol::Plus, 10.5, TC_TEXT_DIM)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            // Zoom indicator
                            .when(zoom_pct != 100, |row| {
                                row.child(
                                    div()
                                        .id("browser-zoom-indicator")
                                        .h(px(24.))
                                        .px(px(6.))
                                        .rounded(px(5.))
                                        .bg(rgb(0x251c36))
                                        .border_1()
                                        .border_color(rgb(0x4c2e73))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.reset_zoom();
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .font(font("Geist Mono"))
                                                .text_size(px(10.0))
                                                .text_color(rgb(0xc084fc))
                                                .child(format!("{zoom_pct}%")),
                                        ),
                                )
                            })
                            // Copy URL pill
                            .child(
                                div()
                                    .id("browser-copy-url")
                                    .h(px(24.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(rgb(TC_CARD_FILL))
                                    .border_1()
                                    .border_color(rgb(TC_CARD_STROKE))
                                    .flex()
                                    .items_center()
                                    .gap(px(3.))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.copy_url();
                                        cx.notify();
                                    }))
                                    .child(icons::sf_icon(
                                        if self.copied_toast {
                                             SfSymbol::Checkmark
                                        } else {
                                             SfSymbol::DocOnDoc
                                        },
                                        10.0,
                                        if self.copied_toast {
                                             0x34d399
                                        } else {
                                             TC_TEXT_DIM
                                        },
                                    )),
                            )
                            // Command bar button
                            .child(
                                div()
                                    .id("browser-cmd-k-trigger")
                                    .h(px(24.))
                                    .px(px(7.))
                                    .rounded(px(5.))
                                    .bg(rgb(TC_CARD_FILL))
                                    .border_1()
                                    .border_color(rgb(TC_CARD_STROKE))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.command_bar_open = !this.command_bar_open;
                                        cx.notify();
                                    }))
                                    .child(icons::sf_icon(SfSymbol::MagnifyingGlass, 9.5, TC_TEXT_DIM))
                                    .child(
                                        div()
                                            .font(font("Geist Mono"))
                                            .text_size(px(10.5))
                                            .text_color(rgb(TC_TEXT_FAINT))
                                            .child("⌘K"),
                                    ),
                            ),
                    ),
            )
            // 2. Full-Bleed Edge-to-Edge Webview Viewport (0px Bezels!)
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .relative()
                    .child(
                        // Webview container canvas
                        div()
                            .size_full()
                            .bg(rgb(0x0a0a0f))
                            .flex()
                            .flex_col()
                            .justify_center()
                            .items_center()
                            .when(self.command_bar_open, |canvas| {
                                canvas.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(icons::sf_icon(SfSymbol::Globe, 28.0, TC_TEXT_FAINT))
                                        .child(
                                            div()
                                                .font(font("Geist Mono"))
                                                .text_size(px(13.))
                                                .text_color(rgb(TC_TEXT_DIM))
                                                .child(self.active_url()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(TC_TEXT_FAINT))
                                                .child("WebKit view paused while searching"),
                                        ),
                                )
                            }),
                    )
                    .when(self.command_bar_open, |parent| {
                        parent.child(
                            // 3. Floating Omnibar backdrop + modal overlay (dismisses on click off!)
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .flex_col()
                                .items_center()
                                .pt(px(40.))
                                // Backdrop (Click off closes modal)
                                .child(
                                    div()
                                        .id("omnibar-backdrop")
                                        .absolute()
                                        .inset_0()
                                        .bg(rgba(0x000000a0))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.command_bar_open = false;
                                            this.command_query.clear();
                                            cx.notify();
                                        })),
                                )
                                // Centered Floating Omnibar Panel
                                .child(
                                    div()
                                        .relative()
                                        .w(px(680.))
                                        .max_w(px(680.))
                                        .rounded(px(14.))
                                        .bg(rgb(TC_SCRIM))
                                        .border_1()
                                        .border_color(rgb(TC_PANEL_STROKE))
                                        .shadow_lg()
                                        .flex()
                                        .flex_col()
                                        .overflow_hidden()
                                        // Top search bar
                                        .child(
                                            div()
                                                .w_full()
                                                .h(px(52.))
                                                .px(px(16.))
                                                .border_b_1()
                                                .border_color(rgb(TC_PANEL_STROKE))
                                                .flex()
                                                .items_center()
                                                .gap(px(12.))
                                                .child(icons::sf_icon(
                                                    SfSymbol::MagnifyingGlass,
                                                    15.0,
                                                    TC_ACCENT,
                                                ))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(2.))
                                                        .child(
                                                            div()
                                                                .font(font("Geist Mono"))
                                                                .text_size(px(14.5))
                                                                .text_color(rgb(if self.command_query.is_empty() {
                                                                    TC_TEXT_FAINT
                                                                } else {
                                                                    TC_TEXT
                                                                }))
                                                                .child(if self.command_query.is_empty() {
                                                                    "Search web, enter URL, switch tabs, or run actions...".to_string()
                                                                } else {
                                                                    self.command_query.clone()
                                                                }),
                                                        )
                                                        // Glowing Cyan Caret!
                                                        .child(
                                                            div()
                                                                .w(px(2.))
                                                                .h(px(20.))
                                                                .rounded(px(1.))
                                                                .bg(rgb(TC_ACCENT)),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .font(font("Geist Mono"))
                                                        .text_size(px(11.0))
                                                        .text_color(rgb(TC_TEXT_FAINT))
                                                        .child("↵ select • esc close"),
                                                ),
                                        )
                                        // Candidates list
                                        .child(
                                            div()
                                                .w_full()
                                                .p(px(8.))
                                                .flex()
                                                .flex_col()
                                                .gap(px(3.))
                                                .children(candidates.into_iter().enumerate().map(
                                                    |(idx, cand)| {
                                                        let is_sel = idx == self.command_selected;
                                                        let cand_action = cand.action.clone();
                                                        div()
                                                            .id(SharedString::from(format!(
                                                                "browser-cmd-row-{idx}"
                                                            )))
                                                            .h(px(36.))
                                                            .px(px(10.))
                                                            .rounded(px(8.))
                                                            .bg(rgb(if is_sel {
                                                                TC_GLASS_BG
                                                            } else {
                                                                TC_SCRIM
                                                            }))
                                                            .when(is_sel, |row| {
                                                                row.border_1().border_color(rgb(TC_BORDER))
                                                            })
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .cursor_pointer()
                                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                                this.execute_omnibar_action(cand_action.clone());
                                                                cx.notify();
                                                            }))
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap(px(10.))
                                                                    .child(icons::sf_icon(
                                                                        cand.icon, 12.5, cand.icon_color,
                                                                    ))
                                                                    .child(
                                                                        div()
                                                                            .text_size(px(12.5))
                                                                            .font_weight(FontWeight::MEDIUM)
                                                                            .text_color(rgb(if is_sel {
                                                                                TC_TEXT
                                                                            } else {
                                                                                TC_TEXT_DIM
                                                                            }))
                                                                            .child(cand.title),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .font(font("Geist Mono"))
                                                                            .text_size(px(10.5))
                                                                            .text_color(rgb(TC_TEXT_FAINT))
                                                                            .child(cand.subtitle),
                                                                    ),
                                                            )
                                                            .when(is_sel, |row| {
                                                                row.child(
                                                                    div()
                                                                        .font(font("Geist Mono"))
                                                                        .text_size(px(10.5))
                                                                        .text_color(rgb(TC_ACCENT))
                                                                        .child("↵ Enter"),
                                                                )
                                                            })
                                                    },
                                                )),
                                        )
                                        // Bottom Footer
                                        .child(
                                            div()
                                                .w_full()
                                                .h(px(28.))
                                                .px(px(12.))
                                                .bg(rgb(0x131315))
                                                .border_t_1()
                                                .border_color(rgb(TC_PANEL_STROKE))
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .font(font("Geist Mono"))
                                                        .text_size(px(10.0))
                                                        .text_color(rgb(TC_TEXT_FAINT))
                                                        .child("↑↓ navigate • ↵ select • esc close"),
                                                )
                                                .child(
                                                    div()
                                                        .font(font("Geist Mono"))
                                                        .text_size(px(10.0))
                                                        .text_color(rgb(TC_TEXT_FAINT))
                                                        .child("Bangs: !g google • !gh github • !rs docs.rs • !yt youtube • !w wiki"),
                                                ),
                                        ),
                                )
                        )
                    }),
            )
    }
}

pub fn open_browser_window(initial_url: Option<String>, cx: &mut App) {
    bind_browser_keys(cx);
    ensure_regular_activation_policy();

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::new(px(100.), px(60.)),
            size: Size {
                width: px(1060.),
                height: px(700.),
            },
        })),
        titlebar: Some(TitlebarOptions {
            title: Some("Alpenglow Browser".into()),
            appears_transparent: true,
            traffic_light_position: Some(Point::new(px(12.), px(12.))),
        }),
        window_min_size: Some(Size {
            width: px(520.),
            height: px(340.),
        }),
        kind: crepuscularity_gpui::WindowKind::Normal,
        is_movable: true,
        is_resizable: true,
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };
    let _ = cx.open_window(options, move |window, cx| {
        let view = cx.new(|cx| StandaloneBrowserWindow::with_context(initial_url, window, cx));
        window.activate_window();
        view
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_domain_and_html_parser_extract_clean_blocks() {
        assert_eq!(slugify_domain("https://docs.rs/gpui"), "docs-rs");
        let html = "<html><head><title>Example Site</title></head><body><h1>Welcome to Example</h1><p>This is a minimal internal browser page with Web App installation.</p></body></html>";
        let page = parse_html_to_page("https://example.com", html);
        assert_eq!(page.title, "Example Site");
        assert!(page
            .summary_blocks
            .iter()
            .any(|b| b.contains("Welcome to Example")));
    }

    #[test]
    fn install_web_app_writes_valid_freedesktop_desktop_file() {
        let tmp = std::env::temp_dir().join(format!("alpenglow-xdg-test-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &tmp);
        let path =
            install_web_app("Rust Docs", "https://docs.rs").expect("should write .desktop entry");
        let content = std::fs::read_to_string(&path).expect("read .desktop file");
        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Name=Rust Docs"));
        assert!(content.contains("Exec=alpenglowed open https://docs.rs"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detect_browser_engine_provides_package_manager_install_command() {
        let status = detect_browser_engine();
        assert!(!status.engine_name.is_empty());
        assert!(!status.install_command.is_empty());
    }

    #[test]
    fn standalone_browser_tab_management_and_navigation_works() {
        let mut win = StandaloneBrowserWindow::new(Some("https://docs.rs".to_string()));
        assert_eq!(win.tabs.len(), 3);
        assert_eq!(win.active_url(), "https://docs.rs");

        win.new_tab(Some("https://alpenglowed.dev".to_string()));
        assert_eq!(win.tabs.len(), 4);
        assert_eq!(win.active_tab, 3);
        assert_eq!(win.active_url(), "https://alpenglowed.dev");

        win.prev_tab();
        assert_eq!(win.active_tab, 2);

        win.next_tab();
        assert_eq!(win.active_tab, 3);

        win.close_tab(3);
        assert_eq!(win.tabs.len(), 3);
        assert_eq!(win.active_tab, 2);
    }

    #[test]
    fn standalone_browser_bookmarks_and_zoom_work() {
        let mut win = StandaloneBrowserWindow::new(Some("https://my-custom-site.org".to_string()));
        assert!(!win.is_bookmarked());

        win.toggle_bookmark();
        assert!(win.is_bookmarked());

        win.toggle_bookmark();
        assert!(!win.is_bookmarked());

        let initial_zoom = win.zoom;
        win.zoom_in();
        assert!(win.zoom > initial_zoom);

        win.zoom_out();
        assert!((win.zoom - initial_zoom).abs() < f64::EPSILON);

        win.reset_zoom();
        assert!((win.zoom - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn standalone_browser_command_execution_resolves_bangs_and_urls() {
        let mut win = StandaloneBrowserWindow::new(Some("about:blank".to_string()));

        win.execute_command("!rs gpui");
        assert_eq!(
            win.active_url(),
            "https://docs.rs/releases/search?query=gpui"
        );

        win.execute_command("!gh warpdotdev");
        assert_eq!(win.active_url(), "https://github.com/search?q=warpdotdev");

        win.execute_command("alpenglowed.dev");
        assert_eq!(win.active_url(), "https://alpenglowed.dev");

        win.execute_command("https://crates.io");
        assert_eq!(win.active_url(), "https://crates.io");

        win.execute_command("simple search terms");
        assert_eq!(
            win.active_url(),
            "https://duckduckgo.com/?q=simple search terms"
        );
    }

    #[test]
    fn standalone_browser_omnibar_candidates_and_actions() {
        let mut win = StandaloneBrowserWindow::new(Some("https://example.org".to_string()));
        win.new_tab(Some("https://docs.rs".to_string()));
        win.new_tab(Some("https://github.com".to_string()));

        // Empty query candidates: should offer tab actions and quick navigation
        win.command_query.clear();
        let cands = win.omnibar_candidates();
        assert!(!cands.is_empty());
        assert!(cands.iter().any(|c| c.title == "New Tab"));
        assert!(cands.iter().any(|c| c.title == "Duplicate Tab"));
        assert!(cands.iter().any(|c| c.title.starts_with("Switch to Tab")));

        // Search bang !rs
        win.command_query = "!rs gpui".to_string();
        let cands = win.omnibar_candidates();
        assert_eq!(cands[0].title, "Search Docs.rs");
        match &cands[0].action {
            OmnibarAction::Navigate(url) => {
                assert_eq!(url, "https://docs.rs/releases/search?query=gpui");
            }
            _ => panic!("Expected Navigate action"),
        }

        // Direct domain query
        win.command_query = "alpenglowed.dev".to_string();
        let cands = win.omnibar_candidates();
        assert_eq!(cands[0].title, "Go to Website");

        // Tab filter query
        win.command_query = "example".to_string();
        let cands = win.omnibar_candidates();
        assert!(cands
            .iter()
            .any(|c| matches!(c.action, OmnibarAction::SwitchTab(_))));

        // Execute omnibar action: NewTab
        let tabs_before = win.tabs.len();
        win.execute_omnibar_action(OmnibarAction::NewTab);
        assert_eq!(win.tabs.len(), tabs_before + 1);

        // Execute omnibar action: SwitchTab(0)
        win.execute_omnibar_action(OmnibarAction::SwitchTab(0));
        assert_eq!(win.active_tab, 0);

        // Execute omnibar action: DuplicateTab
        let tabs_before_dup = win.tabs.len();
        win.execute_omnibar_action(OmnibarAction::DuplicateTab);
        assert_eq!(win.tabs.len(), tabs_before_dup + 1);

        // Execute omnibar action: CloseTab
        let tabs_before_close = win.tabs.len();
        win.execute_omnibar_action(OmnibarAction::CloseTab);
        assert_eq!(win.tabs.len(), tabs_before_close - 1);
    }
}
