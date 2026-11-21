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
