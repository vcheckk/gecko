mod app;
mod cache;
mod config;
mod game;
mod library;
mod theme;
mod widgets;

use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug, Clone)]
#[command(version, about = "Gecko: A humble GameCube/Wii emulator")]
struct Args {
    #[arg(long)]
    gcn: Option<PathBuf>,
    #[arg(long)]
    wii: Option<PathBuf>,
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let args = Args::parse();

    iced::application(
        move || app::App::new(args.gcn.clone(), args.wii.clone()),
        app::App::update,
        app::App::view,
    )
    .title(app::App::title)
    .theme(app::App::theme)
    .subscription(app::App::subscription)
    .window_size((1100.0, 720.0))
    .run()
}
