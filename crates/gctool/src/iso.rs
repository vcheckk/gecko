use image::dvd::FstNode;
use owo_colors::OwoColorize;
use termtree::Tree;

pub fn info(data: Vec<u8>) {
    let dvd = image::dvd::Dvd::parse(data);

    let game_name = String::from_utf8_lossy(&dvd.header.game_name);
    let game_name = game_name.trim_end_matches('\0');

    println!("  {:<16} {}", "Game Name:".bold(), game_name);
    println!(
        "  {:<16} {}",
        "Game Code:".bold(),
        String::from_utf8_lossy(&dvd.header.game_code)
    );
    println!(
        "  {:<16} {}",
        "Maker Code:".bold(),
        String::from_utf8_lossy(&dvd.header.maker_code)
    );
    println!(
        "  {:<16} {} / {}",
        "Disk / Version:".bold(),
        dvd.header.disk_id,
        dvd.header.version
    );
    println!("  {:<16} {:08X}", "Magic:".bold(), u32::from_be_bytes(dvd.header.magic));
    println!(
        "  {:<16} {}",
        "Apploader Date:".bold(),
        String::from_utf8_lossy(&dvd.apploader.timestamp)
    );
    println!(
        "  {:<16} {:08X}",
        "Main DOL Offset:".bold(),
        dvd.header.offset_main_executable.get()
    );
    println!(
        "  {:<16} {:08X} ({} bytes)",
        "FST Offset:".bold(),
        dvd.header.offset_filesystem.get(),
        dvd.header.filesystem_size.get()
    );

    println!();
    println!("{}", build_tree(&dvd.filesystem));
}

fn build_tree(node: &FstNode) -> Tree<String> {
    match node {
        FstNode::Directory { name, children } => {
            let label = if name.is_empty() {
                "/".blue().bold().to_string()
            } else {
                format!("{}", name.blue().bold())
            };
            let mut tree = Tree::new(label);
            for child in children {
                tree.push(build_tree(child));
            }
            tree
        }
        FstNode::File { name, size, .. } => {
            let label = format!("{} {}", name, format!("({size} bytes)").dimmed());
            Tree::new(label)
        }
    }
}
