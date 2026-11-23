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
    let (profile_name, p_dir) = resolve_profile();
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

    let mut app = App::new(window, proxy, store, p_dir, profile_name)?;
    app.start()?;

    event_loop.run(move |event, _, control_flow| {
        app.handle(event, control_flow);
    });
}

fn resolve_profile() -> (String, PathBuf) {
    let env_data_dir = std::env::var("PEEK_DATA_DIR").ok();
    let env_profile = std::env::var("PEEK_PROFILE").ok();
    let profiles_root = if let Some(dirs) = directories::ProjectDirs::from("dev", "Peek", "Peek") {
        dirs.data_dir().join("profiles")
    } else {
        PathBuf::from(".peek").join("profiles")
    };
    resolve_profile_with(
        &profiles_root,
        env_data_dir.as_deref(),
        env_profile.as_deref(),
    )
}

pub fn resolve_profile_with(
    profiles_root: &std::path::Path,
    env_data_dir: Option<&str>,
    env_profile: Option<&str>,
) -> (String, PathBuf) {
    if let Some(explicit) = env_data_dir {
        return ("default".to_string(), PathBuf::from(explicit));
    }
    let _ = std::fs::create_dir_all(profiles_root);

    let active_file = profiles_root.join("active_profile");
    let mut active_name = "default".to_string();

    // Priority 1: Check environment override
    if let Some(explicit_profile) = env_profile {
        let clean = explicit_profile.trim();
        if !clean.is_empty()
            && !clean.contains('/')
            && !clean.contains('\\')
            && !clean.starts_with('.')
        {
            active_name = clean.to_string();
        }
    } else if let Ok(saved) = std::fs::read_to_string(&active_file) {
        // Priority 2: Read persisted active profile from disk
        let clean = saved.trim();
        if !clean.is_empty()
            && !clean.contains('/')
            && !clean.contains('\\')
            && !clean.starts_with('.')
        {
            let candidate_dir = profiles_root.join(clean);
            if candidate_dir.exists() {
                active_name = clean.to_string();
            }
        }
    }

    let p_dir = profiles_root.join(&active_name);
    let _ = std::fs::create_dir_all(&p_dir);
    let _ = std::fs::write(&active_file, &active_name);
    (active_name, p_dir)
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

        // 3. View menu (Enables native Full Screen toggle)
        let view_menu_item = NSMenuItem::new(mtm);
        let view_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("View"));
        let fs_item = add_item(
            &view_menu,
            ns_string!("Toggle Full Screen"),
            Some(sel!(toggleFullScreen:)),
            ns_string!("f"),
        );
        let () = msg_send![&*fs_item, setKeyEquivalentModifierMask: (1usize << 20) | (1usize << 18)]; // Command + Control
        view_menu_item.setSubmenu(Some(&view_menu));
        main_menu.addItem(&view_menu_item);

        // 4. Window menu (Enables ⌘M)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_resolution_defaults_and_persists() {
        let unique = format!("peek_test_profile_{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let root = std::env::temp_dir().join(unique);
        let _ = std::fs::remove_dir_all(&root);

        // 1. Initial resolution without active_profile defaults to "default"
        let (name1, path1) = resolve_profile_with(&root, None, None);
        assert_eq!(name1, "default");
        assert_eq!(path1, root.join("default"));
        assert!(path1.exists());
        assert_eq!(std::fs::read_to_string(root.join("active_profile")).unwrap(), "default");

        // 2. Create another profile directory "Sayan" and switch active_profile
        let sayan_dir = root.join("Sayan");
        std::fs::create_dir_all(&sayan_dir).unwrap();
        std::fs::write(root.join("active_profile"), "Sayan").unwrap();

        // 3. Next startup resolves "Sayan"
        let (name2, path2) = resolve_profile_with(&root, None, None);
        assert_eq!(name2, "Sayan");
        assert_eq!(path2, sayan_dir);

        // 4. Non-existent profile in active_profile falls back to "default"
        std::fs::write(root.join("active_profile"), "GhostProfile").unwrap();
        let (name3, path3) = resolve_profile_with(&root, None, None);
        assert_eq!(name3, "default");
        assert_eq!(path3, root.join("default"));

        // 5. Malicious or traversal profile names fall back to "default"
        std::fs::write(root.join("active_profile"), "../../../etc").unwrap();
        let (name4, path4) = resolve_profile_with(&root, None, None);
        assert_eq!(name4, "default");
        assert_eq!(path4, root.join("default"));

        // 6. Environment override takes precedence
        let (name5, path5) = resolve_profile_with(&root, None, Some("Work"));
        assert_eq!(name5, "Work");
        assert_eq!(path5, root.join("Work"));
        assert!(path5.exists());

        let _ = std::fs::remove_dir_all(&root);
    }
}
