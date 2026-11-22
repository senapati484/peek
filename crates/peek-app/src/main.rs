mod app;
mod assets;
mod inject;

use std::path::PathBuf;

use anyhow::Context;
use peek_store::Store;
use tao::dpi::LogicalSize;
use tao::event_loop::EventLoopBuilder;
use tao::window::WindowBuilder;

use app::{App, UserEvent};

fn main() -> anyhow::Result<()> {
    let p_dir = profile_dir();
    std::fs::create_dir_all(&p_dir)?;
    let db_path = p_dir.join("peek.db");
    let store = Store::open(&db_path)
        .with_context(|| format!("opening profile database at {}", db_path.display()))?;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Peek")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(640.0, 480.0))
        .with_visible(false)
        .build(&event_loop)
        .context("creating the Peek window")?;

    set_app_icon();
    setup_macos_menu();

    let mut app = App::new(window, proxy, store, p_dir)?;
    app.start()?;

    event_loop.run(move |event, _, control_flow| {
        app.handle(event, control_flow);
    });
}

fn profile_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("PEEK_DATA_DIR") {
        return PathBuf::from(explicit);
    }
    if let Some(dirs) = directories::ProjectDirs::from("dev", "Peek", "Peek") {
        return dirs
            .data_dir()
            .join("profiles")
            .join("default");
    }
    PathBuf::from(".peek").join("default")
}

#[cfg(target_os = "macos")]
pub fn set_app_icon() {
    use objc2::AnyThread;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSImage};
    use objc2_foundation::{MainThreadMarker, NSData};

    const ICON_BYTES: &[u8] = include_bytes!("../../../assets/app_icon_1024.png");

    unsafe {
        let mtm = MainThreadMarker::new_unchecked();
        let app = NSApplication::sharedApplication(mtm);
        let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        let data = NSData::with_bytes(ICON_BYTES);
        if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
            app.setApplicationIconImage(Some(&image));
            app.dockTile().display();
        }
    }
}

#[cfg(target_os = "macos")]
pub fn setup_macos_menu() {
    use objc2::{rc::Retained, sel, MainThreadOnly};
    use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
    use objc2_foundation::{ns_string, MainThreadMarker, NSString};

    unsafe {
        let mtm = MainThreadMarker::new_unchecked();
        let app = NSApplication::sharedApplication(mtm);

        // Explicitly set process name to "Peek" for macOS dock and menu
        use objc2::msg_send;
        let proc_info_cls = objc2::class!(NSProcessInfo);
        let proc_info: *mut objc2::runtime::AnyObject = msg_send![proc_info_cls, processInfo];
        if !proc_info.is_null() {
            let () = msg_send![proc_info, setProcessName: ns_string!("Peek")];
        }

        let main_menu = NSMenu::new(mtm);

        let add_item = |menu: &NSMenu,
                        title: &NSString,
                        action: Option<objc2::runtime::Sel>,
                        key_eq: &NSString|
         -> Retained<NSMenuItem> {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                title,
                action,
                key_eq,
            );
            menu.addItem(&item);
            item
        };

        // 1. App menu (Peek)
        let app_menu_item = NSMenuItem::new(mtm);
        let app_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Peek"));

        add_item(
            &app_menu,
            ns_string!("About Peek"),
            Some(sel!(orderFrontStandardAboutPanel:)),
            ns_string!(""),
        );
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_item(
            &app_menu,
            ns_string!("Hide Peek"),
            Some(sel!(hide:)),
            ns_string!("h"),
        );
        add_item(
            &app_menu,
            ns_string!("Hide Others"),
            Some(sel!(hideOtherApplications:)),
            ns_string!("h"),
        );
        add_item(
            &app_menu,
            ns_string!("Show All"),
            Some(sel!(unhideAllApplications:)),
            ns_string!(""),
        );
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_item(
            &app_menu,
            ns_string!("Quit Peek"),
            Some(sel!(terminate:)),
            ns_string!("q"),
        );
        app_menu_item.setSubmenu(Some(&app_menu));
        main_menu.addItem(&app_menu_item);

        // 2. Edit menu (Enables ⌘A, ⌘C, ⌘V, ⌘X, ⌘Z, ⌘⇧Z)
        let edit_menu_item = NSMenuItem::new(mtm);
        let edit_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Edit"));

        add_item(
            &edit_menu,
            ns_string!("Undo"),
            Some(sel!(undo:)),
            ns_string!("z"),
        );
        add_item(
            &edit_menu,
            ns_string!("Redo"),
            Some(sel!(redo:)),
            ns_string!("Z"),
        );
        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_item(
            &edit_menu,
            ns_string!("Cut"),
            Some(sel!(cut:)),
            ns_string!("x"),
        );
        add_item(
            &edit_menu,
            ns_string!("Copy"),
            Some(sel!(copy:)),
            ns_string!("c"),
        );
        add_item(
            &edit_menu,
            ns_string!("Paste"),
            Some(sel!(paste:)),
            ns_string!("v"),
        );
        add_item(
            &edit_menu,
            ns_string!("Paste and Match Style"),
            Some(sel!(pasteAsPlainText:)),
            ns_string!("V"),
        );
        add_item(
            &edit_menu,
            ns_string!("Delete"),
            Some(sel!(delete:)),
            ns_string!(""),
        );
        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_item(
            &edit_menu,
            ns_string!("Select All"),
            Some(sel!(selectAll:)),
            ns_string!("a"),
        );
        edit_menu_item.setSubmenu(Some(&edit_menu));
        main_menu.addItem(&edit_menu_item);

        // 3. Window menu (Enables ⌘M)
        let window_menu_item = NSMenuItem::new(mtm);
        let window_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Window"));

        add_item(
            &window_menu,
            ns_string!("Minimize"),
            Some(sel!(performMiniaturize:)),
            ns_string!("m"),
        );
        add_item(
            &window_menu,
            ns_string!("Zoom"),
            Some(sel!(performZoom:)),
            ns_string!(""),
        );
        window_menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_item(
            &window_menu,
            ns_string!("Bring All to Front"),
            Some(sel!(arrangeInFront:)),
            ns_string!(""),
        );
        window_menu_item.setSubmenu(Some(&window_menu));
        main_menu.addItem(&window_menu_item);

        app.setMainMenu(Some(&main_menu));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn setup_macos_menu() {}

#[cfg(not(target_os = "macos"))]
pub fn set_app_icon() {}
