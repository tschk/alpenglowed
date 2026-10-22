use std::path::PathBuf;
use std::process::Command;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{div, font, px, rgb, Div, FontWeight};

use crate::components::Theme;
use crate::icons::{self, SfSymbol};

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

            let key_window: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let mut win = key_window(app, sel_registerName(c"keyWindow".as_ptr()));
            if win.is_null() {
                let windows_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let windows = windows_fn(app, sel_registerName(c"windows".as_ptr()));
                if !windows.is_null() {
                    let first_obj: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    win = first_obj(windows, sel_registerName(c"firstObject".as_ptr()));
                }
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

            let mut guard = GLOBAL_WEBVIEW.lock().unwrap();
            let frame = CGRect {
                origin: CGPoint { x, y },
                size: CGSize { width, height },
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

                let set_wants_layer: extern "C" fn(*mut c_void, *mut c_void, bool) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_wants_layer(wv, sel_registerName(c"setWantsLayer:".as_ptr()), true);

                let layer_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let layer = layer_fn(wv, sel_registerName(c"layer".as_ptr()));
                if !layer.is_null() {
                    let set_corner_radius: extern "C" fn(*mut c_void, *mut c_void, f64) =
                        std::mem::transmute(objc_msgSend as *const ());
                    set_corner_radius(layer, sel_registerName(c"setCornerRadius:".as_ptr()), 14.0);
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
}

pub fn detach_native_webview() {
    #[cfg(target_os = "macos")]
    macos_webview::detach();
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
}
