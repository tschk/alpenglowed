use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use gpui::{px, rgb, svg, AssetSource, Result as GpuiResult, SharedString, Styled, Svg};

/// Cross-platform SF Symbol icon identifier.
///
/// On macOS, `IconAssets` resolves any SF Symbol name directly via AppKit's
/// `NSImage(systemSymbolName:)` -> `CUINamedVectorGlyph` -> `CGPath` vector outline
/// (with instant fallback to the embedded vector library).
/// On Linux, `IconAssets` serves the matching crisp vector SVG from `BUILTIN_SF_SYMBOLS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfSymbol {
    MagnifyingGlass,
    ChevronLeft,
    ChevronRight,
    RectangleLeftHalfFilled,
    RectangleRightHalfFilled,
    RectangleTopHalfFilled,
    RectangleBottomHalfFilled,
    RectangleInsetTopLeadingFilled,
    RectangleInsetTopTrailingFilled,
    RectangleInsetBottomLeadingFilled,
    RectangleInsetBottomTrailingFilled,
    ArrowUpLeftAndArrowDownRight,
    RectangleInsetFilled,
    MacWindow,
    RectangleCenterInsetFilled,
    ArrowUturnBackward,
    DoorLeftHandOpen,
    SquareGrid2x2,
    ArrowLeftAndRightSquare,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    DocOnDoc,
    DocTextMagnifyingGlass,
    Link,
    Folder,
    Terminal,
    Globe,
    CurlyBraces,
    PlusForwardslashMinus,
    Sparkles,
    BoltFill,
    Return,
    DeleteLeft,
    Command,
    Equal,
    Checkmark,
    Xmark,
    Clock,
    Gearshape,
    SpeakerWave2,
    MoonFill,
    LockFill,
    Power,
    PlusSquareOnSquare,
    MinusSquare,
    ArrowClockwise,
    ArrowDownCircle,
    Bookmark,
    PlayFill,
    Trash,
    ChevronDown,
    ChevronUp,
    ArrowUpRightSquare,
    Custom(&'static str),
}

impl SfSymbol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MagnifyingGlass => "magnifyingglass",
            Self::ChevronLeft => "chevron.left",
            Self::ChevronRight => "chevron.right",
            Self::RectangleLeftHalfFilled => "rectangle.lefthalf.filled",
            Self::RectangleRightHalfFilled => "rectangle.righthalf.filled",
            Self::RectangleTopHalfFilled => "rectangle.tophalf.filled",
            Self::RectangleBottomHalfFilled => "rectangle.bottomhalf.filled",
            Self::RectangleInsetTopLeadingFilled => "rectangle.inset.topleading.filled",
            Self::RectangleInsetTopTrailingFilled => "rectangle.inset.toptrailing.filled",
            Self::RectangleInsetBottomLeadingFilled => "rectangle.inset.bottomleading.filled",
            Self::RectangleInsetBottomTrailingFilled => "rectangle.inset.bottomtrailing.filled",
            Self::ArrowUpLeftAndArrowDownRight => "arrow.up.left.and.arrow.down.right",
            Self::RectangleInsetFilled => "rectangle.inset.filled",
            Self::MacWindow => "macwindow",
            Self::RectangleCenterInsetFilled => "rectangle.center.inset.filled",
            Self::ArrowUturnBackward => "arrow.uturn.backward",
            Self::DoorLeftHandOpen => "door.left.hand.open",
            Self::SquareGrid2x2 => "square.grid.2x2",
            Self::ArrowLeftAndRightSquare => "arrow.left.and.right.square",
            Self::ArrowLeft => "arrow.left",
            Self::ArrowRight => "arrow.right",
            Self::ArrowUp => "arrow.up",
            Self::ArrowDown => "arrow.down",
            Self::DocOnDoc => "doc.on.doc",
            Self::DocTextMagnifyingGlass => "doc.text.magnifyingglass",
            Self::Link => "link",
            Self::Folder => "folder",
            Self::Terminal => "terminal",
            Self::Globe => "globe",
            Self::CurlyBraces => "curlybraces",
            Self::PlusForwardslashMinus => "plus.forwardslash.minus",
            Self::Sparkles => "sparkles",
            Self::BoltFill => "bolt.fill",
            Self::Return => "return",
            Self::DeleteLeft => "delete.left",
            Self::Command => "command",
            Self::Equal => "equal",
            Self::Checkmark => "checkmark",
            Self::Xmark => "xmark",
            Self::Clock => "clock",
            Self::Gearshape => "gearshape",
            Self::SpeakerWave2 => "speaker.wave.2",
            Self::MoonFill => "moon.fill",
            Self::LockFill => "lock.fill",
            Self::Power => "power",
            Self::PlusSquareOnSquare => "plus.square.on.square",
            Self::MinusSquare => "minus.square",
            Self::ArrowClockwise => "arrow.clockwise",
            Self::ArrowDownCircle => "arrow.down.circle",
            Self::Bookmark => "bookmark",
            Self::PlayFill => "play.fill",
            Self::Trash => "trash",
            Self::ChevronDown => "chevron.down",
            Self::ChevronUp => "chevron.up",
            Self::ArrowUpRightSquare => "arrow.up.right.square",
            Self::Custom(name) => name,
        }
    }

    /// Maps an `SfSymbol` to its Freedesktop XDG symbolic icon counterpart on Linux.
    pub const fn freedesktop_name(self) -> &'static str {
        match self {
            Self::MagnifyingGlass => "system-search-symbolic",
            Self::ChevronLeft => "go-previous-symbolic",
            Self::ChevronRight => "go-next-symbolic",
            Self::ChevronUp => "pan-up-symbolic",
            Self::ChevronDown => "pan-down-symbolic",
            Self::ArrowLeft => "go-previous-symbolic",
            Self::ArrowRight => "go-next-symbolic",
            Self::ArrowUp => "go-up-symbolic",
            Self::ArrowDown => "go-down-symbolic",
            Self::ArrowClockwise => "view-refresh-symbolic",
            Self::ArrowDownCircle => "folder-download-symbolic",
            Self::Bookmark => "user-bookmarks-symbolic",
            Self::PlayFill => "media-playback-start-symbolic",
            Self::Trash => "user-trash-symbolic",
            Self::ArrowUpRightSquare => "adw-external-link-symbolic",
            Self::DocOnDoc => "edit-copy-symbolic",
            Self::DocTextMagnifyingGlass => "system-search-symbolic",
            Self::Link => "insert-link-symbolic",
            Self::Folder => "folder-symbolic",
            Self::Terminal => "utilities-terminal-symbolic",
            Self::Globe => "web-browser-symbolic",
            Self::Gearshape => "emblem-system-symbolic",
            Self::SpeakerWave2 => "audio-volume-high-symbolic",
            Self::MoonFill => "weather-clear-night-symbolic",
            Self::LockFill => "system-lock-screen-symbolic",
            Self::Power => "system-shutdown-symbolic",
            Self::Checkmark => "object-select-symbolic",
            Self::Xmark => "window-close-symbolic",
            Self::Clock => "preferences-system-time-symbolic",
            Self::Custom(name) => name,
            _ => "applications-system-symbolic",
        }
    }

    pub fn asset_path(self) -> SharedString {
        SharedString::from(format!("sf/{}.svg", self.as_str()))
    }

    pub fn render(self, size_px: f32, color_hex: u32) -> Svg {
        svg()
            .path(self.asset_path())
            .size(px(size_px))
            .text_color(rgb(color_hex))
    }

    pub fn from_glyph(glyph: &str) -> Self {
        match glyph {
            "⌕" | "magnifyingglass" => Self::MagnifyingGlass,
            "‹" | "chevron.left" => Self::ChevronLeft,
            "›" | "chevron.right" => Self::ChevronRight,
            "⌄" | "chevron.down" => Self::ChevronDown,
            "⌃" | "chevron.up" => Self::ChevronUp,
            "◧" | "rectangle.lefthalf.filled" => Self::RectangleLeftHalfFilled,
            "◨" | "rectangle.righthalf.filled" => Self::RectangleRightHalfFilled,
            "⬒" | "rectangle.tophalf.filled" => Self::RectangleTopHalfFilled,
            "⬓" | "rectangle.bottomhalf.filled" => Self::RectangleBottomHalfFilled,
            "◰" | "rectangle.inset.topleading.filled" => Self::RectangleInsetTopLeadingFilled,
            "◳" | "rectangle.inset.toptrailing.filled" => Self::RectangleInsetTopTrailingFilled,
            "◱" | "rectangle.inset.bottomleading.filled" => {
                Self::RectangleInsetBottomLeadingFilled
            }
            "◲" | "rectangle.inset.bottomtrailing.filled" => {
                Self::RectangleInsetBottomTrailingFilled
            }
            "⤢" | "arrow.up.left.and.arrow.down.right" => Self::ArrowUpLeftAndArrowDownRight,
            "▣" | "rectangle.inset.filled" => Self::RectangleInsetFilled,
            "▢" | "macwindow" => Self::MacWindow,
            "⌖" | "rectangle.center.inset.filled" => Self::RectangleCenterInsetFilled,
            "↺" | "arrow.uturn.backward" => Self::ArrowUturnBackward,
            "↻" | "arrow.clockwise" => Self::ArrowClockwise,
            "⬇" | "↓◯" | "arrow.down.circle" => Self::ArrowDownCircle,
            "★" | "bookmark" => Self::Bookmark,
            "▶" | "play.fill" => Self::PlayFill,
            "🗑" | "trash" => Self::Trash,
            "↗" | "arrow.up.right.square" => Self::ArrowUpRightSquare,
            "⊞" | "door.left.hand.open" | "square.grid.2x2" => Self::SquareGrid2x2,
            "↔" | "arrow.left.and.right.square" => Self::ArrowLeftAndRightSquare,
            "←" | "arrow.left" => Self::ArrowLeft,
            "→" | "arrow.right" => Self::ArrowRight,
            "↑" | "arrow.up" => Self::ArrowUp,
            "↓" | "arrow.down" => Self::ArrowDown,
            "📋" | "doc.on.doc" => Self::DocOnDoc,
            "📄" | "🔎" | "doc.text.magnifyingglass" => Self::DocTextMagnifyingGlass,
            "🔗" | "link" => Self::Link,
            "📁" | "folder" => Self::Folder,
            ">_" | "terminal" => Self::Terminal,
            "🌐" | "🧭" | "globe" => Self::Globe,
            "{}" | "curlybraces" => Self::CurlyBraces,
            "=" | "plus.forwardslash.minus" => Self::PlusForwardslashMinus,
            "?" | "◆" | "sparkles" => Self::Sparkles,
            "⚙" | "gearshape" => Self::Gearshape,
            "⌘" | "command" => Self::Command,
            "↵" | "return" => Self::Return,
            "⌫" | "delete.left" => Self::DeleteLeft,
            "equal" => Self::Equal,
            "✓" | "checkmark" => Self::Checkmark,
            "✕" | "xmark" => Self::Xmark,
            "＋" | "plus.square.on.square" => Self::PlusSquareOnSquare,
            "－" | "minus.square" => Self::MinusSquare,
            _ => Self::Sparkles,
        }
    }

    pub fn from_keycap(key: &str) -> Option<Self> {
        match key {
            "↵" | "Return" | "Enter" => Some(Self::Return),
            "⌫" | "Backspace" | "Delete" => Some(Self::DeleteLeft),
            "⌘" | "Cmd" => Some(Self::Command),
            "←" => Some(Self::ArrowLeft),
            "→" => Some(Self::ArrowRight),
            "↑" => Some(Self::ArrowUp),
            "↓" => Some(Self::ArrowDown),
            _ => None,
        }
    }
}

pub fn sf_icon(symbol: SfSymbol, size_px: f32, color_hex: u32) -> Svg {
    symbol.render(size_px, color_hex)
}

/// Renders a Linux Freedesktop XDG application or symbolic icon by name (`xdg/<name>.svg`),
/// falling back to the built-in SF/Lucide vector library if no system SVG is installed.
pub fn xdg_icon(name: &str, size_px: f32, color_hex: u32) -> Svg {
    svg()
        .path(SharedString::from(format!("xdg/{name}.svg")))
        .size(px(size_px))
        .text_color(rgb(color_hex))
}

pub struct IconAssets;

impl AssetSource for IconAssets {
    fn load(&self, path: &str) -> GpuiResult<Option<Cow<'static, [u8]>>> {
        if let Some(xdg_name) = path
            .strip_prefix("xdg/")
            .and_then(|s| s.strip_suffix(".svg"))
        {
            if let Some(svg_bytes) = lookup_xdg_or_sf_svg(xdg_name) {
                return Ok(Some(Cow::Borrowed(svg_bytes)));
            }
        }
        let name = path
            .strip_prefix("sf/")
            .and_then(|s| s.strip_suffix(".svg"))
            .unwrap_or(path);
        if let Some(svg_bytes) = lookup_sf_symbol_svg(name) {
            return Ok(Some(Cow::Borrowed(svg_bytes)));
        }
        Ok(None)
    }

    fn list(&self, _path: &str) -> GpuiResult<Vec<SharedString>> {
        Ok(BUILTIN_SF_SYMBOLS
            .iter()
            .map(|(name, _)| SharedString::from(format!("sf/{name}.svg")))
            .collect())
    }
}

/// Resolves a Freedesktop XDG SVG icon from standard Linux icon theme directories
/// (`/usr/share/icons/hicolor/scalable/apps`, `Adwaita`, `Papirus`, `breeze`, `/usr/share/pixmaps`,
/// `~/.local/share/icons`, Flatpak exports), normalizing `fill` to `currentColor` when needed.
pub fn resolve_linux_xdg_svg(icon_name: &str) -> Option<String> {
    let clean = icon_name
        .trim()
        .trim_end_matches(".svg")
        .trim_end_matches(".png");
    if clean.is_empty() || clean.contains('/') || clean.contains("..") {
        return None;
    }

    let mut search_paths = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from) {
        search_paths.push(data_home.join(format!("icons/hicolor/scalable/apps/{clean}.svg")));
    }
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        search_paths.push(home.join(format!(
            ".local/share/icons/hicolor/scalable/apps/{clean}.svg"
        )));
    }
    for dir in [
        "/usr/share/icons/hicolor/scalable/apps",
        "/usr/share/icons/Adwaita/scalable/apps",
        "/usr/share/icons/Adwaita/symbolic/actions",
        "/usr/share/icons/Adwaita/symbolic/apps",
        "/usr/share/icons/Adwaita/symbolic/places",
        "/usr/share/icons/Adwaita/symbolic/status",
        "/usr/share/icons/Papirus/48x48/apps",
        "/usr/share/icons/breeze/apps/48",
        "/usr/share/pixmaps",
        "/var/lib/flatpak/exports/share/icons/hicolor/scalable/apps",
    ] {
        search_paths.push(std::path::PathBuf::from(format!("{dir}/{clean}.svg")));
    }

    for candidate in search_paths {
        if let Ok(raw) = std::fs::read_to_string(&candidate) {
            if raw.contains("<svg") && raw.contains("<path") {
                return Some(raw);
            }
        }
    }
    None
}

pub fn lookup_xdg_or_sf_svg(name: &str) -> Option<&'static [u8]> {
    static XDG_CACHE: OnceLock<Mutex<HashMap<String, &'static [u8]>>> = OnceLock::new();
    let cache = XDG_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(&bytes) = guard.get(name) {
            return Some(bytes);
        }
    }
    if let Some(svg_str) = resolve_linux_xdg_svg(name) {
        let leaked: &'static [u8] = Box::leak(svg_str.into_bytes().into_boxed_slice());
        if let Ok(mut guard) = cache.lock() {
            guard.insert(name.to_string(), leaked);
        }
        return Some(leaked);
    }
    lookup_sf_symbol_svg(name)
}

pub fn lookup_sf_symbol_svg(name: &str) -> Option<&'static [u8]> {
    #[cfg(target_os = "macos")]
    {
        static CACHE: OnceLock<Mutex<HashMap<String, &'static [u8]>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(&bytes) = guard.get(name) {
                return Some(bytes);
            }
        }
        if let Some(svg_str) = macos_sf_symbol_svg(name) {
            let leaked: &'static [u8] = Box::leak(svg_str.into_bytes().into_boxed_slice());
            if let Ok(mut guard) = cache.lock() {
                guard.insert(name.to_string(), leaked);
            }
            return Some(leaked);
        }
    }
    if let Some((_, v)) = BUILTIN_SF_SYMBOLS.iter().find(|(k, _)| *k == name) {
        return Some(v.as_bytes());
    }
    if let Some(svg_str) = resolve_linux_xdg_svg(name) {
        let leaked: &'static [u8] = Box::leak(svg_str.into_bytes().into_boxed_slice());
        return Some(leaked);
    }
    BUILTIN_SF_SYMBOLS.first().map(|(_, v)| v.as_bytes())
}

#[cfg(target_os = "macos")]
fn macos_sf_symbol_svg(name: &str) -> Option<String> {
    use std::ffi::{c_char, c_void, CString};

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    #[repr(C)]
    struct CGPathElement {
        element_type: i32,
        points: *const CGPoint,
    }

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPathGetBoundingBox(path: *const c_void) -> CGRect;
        fn CGPathIsEmpty(path: *const c_void) -> bool;
        fn CGPathApply(
            path: *const c_void,
            info: *mut c_void,
            function: unsafe extern "C" fn(*mut c_void, *const CGPathElement),
        );
    }

    struct PathCtx {
        scale: f64,
        ox: f64,
        oy: f64,
        min_y: f64,
        d: String,
    }

    unsafe extern "C" fn path_applier(info: *mut c_void, elem_ptr: *const CGPathElement) {
        if info.is_null() || elem_ptr.is_null() {
            return;
        }
        let ctx = unsafe { &mut *(info as *mut PathCtx) };
        let elem = unsafe { &*elem_ptr };
        let pts = elem.points;
        let tx = |p: CGPoint| -> f64 { p.x * ctx.scale + ctx.ox };
        let ty = |p: CGPoint| -> f64 { (p.y - ctx.min_y) * ctx.scale + ctx.oy };
        use std::fmt::Write;
        match elem.element_type {
            0 => {
                let p0 = unsafe { *pts };
                let _ = write!(ctx.d, "M{:.2} {:.2}", tx(p0), ty(p0));
            }
            1 => {
                let p0 = unsafe { *pts };
                let _ = write!(ctx.d, "L{:.2} {:.2}", tx(p0), ty(p0));
            }
            2 => {
                let p0 = unsafe { *pts };
                let p1 = unsafe { *pts.add(1) };
                let _ = write!(
                    ctx.d,
                    "Q{:.2} {:.2} {:.2} {:.2}",
                    tx(p0),
                    ty(p0),
                    tx(p1),
                    ty(p1)
                );
            }
            3 => {
                let p0 = unsafe { *pts };
                let p1 = unsafe { *pts.add(1) };
                let p2 = unsafe { *pts.add(2) };
                let _ = write!(
                    ctx.d,
                    "C{:.2} {:.2} {:.2} {:.2} {:.2} {:.2}",
                    tx(p0),
                    ty(p0),
                    tx(p1),
                    ty(p1),
                    tx(p2),
                    ty(p2)
                );
            }
            4 => {
                ctx.d.push('Z');
            }
            _ => {}
        }
    }

    unsafe {
        let c_name = CString::new(name).ok()?;
        let ns_string_cls = objc_getClass(c"NSString".as_ptr());
        let sel_str_utf8 = sel_registerName(c"stringWithUTF8String:".as_ptr());
        let msg_ptr_cstr: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *const c_char,
        ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
        let ns_name = msg_ptr_cstr(ns_string_cls, sel_str_utf8, c_name.as_ptr());
        if ns_name.is_null() {
            return None;
        }

        let ns_image_cls = objc_getClass(c"NSImage".as_ptr());
        let sel_sys_sym =
            sel_registerName(c"imageWithSystemSymbolName:accessibilityDescription:".as_ptr());
        let msg_img: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
        ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
        let img = msg_img(ns_image_cls, sel_sys_sym, ns_name, std::ptr::null_mut());
        if img.is_null() {
            return None;
        }

        let msg_id: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> bool =
            std::mem::transmute(objc_msgSend as *const ());
        let sel_responds = sel_registerName(c"respondsToSelector:".as_ptr());

        let reps = msg_id(img, sel_registerName(c"representations".as_ptr()));
        if reps.is_null() {
            return None;
        }
        let rep = msg_id(reps, sel_registerName(c"firstObject".as_ptr()));
        if rep.is_null() {
            return None;
        }

        let sel_vg = sel_registerName(c"vectorGlyph".as_ptr());
        if !msg_bool(rep, sel_responds, sel_vg) {
            return None;
        }
        let vg = msg_id(rep, sel_vg);
        if vg.is_null() {
            return None;
        }

        let sel_cgpath = sel_registerName(c"CGPath".as_ptr());
        if !msg_bool(vg, sel_responds, sel_cgpath) {
            return None;
        }
        let cg_path = msg_id(vg, sel_cgpath);
        if cg_path.is_null() || CGPathIsEmpty(cg_path) {
            return None;
        }

        let bbox = CGPathGetBoundingBox(cg_path);
        if bbox.size.width <= 0.0 || bbox.size.height <= 0.0 {
            return None;
        }
        let target = 19.0_f64;
        let max_dim = bbox.size.width.max(bbox.size.height).max(1.0);
        let scale = target / max_dim;
        let ox = (24.0 - bbox.size.width * scale) * 0.5 - bbox.origin.x * scale;
        let oy = (24.0 - bbox.size.height * scale) * 0.5;
        let mut ctx = PathCtx {
            scale,
            ox,
            oy,
            min_y: bbox.origin.y,
            d: String::with_capacity(512),
        };
        CGPathApply(
            cg_path,
            (&mut ctx as *mut PathCtx).cast::<c_void>(),
            path_applier,
        );
        if ctx.d.is_empty() {
            return None;
        }
        Some(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path fill=\"currentColor\" d=\"{}\"/></svg>",
            ctx.d
        ))
    }
}

pub const BUILTIN_SF_SYMBOLS: &[(&str, &str)] = &[
    (
        "magnifyingglass",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M2.59 10.19C2.59 14.43 6.04 17.87 10.28 17.87C11.95 17.87 13.48 17.33 14.74 16.42L19.49 21.18C19.71 21.40 20.00 21.50 20.30 21.50C20.96 21.50 21.41 21.01 21.41 20.37C21.41 20.06 21.30 19.78 21.10 19.57L16.38 14.84C17.37 13.55 17.97 11.94 17.97 10.19C17.97 5.95 14.52 2.50 10.28 2.50C6.04 2.50 2.59 5.95 2.59 10.19ZM4.24 10.19C4.24 6.85 6.94 4.15 10.28 4.15C13.61 4.15 16.32 6.85 16.32 10.19C16.32 13.52 13.61 16.23 10.28 16.23C6.94 16.23 4.24 13.52 4.24 10.19Z"/></svg>"#,
    ),
    (
        "chevron.left",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M6.65 12.00C6.65 12.27 6.76 12.52 6.96 12.73L15.64 21.21C15.82 21.40 16.08 21.50 16.36 21.50C16.92 21.50 17.35 21.07 17.35 20.50C17.35 20.22 17.24 19.98 17.06 19.79L9.10 12.00L17.06 4.21C17.24 4.02 17.35 3.76 17.35 3.50C17.35 2.93 16.92 2.50 16.36 2.50C16.08 2.50 15.82 2.60 15.64 2.79L6.96 11.27C6.76 11.48 6.65 11.73 6.65 12.00Z"/></svg>"#,
    ),
    (
        "chevron.right",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M17.35 12.00C17.35 11.73 17.25 11.48 17.04 11.27L8.36 2.79C8.18 2.60 7.94 2.50 7.66 2.50C7.09 2.50 6.65 2.93 6.65 3.50C6.65 3.76 6.77 4.02 6.94 4.21L14.91 12.00L6.94 19.79C6.77 19.98 6.65 20.22 6.65 20.50C6.65 21.07 7.09 21.50 7.66 21.50C7.94 21.50 8.18 21.40 8.36 21.21L17.04 12.73C17.25 12.52 17.35 12.27 17.35 12.00Z"/></svg>"#,
    ),
    (
        "rectangle.lefthalf.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08ZM12.02 18.12L12.02 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12Z"/></svg>"#,
    ),
    (
        "rectangle.righthalf.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08ZM3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L12.02 5.88L12.02 18.12L5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86Z"/></svg>"#,
    ),
    (
        "rectangle.tophalf.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08ZM3.80 16.86L3.80 11.99L20.20 11.99L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12L5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86Z"/></svg>"#,
    ),
    (
        "rectangle.bottomhalf.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08ZM3.80 11.99L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 11.99Z"/></svg>"#,
    ),
    (
        "rectangle.inset.topleading.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08L2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42ZM5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12ZM5.60 11.99L11.29 11.99C11.80 11.99 12.00 11.78 12.00 11.27L12.00 7.71C12.00 7.18 11.80 6.98 11.29 6.98L5.60 6.98C5.10 6.98 4.90 7.18 4.90 7.71L4.90 11.27C4.90 11.78 5.10 11.99 5.60 11.99Z"/></svg>"#,
    ),
    (
        "rectangle.inset.toptrailing.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08L2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42ZM5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12ZM12.71 11.99L18.40 11.99C18.90 11.99 19.10 11.78 19.10 11.27L19.10 7.71C19.10 7.18 18.90 6.98 18.40 6.98L12.71 6.98C12.20 6.98 12.00 7.18 12.00 7.71L12.00 11.27C12.00 11.78 12.20 11.99 12.71 11.99Z"/></svg>"#,
    ),
    (
        "rectangle.inset.bottomleading.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08L2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42ZM5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12ZM5.60 17.03L11.29 17.03C11.80 17.03 12.00 16.81 12.00 16.29L12.00 12.71C12.00 12.20 11.80 11.99 11.29 11.99L5.60 11.99C5.10 11.99 4.90 12.20 4.90 12.71L4.90 16.29C4.90 16.81 5.10 17.03 5.60 17.03Z"/></svg>"#,
    ),
    (
        "rectangle.inset.bottomtrailing.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08L2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42ZM5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12ZM12.71 17.03L18.40 17.03C18.90 17.03 19.10 16.81 19.10 16.29L19.10 12.71C19.10 12.20 18.90 11.99 18.40 11.99L12.71 11.99C12.20 11.99 12.00 12.20 12.00 12.71L12.00 16.29C12.00 16.81 12.20 17.03 12.71 17.03Z"/></svg>"#,
    ),
    (
        "arrow.up.left.and.arrow.down.right",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4.34 8.79L4.05 4.03L8.82 4.33L9.65 4.33C10.17 4.33 10.59 3.95 10.59 3.42C10.59 2.88 10.18 2.50 9.65 2.50L4.12 2.50C3.11 2.50 2.52 3.09 2.52 4.10L2.52 9.63C2.52 10.15 2.90 10.55 3.43 10.55C3.97 10.55 4.34 10.16 4.34 9.63ZM4.63 3.47C3.76 2.66 2.69 3.77 3.49 4.62L6.76 8.08L9.82 11.15C9.99 11.34 10.21 11.42 10.46 11.42C11.02 11.42 11.44 11.06 11.44 10.49C11.44 10.23 11.35 9.99 11.16 9.81L8.09 6.74ZM19.66 15.21L19.95 19.96L15.19 19.67L14.36 19.67C13.83 19.67 13.41 20.05 13.41 20.58C13.41 21.12 13.82 21.50 14.36 21.50L19.88 21.50C20.90 21.50 21.48 20.91 21.48 19.90L21.48 14.37C21.48 13.85 21.10 13.45 20.57 13.45C20.04 13.45 19.66 13.84 19.66 14.37ZM19.37 20.53C20.25 21.34 21.31 20.23 20.52 19.38L17.25 15.92L14.19 12.84C14.02 12.66 13.79 12.57 13.54 12.57C12.98 12.57 12.56 12.94 12.56 13.51C12.56 13.77 12.66 14.01 12.84 14.19L15.91 17.26Z"/></svg>"#,
    ),
    (
        "rectangle.inset.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08L2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42ZM5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12ZM5.60 17.03L18.40 17.03C18.85 17.03 19.10 16.76 19.10 16.29L19.10 7.71C19.10 7.25 18.85 6.98 18.40 6.98L5.60 6.98C5.16 6.98 4.90 7.25 4.90 7.71L4.90 16.29C4.90 16.76 5.16 17.03 5.60 17.03Z"/></svg>"#,
    ),
    (
        "macwindow",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.90 19.42L18.30 19.42C20.33 19.42 21.50 18.19 21.50 16.01L21.50 7.99C21.50 5.81 20.33 4.58 18.30 4.58L5.90 4.58C3.73 4.58 2.50 5.81 2.50 7.99L2.50 16.01C2.50 18.19 3.73 19.42 5.90 19.42ZM5.90 18.12C4.57 18.12 3.80 17.35 3.80 16.01L3.80 7.99C3.80 6.65 4.57 5.88 5.90 5.88L18.10 5.88C19.44 5.88 20.20 6.65 20.20 7.99L20.20 16.01C20.20 17.35 19.44 18.12 18.10 18.12ZM10.70 8.82C11.09 8.82 11.42 8.49 11.42 8.11C11.42 7.72 11.09 7.39 10.70 7.39C10.30 7.39 9.98 7.72 9.98 8.11C9.98 8.49 10.30 8.82 10.70 8.82ZM8.38 8.82C8.78 8.82 9.10 8.49 9.10 8.11C9.10 7.72 8.78 7.39 8.38 7.39C7.99 7.39 7.66 7.72 7.66 8.11C7.66 8.49 7.99 8.82 8.38 8.82ZM6.07 8.82C6.47 8.82 6.79 8.49 6.79 8.11C6.79 7.72 6.47 7.39 6.07 7.39C5.67 7.39 5.35 7.72 5.35 8.11C5.35 8.49 5.67 8.82 6.07 8.82Z"/></svg>"#,
    ),
    (
        "rectangle.center.inset.filled",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08L2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42ZM5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12ZM7.59 15.68L16.41 15.68C16.98 15.68 17.22 15.44 17.22 14.86L17.22 9.13C17.22 8.56 16.98 8.32 16.41 8.32L7.59 8.32C7.02 8.32 6.79 8.56 6.79 9.13L6.79 14.86C6.79 15.44 7.02 15.68 7.59 15.68Z"/></svg>"#,
    ),
    (
        "arrow.uturn.backward",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M2.50 9.37C2.50 9.62 2.60 9.85 2.80 10.04L8.71 15.85C8.89 16.02 9.14 16.13 9.37 16.13C9.90 16.13 10.25 15.76 10.25 15.24C10.25 14.98 10.17 14.77 10.00 14.62L7.11 11.79L4.27 9.37L7.11 6.94L10.00 4.12C10.17 3.95 10.25 3.76 10.25 3.50C10.25 2.98 9.90 2.61 9.37 2.61C9.14 2.61 8.89 2.72 8.71 2.89L2.80 8.70C2.60 8.89 2.50 9.12 2.50 9.37ZM11.24 20.49C11.24 20.98 11.60 21.39 12.15 21.39L14.47 21.39C18.97 21.39 21.50 18.81 21.50 14.97C21.50 11.12 18.91 8.47 14.31 8.47L7.57 8.47L4.23 8.62C3.82 8.64 3.48 8.96 3.48 9.37C3.48 9.78 3.82 10.10 4.23 10.12L7.57 10.27L14.47 10.27C17.89 10.27 19.71 12.16 19.71 14.86C19.71 17.57 17.89 19.59 14.47 19.59L12.15 19.59C11.60 19.59 11.24 20.00 11.24 20.49Z"/></svg>"#,
    ),
    (
        "door.left.hand.open",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.18 20.78C5.18 21.17 5.50 21.50 5.90 21.50C6.30 21.50 6.62 21.17 6.62 20.78L6.62 4.42C6.62 4.13 6.80 3.94 7.07 3.94L16.92 3.94C17.20 3.94 17.38 4.13 17.38 4.42L17.38 20.78C17.38 21.17 17.70 21.50 18.10 21.50C18.50 21.50 18.82 21.17 18.82 20.78L18.82 4.33C18.82 3.24 18.07 2.50 16.95 2.50L7.05 2.50C5.93 2.50 5.18 3.24 5.18 4.33ZM7.66 20.98C7.66 21.15 7.81 21.25 7.98 21.17L10.78 19.93C11.02 19.83 11.10 19.76 11.10 19.53L11.10 6.63C11.10 6.40 11.01 6.33 10.80 6.23L7.98 4.99C7.81 4.91 7.66 5.00 7.66 5.18Z"/></svg>"#,
    ),
    (
        "square.grid.2x2",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M14.83 21.50L19.52 21.50C20.84 21.50 21.49 20.84 21.49 19.48L21.49 14.87C21.49 13.51 20.84 12.85 19.52 12.85L14.83 12.85C13.52 12.85 12.85 13.51 12.85 14.87L12.85 19.48C12.85 20.84 13.52 21.50 14.83 21.50ZM14.86 20.04C14.49 20.04 14.31 19.85 14.31 19.49L14.31 14.87C14.31 14.49 14.49 14.31 14.86 14.31L19.50 14.31C19.87 14.31 20.04 14.49 20.04 14.87L20.04 19.49C20.04 19.85 19.87 20.04 19.50 20.04ZM4.48 21.50L9.17 21.50C10.48 21.50 11.15 20.84 11.15 19.48L11.15 14.87C11.15 13.51 10.48 12.85 9.17 12.85L4.48 12.85C3.16 12.85 2.51 13.51 2.51 14.87L2.51 19.48C2.51 20.84 3.16 21.50 4.48 21.50ZM4.50 20.04C4.14 20.04 3.96 19.85 3.96 19.49L3.96 14.87C3.96 14.49 4.14 14.31 4.50 14.31L9.14 14.31C9.51 14.31 9.69 14.49 9.69 14.87L9.69 19.49C9.69 19.85 9.51 20.04 9.14 20.04ZM14.83 11.14L19.52 11.14C20.84 11.14 21.49 10.49 21.49 9.13L21.49 4.52C21.49 3.16 20.84 2.50 19.52 2.50L14.83 2.50C13.52 2.50 12.85 3.16 12.85 4.52L12.85 9.13C12.85 10.49 13.52 11.14 14.83 11.14ZM14.86 9.69C14.49 9.69 14.31 9.51 14.31 9.13L14.31 4.51C14.31 4.15 14.49 3.96 14.86 3.96L19.50 3.96C19.87 3.96 20.04 4.15 20.04 4.51L20.04 9.13C20.04 9.51 19.87 9.69 19.50 9.69ZM4.48 11.14L9.17 11.14C10.48 11.14 11.15 10.49 11.15 9.13L11.15 4.52C11.15 3.16 10.48 2.50 9.17 2.50L4.48 2.50C3.16 2.50 2.51 3.16 2.51 4.52L2.51 9.13C2.51 10.49 3.16 11.14 4.48 11.14ZM4.50 9.69C4.14 9.69 3.96 9.51 3.96 9.13L3.96 4.51C3.96 4.15 4.14 3.96 4.50 3.96L9.14 3.96C9.51 3.96 9.69 4.15 9.69 4.51L9.69 9.13C9.69 9.51 9.51 9.69 9.14 9.69Z"/></svg>"#,
    ),
    (
        "arrow.left.and.right.square",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.74 21.50L18.26 21.50C20.42 21.50 21.49 20.42 21.49 18.30L21.49 5.71C21.49 3.58 20.42 2.50 18.26 2.50L5.74 2.50C3.59 2.50 2.51 3.57 2.51 5.71L2.51 18.30C2.51 20.43 3.59 21.50 5.74 21.50ZM5.77 19.83C4.74 19.83 4.16 19.28 4.16 18.21L4.16 5.79C4.16 4.71 4.74 4.17 5.77 4.17L18.23 4.17C19.25 4.17 19.84 4.71 19.84 5.79L19.84 18.21C19.84 19.28 19.25 19.83 18.23 19.83ZM5.60 12.02C5.60 12.25 5.70 12.44 5.90 12.63L8.86 15.61C9.00 15.75 9.19 15.83 9.41 15.83C9.83 15.83 10.15 15.50 10.15 15.07C10.15 14.85 10.07 14.67 9.92 14.52L9.14 13.76L7.01 12.02L9.14 10.28L9.92 9.52C10.07 9.38 10.15 9.20 10.15 8.96C10.15 8.55 9.83 8.21 9.41 8.21C9.19 8.21 8.99 8.31 8.86 8.43L5.90 11.41C5.70 11.60 5.60 11.79 5.60 12.02ZM17.74 12.02C17.74 11.64 17.44 11.35 17.05 11.34L13.93 11.24L10.07 11.24L6.95 11.34C6.56 11.35 6.26 11.64 6.26 12.02C6.26 12.40 6.56 12.69 6.95 12.70L10.07 12.79L13.93 12.79L17.05 12.70C17.44 12.69 17.74 12.40 17.74 12.02ZM18.41 12.02C18.41 11.79 18.30 11.60 18.12 11.41L15.14 8.43C15.01 8.31 14.81 8.21 14.60 8.21C14.17 8.21 13.85 8.55 13.85 8.96C13.85 9.20 13.93 9.38 14.09 9.52L14.86 10.28L17.00 12.02L14.86 13.76L14.09 14.52C13.93 14.67 13.85 14.85 13.85 15.07C13.85 15.50 14.17 15.83 14.60 15.83C14.81 15.83 15.00 15.75 15.14 15.61L18.12 12.63C18.30 12.44 18.41 12.25 18.41 12.02Z"/></svg>"#,
    ),
    (
        "arrow.left",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M10.28 19.76C10.78 19.76 11.17 19.38 11.17 18.87C11.17 18.63 11.09 18.39 10.92 18.23L8.64 15.90L4.36 12.00L8.64 8.10L10.92 5.77C11.09 5.61 11.17 5.37 11.17 5.13C11.17 4.62 10.78 4.24 10.28 4.24C10.04 4.24 9.81 4.34 9.59 4.55L2.82 11.32C2.61 11.51 2.50 11.75 2.50 12.00C2.50 12.25 2.61 12.49 2.82 12.68L9.61 19.47C9.82 19.66 10.04 19.76 10.28 19.76ZM7.66 12.92L20.59 12.92C21.12 12.92 21.50 12.53 21.50 12.00C21.50 11.47 21.12 11.08 20.59 11.08L7.66 11.08L4.36 11.29C3.94 11.29 3.65 11.58 3.65 12.00C3.65 12.42 3.94 12.71 4.37 12.71Z"/></svg>"#,
    ),
    (
        "arrow.right",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M13.72 19.76C13.96 19.76 14.18 19.66 14.39 19.47L21.18 12.68C21.39 12.49 21.50 12.25 21.50 12.00C21.50 11.75 21.39 11.51 21.18 11.32L14.41 4.55C14.19 4.34 13.96 4.24 13.72 4.24C13.22 4.24 12.83 4.62 12.83 5.13C12.83 5.37 12.91 5.61 13.08 5.77L15.36 8.10L19.64 12.00L15.36 15.90L13.08 18.23C12.91 18.39 12.83 18.63 12.83 18.87C12.83 19.38 13.22 19.76 13.72 19.76ZM3.41 12.92L16.34 12.92L19.63 12.71C20.06 12.68 20.35 12.42 20.35 12.00C20.35 11.58 20.06 11.32 19.64 11.29L16.34 11.08L3.41 11.08C2.88 11.08 2.50 11.47 2.50 12.00C2.50 12.53 2.88 12.92 3.41 12.92Z"/></svg>"#,
    ),
    (
        "arrow.up",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.25 11.03C5.48 11.03 5.72 10.93 5.88 10.78L8.17 8.53L11.99 4.34L15.83 8.53L18.12 10.78C18.28 10.93 18.52 11.03 18.75 11.03C19.25 11.03 19.62 10.64 19.62 10.14C19.62 9.91 19.53 9.69 19.34 9.49L12.67 2.81C12.48 2.61 12.25 2.50 12.00 2.50C11.75 2.50 11.52 2.61 11.33 2.81L4.66 9.49C4.47 9.69 4.38 9.91 4.38 10.14C4.38 10.64 4.75 11.03 5.25 11.03ZM12.00 21.50C12.52 21.50 12.90 21.13 12.90 20.61L12.90 7.37L12.79 4.38C12.79 3.91 12.47 3.59 12.00 3.59C11.53 3.59 11.21 3.91 11.21 4.38L11.10 7.37L11.10 20.61C11.10 21.13 11.48 21.50 12.00 21.50Z"/></svg>"#,
    ),
    (
        "arrow.down",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12.00 21.50C12.25 21.50 12.48 21.39 12.67 21.19L19.34 14.51C19.53 14.31 19.62 14.09 19.62 13.86C19.62 13.36 19.25 12.97 18.75 12.97C18.52 12.97 18.28 13.07 18.12 13.22L15.83 15.47L11.99 19.66L8.17 15.47L5.88 13.22C5.72 13.07 5.48 12.97 5.25 12.97C4.75 12.97 4.38 13.36 4.38 13.86C4.38 14.09 4.47 14.31 4.66 14.51L11.33 21.19C11.52 21.39 11.75 21.50 12.00 21.50ZM12.00 20.41C12.47 20.41 12.79 20.09 12.79 19.62L12.90 16.63L12.90 3.39C12.90 2.87 12.52 2.50 12.00 2.50C11.48 2.50 11.10 2.87 11.10 3.39L11.10 16.63L11.21 19.62C11.21 20.09 11.53 20.41 12.00 20.41Z"/></svg>"#,
    ),
    (
        "doc.on.doc",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M7.94 4.87L7.94 15.36C7.94 16.94 8.72 17.73 10.28 17.73L17.38 17.73C18.94 17.73 19.72 16.94 19.72 15.36L19.72 9.23C19.72 8.31 19.53 7.71 18.97 7.13L15.15 3.25C14.62 2.70 13.99 2.50 13.18 2.50L10.28 2.50C8.72 2.50 7.94 3.29 7.94 4.87ZM9.15 15.34L9.15 4.89C9.15 4.14 9.56 3.72 10.34 3.72L13.68 3.72L13.68 7.59C13.68 8.46 14.11 8.88 14.97 8.88L18.50 8.88L18.50 15.34C18.50 16.09 18.10 16.51 17.32 16.51L10.34 16.51C9.56 16.51 9.15 16.09 9.15 15.34ZM14.77 7.41L14.77 4.31L18.19 7.79L15.15 7.79C14.88 7.79 14.77 7.68 14.77 7.41ZM4.28 19.13C4.28 20.71 5.06 21.50 6.62 21.50L13.72 21.50C15.27 21.50 16.06 20.71 16.06 19.13L16.06 13.23C16.06 12.26 15.95 11.84 15.35 11.22L11.18 6.99C10.61 6.40 10.13 6.27 9.28 6.27L6.62 6.27C5.06 6.27 4.28 7.06 4.28 8.64ZM4.28 19.13C4.28 20.71 5.06 21.50 6.62 21.50L13.72 21.50C15.27 21.50 16.06 20.71 16.06 19.13L16.06 13.23C16.06 12.26 15.95 11.84 15.35 11.22L11.18 6.99C10.61 6.40 10.13 6.27 9.28 6.27L6.62 6.27C5.06 6.27 4.28 7.06 4.28 8.64ZM5.49 19.11L5.49 8.66C5.49 7.91 5.90 7.48 6.68 7.48L9.16 7.48L9.16 11.86C9.16 12.82 9.64 13.29 10.58 13.29L14.84 13.29L14.84 19.11C14.84 19.86 14.44 20.28 13.66 20.28L6.67 20.28C5.90 20.28 5.49 19.86 5.49 19.11ZM10.72 12.15C10.42 12.15 10.29 12.03 10.29 11.73L10.29 7.76L14.61 12.15Z"/></svg>"#,
    ),
    (
        "doc.text.magnifyingglass",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4.32 16.74C4.32 18.36 5.12 19.18 6.73 19.18L14.99 19.18C16.59 19.18 17.40 18.37 17.40 16.74L17.40 4.94C17.40 3.31 16.59 2.50 14.99 2.50L6.73 2.50C5.12 2.50 4.32 3.32 4.32 4.94ZM16.14 4.96L16.14 16.72C16.14 17.49 15.70 17.93 14.93 17.93L6.79 17.93C5.98 17.93 5.57 17.49 5.57 16.72L5.57 4.96C5.57 4.19 5.98 3.75 6.79 3.75L14.93 3.75C15.70 3.75 16.14 4.18 16.14 4.96ZM7.92 9.63L13.81 9.63C14.09 9.63 14.29 9.42 14.29 9.15C14.29 8.88 14.09 8.68 13.81 8.68L7.92 8.68C7.63 8.68 7.43 8.88 7.43 9.15C7.43 9.42 7.63 9.63 7.92 9.63ZM7.92 6.92L13.81 6.92C14.09 6.92 14.29 6.71 14.29 6.44C14.29 6.17 14.09 5.97 13.81 5.97L7.92 5.97C7.63 5.97 7.43 6.17 7.43 6.44C7.43 6.71 7.63 6.92 7.92 6.92ZM13.88 19.84C14.38 19.84 14.87 19.75 15.32 19.57L16.70 20.95C17.05 21.30 17.49 21.50 17.99 21.50C18.94 21.50 19.68 20.74 19.68 19.77C19.68 19.32 19.47 18.88 19.16 18.56L17.78 17.17C17.97 16.70 18.08 16.18 18.08 15.64C18.08 13.34 16.19 11.44 13.88 11.44C11.58 11.44 9.68 13.34 9.68 15.64C9.68 17.96 11.58 19.84 13.88 19.84ZM13.88 18.79C15.61 18.79 17.02 17.37 17.02 15.64C17.02 13.92 15.61 12.50 13.88 12.50C12.16 12.50 10.75 13.92 10.75 15.64C10.75 17.37 12.16 18.79 13.88 18.79ZM13.88 17.74C12.74 17.74 11.79 16.79 11.79 15.64C11.79 14.50 12.74 13.56 13.88 13.56C15.03 13.56 15.98 14.50 15.98 15.64C15.98 16.79 15.03 17.74 13.88 17.74ZM17.99 20.44C18.36 20.44 18.62 20.16 18.62 19.77C18.62 19.61 18.53 19.44 18.40 19.32L16.37 17.27L15.43 18.18L17.45 20.19C17.61 20.35 17.77 20.44 17.99 20.44Z"/></svg>"#,
    ),
    (
        "link",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12.11 8.31L10.93 9.52C12.11 9.61 12.87 9.97 13.45 10.56C15.01 12.12 15.01 14.33 13.46 15.87L10.54 18.79C8.99 20.34 6.78 20.35 5.22 18.80C3.66 17.23 3.67 15.03 5.22 13.48L6.97 11.73C6.73 11.16 6.66 10.49 6.75 9.91L4.14 12.53C1.99 14.66 1.97 17.71 4.15 19.88C6.31 22.05 9.36 22.03 11.49 19.89L14.55 16.82C16.68 14.69 16.71 11.65 14.54 9.47C13.97 8.91 13.26 8.52 12.11 8.31ZM11.89 15.68L13.08 14.48C11.90 14.39 11.14 14.03 10.55 13.44C8.99 11.88 8.99 9.67 10.54 8.13L13.46 5.21C15.01 3.66 17.22 3.64 18.78 5.20C20.35 6.77 20.33 8.97 18.78 10.52L17.03 12.27C17.27 12.84 17.34 13.51 17.25 14.09L19.86 11.47C22.01 9.33 22.03 6.29 19.85 4.12C17.69 1.95 14.65 1.97 12.51 4.11L9.45 7.18C7.32 9.32 7.30 12.35 9.46 14.53C10.03 15.09 10.74 15.48 11.89 15.68Z"/></svg>"#,
    ),
    (
        "folder",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.13 19.66L19.13 19.66C20.63 19.66 21.50 18.78 21.50 17.06L21.50 8.47C21.50 6.74 20.62 5.87 18.87 5.87L10.84 5.87C10.27 5.87 9.93 5.74 9.52 5.38L9.00 4.96C8.45 4.49 8.02 4.34 7.20 4.34L4.81 4.34C3.35 4.34 2.50 5.17 2.50 6.86L2.50 17.06C2.50 18.79 3.38 19.66 5.13 19.66ZM5.15 18.31C4.31 18.31 3.85 17.87 3.85 16.99L3.85 6.93C3.85 6.10 4.28 5.68 5.09 5.68L6.85 5.68C7.41 5.68 7.74 5.82 8.17 6.18L8.68 6.61C9.22 7.06 9.67 7.22 10.49 7.22L18.85 7.22C19.68 7.22 20.15 7.67 20.15 8.54L20.15 17.00C20.15 17.87 19.68 18.31 18.85 18.31ZM4.88 9.80L19.11 9.80L19.11 9.28C19.11 8.76 18.85 8.50 18.29 8.50L5.71 8.50C5.16 8.50 4.88 8.76 4.88 9.28Z"/></svg>"#,
    ),
    (
        "terminal",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.03 19.42L18.97 19.42C20.66 19.42 21.50 18.58 21.50 16.92L21.50 7.08C21.50 5.42 20.66 4.58 18.97 4.58L5.03 4.58C3.35 4.58 2.50 5.42 2.50 7.08L2.50 16.92C2.50 18.58 3.35 19.42 5.03 19.42ZM5.05 18.12C4.24 18.12 3.80 17.69 3.80 16.86L3.80 7.14C3.80 6.31 4.24 5.88 5.05 5.88L18.95 5.88C19.75 5.88 20.20 6.31 20.20 7.14L20.20 16.86C20.20 17.69 19.75 18.12 18.95 18.12ZM5.97 10.80C5.34 11.19 5.87 12.15 6.56 11.71L8.88 10.25C9.34 9.98 9.32 9.29 8.88 9.01L6.56 7.57C5.87 7.12 5.33 8.07 5.97 8.46L7.90 9.64ZM9.37 11.70C9.37 11.99 9.60 12.23 9.89 12.23L12.95 12.23C13.25 12.23 13.47 11.99 13.47 11.70C13.47 11.41 13.25 11.18 12.95 11.18L9.89 11.18C9.60 11.18 9.37 11.41 9.37 11.70Z"/></svg>"#,
    ),
    (
        "globe",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M11.99 21.49C17.24 21.49 21.50 17.25 21.50 12.00C21.50 6.75 17.24 2.51 11.99 2.51C6.76 2.51 2.50 6.75 2.50 12.00C2.50 17.25 6.76 21.49 11.99 21.49ZM11.99 20.23C7.45 20.23 3.77 16.54 3.77 12.00C3.77 7.46 7.45 3.77 11.99 3.77C16.54 3.77 20.23 7.46 20.23 12.00C20.23 16.54 16.54 20.23 11.99 20.23ZM11.99 20.98C14.58 20.98 16.70 17.15 16.70 12.02C16.70 6.87 14.59 3.03 11.99 3.03C9.40 3.03 7.30 6.87 7.30 12.02C7.30 17.15 9.41 20.98 11.99 20.98ZM11.99 19.85C10.18 19.85 8.55 16.17 8.55 12.02C8.55 7.80 10.18 4.16 11.99 4.16C13.82 4.16 15.45 7.80 15.45 12.02C15.45 16.17 13.82 19.85 11.99 19.85ZM11.99 21.22C12.33 21.22 12.61 20.95 12.61 20.61L12.61 3.47C12.61 3.13 12.33 2.85 11.99 2.85C11.67 2.85 11.38 3.13 11.38 3.47L11.38 20.61C11.38 20.95 11.67 21.22 11.99 21.22ZM6.21 18.25C7.45 17.26 9.46 16.72 11.99 16.72C14.54 16.72 16.54 17.26 17.79 18.25C18.05 18.46 18.41 18.48 18.65 18.25C18.89 18.02 18.91 17.64 18.65 17.40C17.40 16.27 14.87 15.50 11.99 15.50C9.13 15.50 6.60 16.27 5.35 17.40C5.09 17.64 5.11 18.02 5.35 18.25C5.59 18.48 5.94 18.46 6.21 18.25ZM3.62 12.61L20.57 12.61C20.91 12.61 21.18 12.34 21.18 12.00C21.18 11.66 20.91 11.39 20.57 11.39L3.62 11.39C3.28 11.39 3.01 11.66 3.01 12.00C3.01 12.34 3.28 12.61 3.62 12.61ZM11.99 8.55C14.87 8.55 17.40 7.77 18.65 6.65C18.91 6.41 18.89 6.03 18.65 5.80C18.41 5.56 18.05 5.59 17.79 5.80C16.54 6.79 14.54 7.32 11.99 7.32C9.46 7.32 7.45 6.79 6.21 5.80C5.94 5.59 5.59 5.56 5.35 5.80C5.11 6.03 5.09 6.41 5.35 6.65C6.60 7.77 9.13 8.55 11.99 8.55Z"/></svg>"#,
    ),
    (
        "curlybraces",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M3.44 11.99C3.44 12.52 3.75 12.89 4.39 13.00C5.52 13.18 5.97 13.76 5.97 15.23L5.97 17.82C5.97 20.31 7.11 21.50 9.59 21.50C9.74 21.50 9.89 21.47 10.01 21.43C10.29 21.32 10.44 21.08 10.44 20.77C10.44 20.43 10.29 20.23 9.97 20.15C9.87 20.13 9.79 20.11 9.68 20.10C8.13 20.00 7.57 19.33 7.57 17.63L7.57 14.59C7.57 13.02 6.75 12.16 5.33 12.02C5.29 12.02 5.29 11.99 5.33 11.99C6.75 11.85 7.57 10.99 7.57 9.43L7.57 6.36C7.57 4.66 8.13 4.01 9.68 3.91C9.82 3.90 9.95 3.88 10.04 3.84C10.31 3.75 10.44 3.54 10.44 3.22C10.44 2.90 10.29 2.67 9.97 2.57C9.83 2.52 9.69 2.50 9.51 2.50C7.10 2.50 5.97 3.70 5.97 6.19L5.97 8.78C5.97 10.24 5.52 10.82 4.39 11.00C3.75 11.11 3.44 11.47 3.44 11.99ZM20.56 11.99C20.56 11.47 20.26 11.11 19.62 11.00C18.49 10.82 18.03 10.24 18.03 8.78L18.03 6.19C18.03 3.70 16.90 2.50 14.49 2.50C14.31 2.50 14.17 2.52 14.03 2.57C13.71 2.67 13.55 2.90 13.55 3.22C13.55 3.54 13.71 3.74 13.96 3.84C14.06 3.88 14.18 3.90 14.32 3.91C15.86 3.97 16.44 4.66 16.44 6.36L16.44 9.43C16.44 11.00 17.26 11.99 18.68 11.99C18.71 11.99 18.71 12.02 18.68 12.02C17.26 12.02 16.44 13.01 16.44 14.59L16.44 17.63C16.44 19.33 15.87 20.00 14.32 20.10C14.21 20.11 14.12 20.13 14.03 20.15C13.71 20.21 13.55 20.43 13.55 20.77C13.55 21.10 13.71 21.34 14.00 21.43C14.11 21.47 14.26 21.50 14.41 21.50C16.89 21.50 18.03 20.31 18.03 17.82L18.03 15.23C18.03 13.76 18.49 13.18 19.62 13.00C20.26 12.89 20.56 12.52 20.56 11.99Z"/></svg>"#,
    ),
    (
        "plus.forwardslash.minus",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M2.50 7.55C2.50 7.95 2.82 8.27 3.21 8.27L5.25 8.27L5.25 10.30C5.25 10.71 5.57 11.01 5.96 11.01C6.37 11.01 6.67 10.70 6.67 10.30L6.67 8.27L8.72 8.27C9.11 8.27 9.43 7.95 9.43 7.55C9.43 7.16 9.11 6.84 8.72 6.84L6.67 6.84L6.67 4.80C6.67 4.40 6.35 4.08 5.96 4.08C5.56 4.08 5.25 4.40 5.25 4.80L5.25 6.84L3.21 6.84C2.82 6.84 2.50 7.16 2.50 7.55ZM12.70 12.41L18.52 3.79C18.66 3.60 18.72 3.43 18.72 3.23C18.72 2.82 18.36 2.52 17.95 2.52C17.60 2.52 17.41 2.63 17.12 3.04L11.36 11.60L5.53 20.19C5.39 20.38 5.32 20.55 5.32 20.76C5.32 21.20 5.68 21.48 6.13 21.48C6.46 21.48 6.65 21.35 6.91 20.97ZM14.57 17.14C14.57 17.53 14.89 17.84 15.28 17.84L20.79 17.84C21.19 17.84 21.50 17.53 21.50 17.14C21.50 16.75 21.19 16.43 20.79 16.43L15.28 16.43C14.89 16.43 14.57 16.75 14.57 17.14Z"/></svg>"#,
    ),
    (
        "sparkles",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M11.52 6.56C11.63 6.56 11.69 6.49 11.71 6.39C11.96 5.04 11.94 4.98 13.39 4.72C13.49 4.70 13.56 4.64 13.56 4.53C13.56 4.43 13.49 4.36 13.39 4.34C11.94 4.09 11.96 4.02 11.71 2.67C11.69 2.57 11.63 2.50 11.52 2.50C11.42 2.50 11.37 2.57 11.34 2.67C11.09 4.02 11.12 4.09 9.67 4.34C9.57 4.36 9.50 4.43 9.50 4.53C9.50 4.64 9.57 4.70 9.67 4.72C11.12 4.98 11.09 5.04 11.34 6.39C11.37 6.49 11.42 6.56 11.52 6.56ZM7.50 12.29C7.65 12.29 7.76 12.19 7.78 12.03C8.08 9.81 8.15 9.82 10.45 9.37C10.60 9.35 10.71 9.24 10.71 9.09C10.71 8.93 10.60 8.83 10.45 8.81C8.15 8.48 8.07 8.41 7.78 6.15C7.76 5.99 7.65 5.89 7.50 5.89C7.34 5.89 7.24 5.99 7.22 6.16C6.95 8.39 6.83 8.37 4.55 8.81C4.41 8.84 4.29 8.93 4.29 9.09C4.29 9.25 4.41 9.35 4.58 9.37C6.84 9.74 6.95 9.79 7.22 12.02C7.24 12.19 7.34 12.29 7.50 12.29ZM13.15 21.50C13.36 21.50 13.53 21.35 13.56 21.13C14.15 16.57 14.80 15.87 19.31 15.36C19.55 15.34 19.71 15.17 19.71 14.95C19.71 14.73 19.55 14.57 19.31 14.54C14.80 14.04 14.15 13.34 13.56 8.78C13.53 8.55 13.36 8.40 13.15 8.40C12.93 8.40 12.77 8.55 12.74 8.78C12.14 13.34 11.49 14.04 6.98 14.54C6.75 14.57 6.59 14.73 6.59 14.95C6.59 15.17 6.75 15.34 6.98 15.36C11.48 15.96 12.11 16.57 12.74 21.13C12.77 21.35 12.93 21.50 13.15 21.50Z"/></svg>"#,
    ),
    (
        "bolt.fill",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M6.01 13.06C6.01 13.38 6.26 13.63 6.61 13.63L11.43 13.63L8.89 20.52C8.55 21.40 9.47 21.87 10.04 21.15L17.76 11.49C17.91 11.30 17.99 11.14 17.99 10.94C17.99 10.62 17.73 10.37 17.38 10.37L12.57 10.37L15.10 3.48C15.43 2.60 14.53 2.13 13.95 2.85L6.23 12.51C6.08 12.70 6.01 12.86 6.01 13.06Z"/></svg>"#,
    ),
    (
        "return",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M3.41 14.22C3.41 14.60 3.70 14.88 4.08 14.89L7.04 15.03L18.52 15.03C20.62 15.03 21.50 14.06 21.50 12.03L21.50 6.70C21.50 4.60 20.62 3.73 18.52 3.73L13.48 3.73C12.99 3.73 12.67 4.09 12.67 4.54C12.67 4.98 12.98 5.34 13.48 5.34L18.52 5.34C19.48 5.34 19.89 5.75 19.89 6.70L19.89 12.03C19.89 13.00 19.48 13.42 18.52 13.42L7.04 13.42L4.08 13.55C3.70 13.57 3.41 13.85 3.41 14.22ZM2.50 14.22C2.50 14.44 2.59 14.65 2.77 14.83L8.06 20.02C8.22 20.17 8.44 20.27 8.65 20.27C9.13 20.27 9.44 19.94 9.44 19.47C9.44 19.24 9.37 19.07 9.22 18.92L6.63 16.39L4.41 14.49L4.41 13.96L6.63 12.05L9.22 9.52C9.37 9.38 9.44 9.20 9.44 8.97C9.44 8.50 9.13 8.18 8.65 8.18C8.44 8.18 8.22 8.27 8.06 8.43L2.77 13.62C2.59 13.79 2.50 14.00 2.50 14.22Z"/></svg>"#,
    ),
    (
        "delete.left",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M18.75 20.07C20.57 20.07 21.50 19.16 21.50 17.35L21.50 6.65C21.50 4.84 20.57 3.93 18.75 3.93L11.13 3.93C10.09 3.93 9.22 4.20 8.49 4.95L3.33 10.32C2.76 10.91 2.50 11.44 2.50 11.98C2.50 12.53 2.75 13.06 3.33 13.65L8.50 18.99C9.23 19.74 10.10 20.06 11.13 20.06ZM18.73 18.65L11.11 18.65C10.44 18.65 10.02 18.50 9.56 18.03L4.43 12.73C4.13 12.42 4.04 12.21 4.04 11.98C4.04 11.76 4.14 11.55 4.43 11.25L9.55 5.93C10.01 5.45 10.44 5.35 11.10 5.35L18.73 5.35C19.60 5.35 20.08 5.81 20.08 6.72L20.08 17.28C20.08 18.19 19.60 18.65 18.73 18.65ZM11.12 15.48L17.09 9.50C17.23 9.37 17.30 9.20 17.30 9.01C17.30 8.62 16.99 8.33 16.60 8.33C16.41 8.33 16.24 8.39 16.12 8.53L10.12 14.49C9.98 14.63 9.91 14.79 9.91 14.99C9.91 15.38 10.22 15.70 10.62 15.70C10.82 15.70 10.98 15.62 11.12 15.48ZM16.10 15.48C16.22 15.62 16.40 15.70 16.60 15.70C16.99 15.70 17.30 15.38 17.30 14.99C17.30 14.79 17.23 14.63 17.09 14.49L11.10 8.53C10.96 8.39 10.81 8.33 10.62 8.33C10.22 8.33 9.91 8.62 9.91 9.01C9.91 9.20 9.98 9.37 10.12 9.50Z"/></svg>"#,
    ),
    (
        "command",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M6.09 21.48C8.07 21.48 9.68 19.87 9.68 17.88L9.68 6.12C9.68 4.13 8.07 2.52 6.09 2.52C4.11 2.52 2.50 4.13 2.50 6.12C2.50 8.12 4.11 9.69 6.09 9.69L8.91 9.69C9.32 9.69 9.66 9.35 9.66 8.92C9.66 8.50 9.32 8.17 8.91 8.17L6.09 8.17C4.99 8.17 4.06 7.24 4.06 6.12C4.06 5.01 4.99 4.10 6.09 4.10C7.20 4.10 8.12 5.01 8.12 6.12L8.12 17.88C8.12 18.99 7.20 19.90 6.09 19.90C4.99 19.90 4.06 18.99 4.06 17.88C4.06 16.76 4.99 15.83 6.09 15.83L17.91 15.83C19.01 15.83 19.94 16.76 19.94 17.88C19.94 18.99 19.01 19.90 17.91 19.90C16.80 19.90 15.89 18.99 15.89 17.88L15.89 6.12C15.89 5.01 16.80 4.10 17.91 4.10C19.01 4.10 19.94 5.01 19.94 6.12C19.94 7.24 19.01 8.17 17.91 8.17L8.91 8.17C8.48 8.17 8.14 8.50 8.14 8.92C8.14 9.35 8.48 9.69 8.91 9.69L17.91 9.69C19.89 9.69 21.50 8.12 21.50 6.12C21.50 4.13 19.89 2.52 17.91 2.52C15.93 2.52 14.32 4.13 14.32 6.12L14.32 17.88C14.32 19.87 15.93 21.48 17.91 21.48C19.89 21.48 21.50 19.87 21.50 17.88C21.50 15.88 19.89 14.32 17.91 14.32L6.09 14.32C4.11 14.32 2.50 15.88 2.50 17.88C2.50 19.87 4.11 21.48 6.09 21.48Z"/></svg>"#,
    ),
    (
        "equal",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M3.73 8.90L20.27 8.90C20.93 8.90 21.50 8.36 21.50 7.69C21.50 7.01 20.93 6.47 20.27 6.47L3.73 6.47C3.06 6.47 2.50 7.01 2.50 7.69C2.50 8.36 3.06 8.90 3.73 8.90ZM3.73 17.53L20.27 17.53C20.93 17.53 21.50 16.99 21.50 16.33C21.50 15.64 20.93 15.10 20.27 15.10L3.73 15.10C3.06 15.10 2.50 15.64 2.50 16.33C2.50 16.99 3.06 17.53 3.73 17.53Z"/></svg>"#,
    ),
    (
        "checkmark",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M9.69 21.42C10.16 21.42 10.54 21.22 10.80 20.81L21.22 4.41C21.43 4.09 21.50 3.84 21.50 3.58C21.50 2.99 21.10 2.58 20.49 2.58C20.06 2.58 19.81 2.73 19.55 3.15L9.64 18.92L4.51 12.19C4.23 11.82 3.96 11.66 3.55 11.66C2.94 11.66 2.50 12.09 2.50 12.70C2.50 12.95 2.61 13.23 2.82 13.50L8.54 20.78C8.88 21.22 9.22 21.42 9.69 21.42Z"/></svg>"#,
    ),
    (
        "xmark",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M19.67 2.81L2.81 19.67C2.40 20.07 2.39 20.77 2.81 21.19C3.24 21.60 3.93 21.60 4.33 21.19L21.20 4.33C21.60 3.92 21.61 3.22 21.20 2.81C20.77 2.41 20.09 2.39 19.67 2.81ZM21.20 19.67L4.33 2.81C3.93 2.41 3.23 2.39 2.81 2.81C2.40 3.23 2.40 3.92 2.81 4.33L19.67 21.19C20.08 21.60 20.78 21.60 21.20 21.19C21.60 20.76 21.60 20.07 21.20 19.67Z"/></svg>"#,
    ),
    (
        "clock",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M11.99 21.49C17.24 21.49 21.50 17.25 21.50 12.00C21.50 6.75 17.24 2.51 11.99 2.51C6.76 2.51 2.50 6.75 2.50 12.00C2.50 17.25 6.76 21.49 11.99 21.49ZM11.99 19.92C7.62 19.92 4.08 16.38 4.08 12.00C4.08 7.62 7.62 4.08 11.99 4.08C16.37 4.08 19.91 7.62 19.91 12.00C19.91 16.38 16.37 19.92 11.99 19.92ZM7.12 13.00L11.98 13.00C12.35 13.00 12.64 12.72 12.64 12.36L12.64 6.08C12.64 5.72 12.35 5.44 11.98 5.44C11.63 5.44 11.35 5.72 11.35 6.08L11.35 11.71L7.12 11.71C6.75 11.71 6.48 11.99 6.48 12.36C6.48 12.72 6.75 13.00 7.12 13.00Z"/></svg>"#,
    ),
    (
        "gearshape",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M11.10 21.49L12.89 21.49C13.58 21.49 14.11 21.07 14.27 20.42L14.65 18.75L14.94 18.66L16.37 19.55C16.95 19.90 17.62 19.83 18.10 19.33L19.35 18.09C19.84 17.61 19.92 16.93 19.56 16.37L18.66 14.92L18.76 14.66L20.42 14.28C21.08 14.13 21.50 13.58 21.50 12.90L21.50 11.15C21.50 10.47 21.09 9.94 20.42 9.78L18.78 9.38L18.67 9.11L19.57 7.66C19.93 7.09 19.86 6.42 19.36 5.93L18.11 4.69C17.64 4.20 16.97 4.12 16.39 4.47L14.94 5.35L14.65 5.24L14.27 3.58C14.11 2.93 13.58 2.51 12.89 2.51L11.10 2.51C10.41 2.51 9.89 2.93 9.73 3.58L9.35 5.24L9.04 5.35L7.60 4.47C7.02 4.12 6.36 4.20 5.87 4.69L4.64 5.93C4.14 6.42 4.07 7.09 4.43 7.66L5.32 9.11L5.21 9.38L3.57 9.78C2.91 9.94 2.50 10.47 2.50 11.15L2.50 12.90C2.50 13.58 2.92 14.13 3.57 14.28L5.23 14.66L5.33 14.92L4.43 16.37C4.08 16.93 4.16 17.61 4.64 18.09L5.88 19.33C6.37 19.83 7.04 19.90 7.62 19.55L9.06 18.66L9.35 18.75L9.73 20.42C9.89 21.07 10.41 21.49 11.10 21.49ZM11.24 20.10C11.09 20.10 11.01 20.04 10.99 19.90L10.45 17.70C9.91 17.56 9.41 17.36 9.02 17.11L7.09 18.31C6.98 18.38 6.86 18.37 6.76 18.26L5.72 17.21C5.61 17.11 5.61 17.01 5.69 16.88L6.87 14.96C6.67 14.58 6.43 14.09 6.29 13.54L4.08 13.01C3.95 12.99 3.88 12.91 3.88 12.76L3.88 11.28C3.88 11.11 3.94 11.05 4.08 11.01L6.28 10.49C6.42 9.91 6.70 9.39 6.85 9.06L5.68 7.14C5.59 7.01 5.59 6.90 5.70 6.80L6.75 5.76C6.86 5.67 6.95 5.65 7.09 5.72L9.00 6.89C9.39 6.67 9.93 6.45 10.46 6.30L10.99 4.10C11.01 3.96 11.09 3.90 11.24 3.90L12.76 3.90C12.91 3.90 12.99 3.96 13.01 4.10L13.54 6.32C14.09 6.46 14.58 6.68 14.97 6.90L16.90 5.72C17.04 5.65 17.13 5.67 17.24 5.76L18.30 6.80C18.39 6.90 18.41 7.01 18.32 7.14L17.14 9.06C17.31 9.39 17.57 9.91 17.71 10.49L19.91 11.01C20.06 11.05 20.11 11.11 20.11 11.28L20.11 12.76C20.11 12.91 20.05 12.99 19.91 13.01L17.70 13.54C17.57 14.09 17.33 14.58 17.12 14.96L18.31 16.88C18.39 17.01 18.38 17.11 18.28 17.21L17.24 18.26C17.13 18.37 17.01 18.38 16.90 18.31L14.96 17.11C14.59 17.36 14.08 17.56 13.54 17.70L13.01 19.90C12.99 20.04 12.91 20.10 12.76 20.10ZM11.99 15.38C13.87 15.38 15.38 13.87 15.38 12.00C15.38 10.13 13.87 8.62 11.99 8.62C10.12 8.62 8.62 10.13 8.62 12.00C8.62 13.87 10.12 15.38 11.99 15.38ZM11.99 14.01C10.89 14.01 10.00 13.10 10.00 12.00C10.00 10.89 10.89 9.99 11.99 9.99C13.10 9.99 14.00 10.89 14.00 12.00C14.00 13.10 13.10 14.01 11.99 14.01Z"/></svg>"#,
    ),
    (
        "speaker.wave.2",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M18.83 17.84C19.15 18.05 19.54 17.98 19.77 17.65C20.85 16.16 21.50 14.09 21.50 11.99C21.50 9.89 20.86 7.81 19.77 6.33C19.54 6.00 19.15 5.94 18.83 6.15C18.53 6.36 18.47 6.76 18.72 7.11C19.65 8.42 20.18 10.18 20.18 11.99C20.18 13.80 19.63 15.55 18.72 16.87C18.48 17.23 18.53 17.63 18.83 17.84ZM15.68 15.62C15.97 15.81 16.37 15.76 16.60 15.43C17.25 14.56 17.64 13.30 17.64 11.99C17.64 10.69 17.25 9.43 16.60 8.56C16.37 8.23 15.97 8.17 15.68 8.37C15.34 8.61 15.28 9.03 15.54 9.38C16.04 10.06 16.32 11.00 16.32 11.99C16.32 12.98 16.04 13.92 15.54 14.60C15.29 14.97 15.34 15.37 15.68 15.62ZM4.24 15.46L6.68 15.46C6.75 15.46 6.82 15.49 6.88 15.54L10.72 18.98C11.13 19.33 11.44 19.51 11.85 19.51C12.40 19.51 12.81 19.11 12.81 18.55L12.81 5.50C12.81 4.95 12.40 4.49 11.83 4.49C11.42 4.49 11.17 4.68 10.72 5.08L6.88 8.48C6.82 8.54 6.75 8.56 6.68 8.56L4.24 8.56C3.08 8.56 2.50 9.15 2.50 10.39L2.50 13.64C2.50 14.87 3.08 15.46 4.24 15.46ZM4.30 14.22C3.98 14.22 3.84 14.07 3.84 13.75L3.84 10.27C3.84 9.95 3.98 9.80 4.30 9.80L7.02 9.80C7.24 9.80 7.41 9.77 7.60 9.58L11.23 6.29C11.27 6.25 11.32 6.22 11.37 6.22C11.44 6.22 11.49 6.26 11.49 6.34L11.49 17.65C11.49 17.74 11.44 17.78 11.38 17.78C11.33 17.78 11.28 17.76 11.23 17.71L7.60 14.44C7.41 14.26 7.24 14.22 7.02 14.22Z"/></svg>"#,
    ),
    (
        "moon.fill",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12.43 21.50C16.53 21.50 19.86 19.03 21.33 15.88C21.64 15.22 21.22 14.77 20.60 14.97C19.90 15.20 18.75 15.42 17.69 15.42C12.03 15.42 8.80 12.20 8.80 6.54C8.80 5.48 9.04 4.28 9.37 3.42C9.65 2.72 9.17 2.30 8.51 2.60C5.12 4.06 2.57 7.52 2.57 11.64C2.57 17.08 6.98 21.50 12.43 21.50Z"/></svg>"#,
    ),
    (
        "lock.fill",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M7.56 21.50L16.44 21.50C17.86 21.50 18.61 20.73 18.61 19.19L18.61 12.52C18.61 11.00 17.86 10.23 16.44 10.23L7.56 10.23C6.14 10.23 5.39 11.00 5.39 12.52L5.39 19.19C5.39 20.73 6.14 21.50 7.56 21.50ZM7.09 10.97L8.62 10.97L8.62 7.70C8.62 5.25 10.19 3.96 12.00 3.96C13.81 3.96 15.39 5.25 15.39 7.70L15.39 10.97L16.92 10.97L16.92 7.90C16.92 4.26 14.54 2.50 12.00 2.50C9.46 2.50 7.09 4.26 7.09 7.90Z"/></svg>"#,
    ),
    (
        "power",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M11.99 21.50C17.10 21.50 21.24 17.37 21.24 12.27C21.24 9.41 19.98 7.06 18.24 5.47C17.45 4.74 16.41 5.85 17.20 6.60C18.74 8.00 19.70 10.01 19.70 12.27C19.70 16.52 16.25 19.97 11.99 19.97C7.74 19.97 4.30 16.52 4.30 12.27C4.30 9.99 5.27 7.99 6.80 6.59C7.60 5.83 6.57 4.77 5.77 5.48C4.01 7.04 2.76 9.49 2.76 12.27C2.76 17.37 6.90 21.50 11.99 21.50ZM11.99 12.17C12.44 12.17 12.72 11.87 12.72 11.41L12.72 3.28C12.72 2.81 12.44 2.50 11.99 2.50C11.56 2.50 11.27 2.81 11.27 3.28L11.27 11.41C11.27 11.87 11.56 12.17 11.99 12.17Z"/></svg>"#,
    ),
    (
        "plus.square.on.square",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.32 17.25L14.33 17.25C16.20 17.25 17.15 16.32 17.15 14.46L17.15 5.38C17.15 3.53 16.20 2.60 14.33 2.60L5.32 2.60C3.44 2.60 2.50 3.52 2.50 5.38L2.50 14.46C2.50 16.32 3.44 17.25 5.32 17.25ZM5.34 15.80C4.44 15.80 3.94 15.32 3.94 14.39L3.94 5.46C3.94 4.52 4.44 4.04 5.34 4.04L14.31 4.04C15.19 4.04 15.70 4.52 15.70 5.46L15.70 14.39C15.70 15.32 15.19 15.80 14.31 15.80ZM9.67 21.40L18.68 21.40C20.56 21.40 21.50 20.47 21.50 18.62L21.50 9.54C21.50 7.69 20.56 6.76 18.68 6.76L9.67 6.76C7.79 6.76 6.85 7.68 6.85 9.54L6.85 18.62C6.85 20.48 7.79 21.40 9.67 21.40ZM9.67 21.40L18.68 21.40C20.56 21.40 21.50 20.47 21.50 18.62L21.50 9.54C21.50 7.69 20.56 6.76 18.68 6.76L9.67 6.76C7.79 6.76 6.85 7.68 6.85 9.54L6.85 18.62C6.85 20.48 7.79 21.40 9.67 21.40ZM9.70 19.96C8.80 19.96 8.30 19.48 8.30 18.54L8.30 9.62C8.30 8.68 8.80 8.20 9.70 8.20L18.66 8.20C19.55 8.20 20.06 8.68 20.06 9.62L20.06 18.54C20.06 19.48 19.55 19.96 18.66 19.96ZM14.93 17.47L14.93 10.68C14.93 10.21 14.62 9.90 14.17 9.90C13.73 9.90 13.43 10.22 13.43 10.68L13.43 17.47C13.43 17.92 13.73 18.23 14.17 18.23C14.62 18.23 14.93 17.92 14.93 17.47ZM10.78 14.82L17.58 14.82C18.03 14.82 18.34 14.52 18.34 14.08C18.34 13.63 18.03 13.32 17.58 13.32L10.78 13.32C10.32 13.32 10.02 13.63 10.02 14.08C10.02 14.52 10.32 14.82 10.78 14.82Z"/></svg>"#,
    ),
    (
        "minus.square",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.74 21.50L18.26 21.50C20.42 21.50 21.49 20.42 21.49 18.30L21.49 5.71C21.49 3.58 20.42 2.50 18.26 2.50L5.74 2.50C3.59 2.50 2.51 3.57 2.51 5.71L2.51 18.30C2.51 20.43 3.59 21.50 5.74 21.50ZM5.77 19.83C4.74 19.83 4.16 19.28 4.16 18.21L4.16 5.79C4.16 4.71 4.74 4.17 5.77 4.17L18.23 4.17C19.25 4.17 19.84 4.71 19.84 5.79L19.84 18.21C19.84 19.28 19.25 19.83 18.23 19.83ZM7.96 12.88L16.05 12.88C16.63 12.88 17.02 12.57 17.02 12.03C17.02 11.47 16.66 11.15 16.05 11.15L7.96 11.15C7.37 11.15 6.98 11.47 6.98 12.03C6.98 12.57 7.39 12.88 7.96 12.88Z"/></svg>"#,
    ),
    (
        "arrow.clockwise",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12.00 4.00C14.12 4.00 16.05 4.83 17.48 6.18L19.60 4.06C19.99 3.67 20.66 3.95 20.66 4.50L20.66 10.00C20.66 10.55 20.21 11.00 19.66 11.00L14.16 11.00C13.61 11.00 13.33 10.33 13.72 9.94L15.96 7.70C14.90 6.72 13.52 6.12 12.00 6.12C8.75 6.12 6.12 8.75 6.12 12.00C6.12 15.25 8.75 17.88 12.00 17.88C14.68 17.88 16.94 16.08 17.65 13.62C17.81 13.06 18.40 12.74 18.96 12.90C19.52 13.06 19.84 13.65 19.68 14.21C18.71 17.56 15.64 20.00 12.00 20.00C7.58 20.00 4.00 16.42 4.00 12.00C4.00 7.58 7.58 4.00 12.00 4.00Z"/></svg>"#,
    ),
    (
        "arrow.down.circle",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12.00 2.50C17.25 2.50 21.50 6.75 21.50 12.00C21.50 17.25 17.25 21.50 12.00 21.50C6.75 21.50 2.50 17.25 2.50 12.00C2.50 6.75 6.75 2.50 12.00 2.50ZM12.00 4.10C7.64 4.10 4.10 7.64 4.10 12.00C4.10 16.36 7.64 19.90 12.00 19.90C16.36 19.90 19.90 16.36 19.90 12.00C19.90 7.64 16.36 4.10 12.00 4.10ZM12.00 7.20C12.47 7.20 12.85 7.58 12.85 8.05L12.85 13.65L15.15 11.35C15.48 11.02 16.02 11.02 16.35 11.35C16.68 11.68 16.68 12.22 16.35 12.55L12.60 16.30C12.27 16.63 11.73 16.63 11.40 16.30L7.65 12.55C7.32 12.22 7.32 11.68 7.65 11.35C7.98 11.02 8.52 11.02 8.85 11.35L11.15 13.65L11.15 8.05C11.15 7.58 11.53 7.20 12.00 7.20Z"/></svg>"#,
    ),
    (
        "bookmark",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M6.50 3.00L17.50 3.00C18.60 3.00 19.50 3.90 19.50 5.00L19.50 20.20C19.50 21.02 18.56 21.49 17.90 21.00L12.00 16.58L6.10 21.00C5.44 21.49 4.50 21.02 4.50 20.20L4.50 5.00C4.50 3.90 5.40 3.00 6.50 3.00ZM6.50 4.80L6.50 18.40L11.40 14.72C11.76 14.45 12.24 14.45 12.60 14.72L17.50 18.40L17.50 4.80Z"/></svg>"#,
    ),
    (
        "play.fill",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M6.40 4.15C5.55 3.63 4.50 4.24 4.50 5.24L4.50 18.76C4.50 19.76 5.55 20.37 6.40 19.85L19.15 13.09C20.02 12.61 20.02 11.39 19.15 10.91Z"/></svg>"#,
    ),
    (
        "trash",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M9.50 3.00L14.50 3.00C15.33 3.00 16.00 3.67 16.00 4.50L16.00 5.50L19.50 5.50C20.05 5.50 20.50 5.95 20.50 6.50C20.50 7.05 20.05 7.50 19.50 7.50L18.45 7.50L17.65 19.10C17.55 20.45 16.43 21.50 15.08 21.50L8.92 21.50C7.57 21.50 6.45 20.45 6.35 19.10L5.55 7.50L4.50 7.50C3.95 7.50 3.50 7.05 3.50 6.50C3.50 5.95 3.95 5.50 4.50 5.50L8.00 5.50L8.00 4.50C8.00 3.67 8.67 3.00 9.50 3.00ZM7.56 7.50L8.34 18.95C8.38 19.43 8.78 19.80 9.26 19.80L14.74 19.80C15.22 19.80 15.62 19.43 15.66 18.95L16.44 7.50Z"/></svg>"#,
    ),
    (
        "chevron.down",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12.00 16.85C11.73 16.85 11.48 16.74 11.27 16.54L4.29 9.56C3.90 9.17 3.90 8.54 4.29 8.15C4.68 7.76 5.31 7.76 5.70 8.15L12.00 14.45L18.30 8.15C18.69 7.76 19.32 7.76 19.71 8.15C20.10 8.54 20.10 9.17 19.71 9.56L12.73 16.54C12.52 16.74 12.27 16.85 12.00 16.85Z"/></svg>"#,
    ),
    (
        "chevron.up",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12.00 7.15C12.27 7.15 12.52 7.26 12.73 7.46L19.71 14.44C20.10 14.83 20.10 15.46 19.71 15.85C19.32 16.24 18.69 16.24 18.30 15.85L12.00 9.55L5.70 15.85C5.31 16.24 4.68 16.24 4.29 15.85C3.90 15.46 3.90 14.83 4.29 14.44L11.27 7.46C11.48 7.26 11.73 7.15 12.00 7.15Z"/></svg>"#,
    ),
    (
        "arrow.up.right.square",
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M5.74 21.50L18.26 21.50C20.42 21.50 21.49 20.42 21.49 18.30L21.49 5.71C21.49 3.58 20.42 2.50 18.26 2.50L5.74 2.50C3.59 2.50 2.51 3.57 2.51 5.71L2.51 18.30C2.51 20.43 3.59 21.50 5.74 21.50ZM5.77 19.83C4.74 19.83 4.16 19.28 4.16 18.21L4.16 5.79C4.16 4.71 4.74 4.17 5.77 4.17L18.23 4.17C19.25 4.17 19.84 4.71 19.84 5.79L19.84 18.21C19.84 19.28 19.25 19.83 18.23 19.83ZM9.50 7.50C9.00 7.50 8.60 7.90 8.60 8.40C8.60 8.90 9.00 9.30 9.50 9.30L13.43 9.30L7.86 14.86C7.51 15.21 7.51 15.79 7.86 16.14C8.21 16.49 8.79 16.49 9.14 16.14L14.70 10.57L14.70 14.50C14.70 15.00 15.10 15.40 15.60 15.40C16.10 15.40 16.50 15.00 16.50 14.50L16.50 8.40C16.50 7.90 16.10 7.50 15.60 7.50Z"/></svg>"#,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_sf_symbols_all_resolve_valid_svg() {
        for (name, _) in BUILTIN_SF_SYMBOLS {
            let bytes = lookup_sf_symbol_svg(name).expect("symbol should resolve");
            let text = std::str::from_utf8(bytes).expect("valid utf8");
            assert!(text.starts_with("<svg"), "invalid svg for {name}");
            assert!(text.contains("<path"), "missing path for {name}");
        }
    }

    #[test]
    fn sf_symbol_from_glyph_maps_legacy_and_sf_names() {
        assert_eq!(SfSymbol::from_glyph("⌕"), SfSymbol::MagnifyingGlass);
        assert_eq!(
            SfSymbol::from_glyph("rectangle.lefthalf.filled"),
            SfSymbol::RectangleLeftHalfFilled
        );
        assert_eq!(SfSymbol::from_glyph("📋"), SfSymbol::DocOnDoc);
        assert_eq!(SfSymbol::from_glyph("🔗"), SfSymbol::Link);
        assert_eq!(SfSymbol::from_glyph("📁"), SfSymbol::Folder);
        assert_eq!(SfSymbol::from_glyph("↻"), SfSymbol::ArrowClockwise);
        assert_eq!(SfSymbol::from_glyph("⬇"), SfSymbol::ArrowDownCircle);
        assert_eq!(SfSymbol::from_keycap("↵"), Some(SfSymbol::Return));
    }

    #[test]
    fn freedesktop_xdg_mapping_and_fallback_work_on_linux_and_macos() {
        assert_eq!(
            SfSymbol::Terminal.freedesktop_name(),
            "utilities-terminal-symbolic"
        );
        assert_eq!(SfSymbol::Globe.freedesktop_name(), "web-browser-symbolic");
        assert!(resolve_linux_xdg_svg("../invalid/path").is_none());
        let fallback = lookup_xdg_or_sf_svg("terminal").expect("fallback svg");
        assert!(std::str::from_utf8(fallback).unwrap().starts_with("<svg"));
    }
}
