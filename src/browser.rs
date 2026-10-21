use std::path::PathBuf;
use std::process::Command;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{div, font, px, rgb, Div, FontWeight, InteractiveElement, SharedString};

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

/// Renders the Internal Browser view with tabs, toolbar, search bangs, and live page content.
#[allow(clippy::too_many_arguments)]
pub fn render_internal_browser<
    FNav,
    FInstallApp,
    FInstallEngine,
    FOpenExt,
    FSwitchTab,
    FNewTab,
    FCloseTab,
    FBack,
    FReload,
>(
    page: &InternalBrowserPage,
    engine: &BrowserEngineStatus,
    tabs: &[BrowserTabDisplay],
    on_navigate: FNav,
    on_install_app: FInstallApp,
    on_install_engine: FInstallEngine,
    on_open_external: FOpenExt,
    on_switch_tab: FSwitchTab,
    on_new_tab: FNewTab,
    on_close_tab: FCloseTab,
    on_back: FBack,
    on_reload: FReload,
) -> Div
where
    FNav: Fn(String, &mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App)
        + Clone
        + 'static,
    FInstallApp:
        Fn(&mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App) + Clone + 'static,
    FInstallEngine:
        Fn(&mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App) + Clone + 'static,
    FOpenExt: Fn(&mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App) + Clone + 'static,
    FSwitchTab: Fn(usize, &mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App)
        + Clone
        + 'static,
    FNewTab: Fn(&mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App) + Clone + 'static,
    FCloseTab: Fn(usize, &mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App)
        + Clone
        + 'static,
    FBack: Fn(&mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App) + Clone + 'static,
    FReload: Fn(&mut crepuscularity_gpui::Window, &mut crepuscularity_gpui::App) + Clone + 'static,
{
    div()
        .w_full()
        .px(px(12.))
        .pt(px(2.))
        .pb(px(8.))
        .flex()
        .flex_col()
        .gap(px(6.))
        // 1. Browser Tab Strip
        .child(
            div()
                .w_full()
                .h(px(28.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .overflow_hidden()
                        .children(tabs.iter().enumerate().map(|(idx, tab)| {
                            let switch_fn = on_switch_tab.clone();
                            let close_fn = on_close_tab.clone();
                            let is_active = tab.is_active;
                            div()
                                .id(SharedString::from(format!("browser-tab-{idx}")))
                                .h(px(26.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .bg(rgb(if is_active { 0x2b2b35 } else { 0x1c1c22 }))
                                .border_1()
                                .border_color(rgb(if is_active {
                                    Theme::CARD_BORDER
                                } else {
                                    Theme::CARD_STROKE
                                }))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .cursor_pointer()
                                .on_click(move |_, w, cx| switch_fn(idx, w, cx))
                                .child(icons::sf_icon(
                                    SfSymbol::Globe,
                                    10.5,
                                    if is_active {
                                        0x60a5fa
                                    } else {
                                        Theme::TEXT_TERTIARY
                                    },
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(if is_active {
                                            FontWeight::MEDIUM
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(rgb(if is_active {
                                            Theme::TEXT_PRIMARY
                                        } else {
                                            Theme::TEXT_SECONDARY
                                        }))
                                        .child(tab.title.chars().take(18).collect::<String>()),
                                )
                                .when(tabs.len() > 1, |row| {
                                    row.child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "browser-close-tab-{idx}"
                                            )))
                                            .w(px(14.))
                                            .h(px(14.))
                                            .rounded(px(3.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .on_click(move |_, w, cx| close_fn(idx, w, cx))
                                            .child(icons::sf_icon(
                                                SfSymbol::Xmark,
                                                8.5,
                                                Theme::TEXT_TERTIARY,
                                            )),
                                    )
                                })
                        })),
                )
                .child(
                    div()
                        .id("browser-new-tab-btn")
                        .w(px(24.))
                        .h(px(24.))
                        .rounded(px(6.))
                        .bg(rgb(0x22222a))
                        .border_1()
                        .border_color(rgb(Theme::CARD_STROKE))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_click(move |_, w, cx| on_new_tab(w, cx))
                        .child(icons::sf_icon(SfSymbol::Plus, 11.0, Theme::TEXT_SECONDARY)),
                ),
        )
        // 2. Navigation & Omnibox Toolbar
        .child(
            div()
                .w_full()
                .h(px(34.))
                .px(px(8.))
                .rounded(px(10.))
                .bg(rgb(0x1e1e24))
                .border_1()
                .border_color(rgb(Theme::CARD_STROKE))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            div()
                                .id("browser-back-btn")
                                .w(px(22.))
                                .h(px(22.))
                                .rounded(px(5.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_click(move |_, w, cx| on_back(w, cx))
                                .child(icons::sf_icon(
                                    SfSymbol::ChevronLeft,
                                    11.0,
                                    Theme::TEXT_SECONDARY,
                                )),
                        )
                        .child(
                            div()
                                .id("browser-reload-btn")
                                .w(px(22.))
                                .h(px(22.))
                                .rounded(px(5.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_click(move |_, w, cx| on_reload(w, cx))
                                .child(icons::sf_icon(
                                    SfSymbol::Clock,
                                    10.5,
                                    Theme::TEXT_SECONDARY,
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .px(px(8.))
                                .h(px(24.))
                                .rounded(px(6.))
                                .bg(rgb(0x26262f))
                                .child(icons::sf_icon(SfSymbol::LockFill, 10.0, 0x34d399))
                                .child(
                                    div()
                                        .font(font("Geist Mono"))
                                        .text_size(px(11.5))
                                        .text_color(rgb(Theme::TEXT_PRIMARY))
                                        .child(page.url.clone()),
                                )
                                .child(
                                    div()
                                        .id("browser-engine-status-pill")
                                        .px(px(5.))
                                        .py(px(0.5))
                                        .rounded(px(4.))
                                        .bg(rgb(if engine.installed { 0x1e1e26 } else { 0x3b1c1c }))
                                        .text_size(px(10.))
                                        .text_color(rgb(if engine.installed {
                                            Theme::TEXT_TERTIARY
                                        } else {
                                            0xf87171
                                        }))
                                        .cursor_pointer()
                                        .on_click(move |_, w, cx| on_install_engine(w, cx))
                                        .child(engine.engine_name.clone()),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            div()
                                .id("browser-install-webapp-btn")
                                .h(px(24.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .bg(rgb(if page.webapp_installed {
                                    0x162b24
                                } else {
                                    0x2b2b34
                                }))
                                .border_1()
                                .border_color(rgb(if page.webapp_installed {
                                    0x34d399
                                } else {
                                    0x3e3e48
                                }))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .cursor_pointer()
                                .on_click(move |_, w, cx| on_install_app(w, cx))
                                .child(icons::sf_icon(
                                    if page.webapp_installed {
                                        SfSymbol::Checkmark
                                    } else {
                                        SfSymbol::ArrowDownCircle
                                    },
                                    10.5,
                                    if page.webapp_installed {
                                        0x34d399
                                    } else {
                                        Theme::TEXT_PRIMARY
                                    },
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(if page.webapp_installed {
                                            0x34d399
                                        } else {
                                            Theme::TEXT_PRIMARY
                                        }))
                                        .child(if page.webapp_installed {
                                            "Installed"
                                        } else {
                                            "Pin App"
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .id("browser-open-external-btn")
                                .h(px(24.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .bg(rgb(0x25252d))
                                .border_1()
                                .border_color(rgb(0x383842))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .cursor_pointer()
                                .on_click(move |_, w, cx| on_open_external(w, cx))
                                .child(icons::sf_icon(SfSymbol::ArrowUpRightSquare, 10.5, 0x60a5fa))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(Theme::TEXT_SECONDARY))
                                        .child("Open"),
                                ),
                        ),
                ),
        )
        // 3. Search Bangs Quick Access Strip
        .child(
            div()
                .w_full()
                .px(px(2.))
                .flex()
                .items_center()
                .gap(px(5.))
                .children(
                    [
                        ("!g Google", "https://www.google.com"),
                        ("!gh GitHub", "https://github.com"),
                        ("!ddg DuckDuckGo", "https://duckduckgo.com"),
                        ("!rs Docs.rs", "https://docs.rs"),
                        ("!wiki Wikipedia", "https://en.wikipedia.org"),
                        ("!yt YouTube", "https://www.youtube.com"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(idx, (label, target))| {
                        let nav = on_navigate.clone();
                        let target_url = target.to_string();
                        div()
                            .id(SharedString::from(format!("browser-bang-chip-{idx}")))
                            .h(px(22.))
                            .px(px(7.))
                            .rounded(px(5.))
                            .bg(rgb(0x202026))
                            .border_1()
                            .border_color(rgb(Theme::CARD_STROKE))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .cursor_pointer()
                            .on_click(move |_, w, cx| nav(target_url.clone(), w, cx))
                            .child(
                                div()
                                    .font(font("Geist Mono"))
                                    .text_size(px(10.5))
                                    .text_color(rgb(Theme::TEXT_TERTIARY))
                                    .child(label),
                            )
                    }),
                ),
        )
        .when_some(page.status_banner.as_ref(), |col, msg| {
            col.child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(7.))
                    .bg(rgb(0x162b24))
                    .border_1()
                    .border_color(rgb(0x285e48))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icons::sf_icon(SfSymbol::Checkmark, 10.5, 0x34d399))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x34d399))
                            .child(msg.clone()),
                    ),
            )
        })
        // 4. Live Rendered Web Page Card
        .child(
            div()
                .w_full()
                .rounded(px(12.))
                .bg(rgb(0x1b1b20))
                .border_1()
                .border_color(rgb(Theme::CARD_STROKE))
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(6.))
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
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(Theme::TEXT_PRIMARY))
                                        .child(page.title.clone()),
                                ),
                        )
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(4.))
                                .bg(rgb(0x24242d))
                                .text_size(px(10.5))
                                .text_color(rgb(Theme::TEXT_TERTIARY))
                                .child(slugify_domain(&page.url)),
                        ),
                )
                .children(page.summary_blocks.iter().take(5).map(|paragraph| {
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(Theme::TEXT_SECONDARY))
                        .child(paragraph.clone())
                })),
        )
        // 5. Page Links & Navigation Section
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(5.))
                .overflow_hidden()
                .children(
                    page.links
                        .iter()
                        .take(4)
                        .enumerate()
                        .map(|(idx, (label, target))| {
                            let target_url = target.clone();
                            let nav = on_navigate.clone();
                            div()
                                .id(SharedString::from(format!("browser-page-link-{idx}")))
                                .h(px(24.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .bg(rgb(0x222228))
                                .border_1()
                                .border_color(rgb(Theme::CARD_STROKE))
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .cursor_pointer()
                                .on_click(move |_, w, cx| nav(target_url.clone(), w, cx))
                                .child(icons::sf_icon(SfSymbol::Link, 10.0, Theme::TEXT_SECONDARY))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(rgb(Theme::TEXT_PRIMARY))
                                        .child(label.clone()),
                                )
                        }),
                ),
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
