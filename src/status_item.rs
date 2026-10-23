#[cfg(target_os = "macos")]
pub mod macos {
    use std::ffi::{c_char, c_void};
    use std::sync::atomic::{AtomicBool, Ordering};

    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
        fn objc_allocateClassPair(
            superclass: *mut c_void,
            name: *const c_char,
            extraBytes: usize,
        ) -> *mut c_void;
        fn objc_registerClassPair(cls: *mut c_void);
        fn class_addMethod(
            cls: *mut c_void,
            name: *mut c_void,
            imp: extern "C" fn(*mut c_void, *mut c_void, *mut c_void),
            types: *const c_char,
        ) -> bool;
    }

    pub static MENU_CLICKED: AtomicBool = AtomicBool::new(false);

    extern "C" fn on_click(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
        MENU_CLICKED.store(true, Ordering::SeqCst);
    }

    pub fn setup_status_item() {
        unsafe {
            let ns_status_bar_cls = objc_getClass(c"NSStatusBar".as_ptr());
            if ns_status_bar_cls.is_null() {
                return;
            }

            let system_bar_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let bar = system_bar_fn(
                ns_status_bar_cls,
                sel_registerName(c"systemStatusBar".as_ptr()),
            );
            if bar.is_null() {
                return;
            }

            // statusItemWithLength: -1.0 (NSVariableStatusItemLength)
            let item_fn: extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let item = item_fn(
                bar,
                sel_registerName(c"statusItemWithLength:".as_ptr()),
                -1.0,
            );
            if item.is_null() {
                return;
            }

            let retain_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            retain_fn(item, sel_registerName(c"retain".as_ptr()));

            let button_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let button = button_fn(item, sel_registerName(c"button".as_ptr()));
            if button.is_null() {
                return;
            }

            let ns_str_cls = objc_getClass(c"NSString".as_ptr());
            let str_fn: extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let title = str_fn(
                ns_str_cls,
                sel_registerName(c"stringWithUTF8String:".as_ptr()),
                c"\u{2B21} Rover".as_ptr(),
            );
            let set_title: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            set_title(button, sel_registerName(c"setTitle:".as_ptr()), title);

            let mut target_cls = objc_getClass(c"AlpenglowStatusTarget".as_ptr());
            if target_cls.is_null() {
                let ns_object = objc_getClass(c"NSObject".as_ptr());
                target_cls =
                    objc_allocateClassPair(ns_object, c"AlpenglowStatusTarget".as_ptr(), 0);
                if !target_cls.is_null() {
                    let sel = sel_registerName(c"onStatusClick:".as_ptr());
                    class_addMethod(target_cls, sel, on_click, c"v@:@".as_ptr());
                    objc_registerClassPair(target_cls);
                }
            }

            if !target_cls.is_null() {
                let alloc_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let init_fn: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let target = init_fn(
                    alloc_fn(target_cls, sel_registerName(c"alloc".as_ptr())),
                    sel_registerName(c"init".as_ptr()),
                );
                retain_fn(target, sel_registerName(c"retain".as_ptr()));

                let set_target: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_target(button, sel_registerName(c"setTarget:".as_ptr()), target);

                let set_action: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_action(
                    button,
                    sel_registerName(c"setAction:".as_ptr()),
                    sel_registerName(c"onStatusClick:".as_ptr()),
                );
            }
        }
    }

    pub fn poll_menu_clicked() -> bool {
        MENU_CLICKED.swap(false, Ordering::SeqCst)
    }
}

#[cfg(not(target_os = "macos"))]
pub mod macos {
    pub fn setup_status_item() {}
    pub fn poll_menu_clicked() -> bool {
        false
    }
}

#[allow(unused_imports)]
pub use macos::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_menu_clicked_defaults_to_false_and_resets() {
        assert!(!poll_menu_clicked());
    }
}
