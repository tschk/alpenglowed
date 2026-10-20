use std::path::PathBuf;
use std::process::Command;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{div, font, px, rgb, Div, FontWeight, SharedString};

use crate::components::{self, Theme};
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

/// Extracts readable text blocks and title from raw HTML.
pub fn parse_html_to_page(url: &str, html: &str) -> InternalBrowserPage {
    let title = extract_tag_content(html, "title").unwrap_or_else(|| domain_title(url));
    let mut blocks = Vec::new();
    for tag in ["h1", "h2", "p", "li"] {
        for extracted in extract_all_tag_contents(html, tag) {
            let clean = collapse_whitespace(&extracted);
            if clean.len() >= 12 && blocks.len() < 6 && !blocks.contains(&clean) {
                blocks.push(clean);
            }
        }
    }
    if blocks.is_empty() {
        blocks = curated_blocks_for_url(url);
    }
    let webapp_installed = webapp_desktop_path(url).exists();
    InternalBrowserPage {
        url: url.to_string(),
        title,
        summary_blocks: blocks,
        links: default_quicklinks(),
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

    // Fast non-blocking local/curated preview unless live fetch is requested via env
    if matches!(
        std::env::var("ALPENGLOWED_BROWSER_LIVE_FETCH").as_deref(),
        Ok("1" | "true")
    ) {
        if let Ok(out) = Command::new("curl")
            .args(["-sL", "--max-time", "2", &normalized])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                let html = String::from_utf8_lossy(&out.stdout);
                return parse_html_to_page(&normalized, &html);
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
            "Click 'Install App' above to pin this site as a native Linux desktop launcher entry.".to_string(),
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
            "Use 'Install App' to create a native ~/.local/share/applications/*.desktop shortcut, or 'Install Engine' to install WebKit2GTK.".to_string(),
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

/// Renders the Internal Browser view (with Web App Install & Browser Engine Install actions).
pub fn render_internal_browser<FNav, FInstallApp, FInstallEngine, FOpenExt>(
    page: &InternalBrowserPage,
    engine: &BrowserEngineStatus,
    on_navigate: FNav,
    on_install_app: FInstallApp,
    on_install_engine: FInstallEngine,
    on_open_external: FOpenExt,
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
{
    div()
        .w_full()
        .px(px(12.))
        .pt(px(4.))
        .pb(px(8.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            // Browser Address & Install Bar
            div()
                .w_full()
                .h(px(36.))
                .px(px(10.))
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
                        .gap(px(7.))
                        .overflow_hidden()
                        .child(icons::sf_icon(SfSymbol::LockFill, 11.0, 0x34d399))
                        .child(
                            div()
                                .font(font("Geist Mono"))
                                .text_size(px(12.))
                                .text_color(rgb(Theme::TEXT_PRIMARY))
                                .child(page.url.clone()),
                        )
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(5.))
                                .bg(rgb(0x282830))
                                .text_size(px(10.5))
                                .text_color(rgb(Theme::TEXT_TERTIARY))
                                .child(engine.engine_name.clone()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            // Install Web App (.desktop) button
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
                                .gap(px(5.))
                                .cursor_pointer()
                                .on_click(move |_, w, cx| on_install_app(w, cx))
                                .child(icons::sf_icon(
                                    if page.webapp_installed {
                                        SfSymbol::Checkmark
                                    } else {
                                        SfSymbol::ArrowDownCircle
                                    },
                                    11.0,
                                    if page.webapp_installed {
                                        0x34d399
                                    } else {
                                        Theme::TEXT_PRIMARY
                                    },
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(if page.webapp_installed {
                                            0x34d399
                                        } else {
                                            Theme::TEXT_PRIMARY
                                        }))
                                        .child(if page.webapp_installed {
                                            "App Installed"
                                        } else {
                                            "Install App"
                                        }),
                                ),
                        )
                        .child(
                            // Install Native Browser Engine button
                            div()
                                .id("browser-install-engine-btn")
                                .h(px(24.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .bg(rgb(0x25252d))
                                .border_1()
                                .border_color(rgb(0x383842))
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .cursor_pointer()
                                .on_click(move |_, w, cx| on_install_engine(w, cx))
                                .child(icons::sf_icon(SfSymbol::ArrowDownCircle, 11.0, 0x60a5fa))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(Theme::TEXT_SECONDARY))
                                        .child(format!("Install ({})", engine.package_manager)),
                                ),
                        )
                        .child(
                            // Open in external browser button
                            div()
                                .id("browser-open-external-btn")
                                .w(px(24.))
                                .h(px(24.))
                                .rounded(px(6.))
                                .bg(rgb(0x25252d))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_click(move |_, w, cx| on_open_external(w, cx))
                                .child(icons::sf_icon(
                                    SfSymbol::ArrowUpRightSquare,
                                    12.0,
                                    Theme::TEXT_SECONDARY,
                                )),
                        ),
                ),
        )
        .when_some(page.status_banner.as_ref(), |col, msg| {
            col.child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(6.))
                    .rounded(px(8.))
                    .bg(rgb(0x162b24))
                    .border_1()
                    .border_color(rgb(0x285e48))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icons::sf_icon(SfSymbol::Checkmark, 11.0, 0x34d399))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(rgb(0x34d399))
                            .child(msg.clone()),
                    ),
            )
        })
        .child(
            // Rendered Web Page Content Card
            div()
                .w_full()
                .rounded(px(12.))
                .bg(rgb(0x1b1b20))
                .border_1()
                .border_color(rgb(Theme::CARD_STROKE))
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icons::sf_icon(SfSymbol::Globe, 15.0, 0x60a5fa))
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(Theme::TEXT_PRIMARY))
                                .child(page.title.clone()),
                        ),
                )
                .children(page.summary_blocks.iter().map(|paragraph| {
                    div()
                        .text_size(px(12.5))
                        .text_color(rgb(Theme::TEXT_SECONDARY))
                        .child(paragraph.clone())
                })),
        )
        .child(components::section_header("Quicklinks", false))
        .child(
            div()
                .w_full()
                .px(px(4.))
                .flex()
                .items_center()
                .gap(px(6.))
                .children(page.links.iter().enumerate().map(|(idx, (label, target))| {
                    let target_url = target.clone();
                    let nav = on_navigate.clone();
                    div()
                        .id(SharedString::from(format!("browser-quicklink-{idx}")))
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .bg(rgb(0x222228))
                        .border_1()
                        .border_color(rgb(Theme::CARD_STROKE))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .on_click(move |_, w, cx| nav(target_url.clone(), w, cx))
                        .child(icons::sf_icon(SfSymbol::Link, 11.0, Theme::TEXT_SECONDARY))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(Theme::TEXT_PRIMARY))
                                .child(label.clone()),
                        )
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
