use std::env;
use std::fs;
use std::process;

use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    group: Vec<Group>,
}

#[derive(Deserialize)]
struct Group {
    label: String,
    #[serde(default)]
    target: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name:      String,
    address:   String,
    via:       Option<String>,
    relay:     Option<String>,
    os:        Option<String>,
    user:      Option<String>,
    key:       Option<String>,
    community: Option<String>,
    tcp_port:  Option<u16>,
}

/// label 先頭の '#' の数を重要度として返す (1 = 最重要)
fn heading_level(label: &str) -> usize {
    label.chars().take_while(|c| *c == '#').count().max(1)
}

/// 重要度に応じた ANSI 装飾を付けた文字列を返す
fn render_label(label: &str) -> String {
    let text = label.trim_start_matches('#').trim();
    match heading_level(label) {
        1 => format!("\x1b[1;4m{}\x1b[0m", text),  // bold + underline
        2 => format!("\x1b[1m{}\x1b[0m", text),     // bold
        _ => format!("\x1b[2m{}\x1b[0m", text),     // dim
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config.toml>", args[0]);
        process::exit(1);
    }

    let path = &args[1];

    let contents = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error: {}: {}", path, e);
        process::exit(1);
    });

    let config: Config = toml::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("error: failed to parse '{}': {}", path, e);
        process::exit(1);
    });

    for group in &config.group {
        println!("\n{}", render_label(&group.label));

        for target in &group.target {
            print!("  {:<20}  {:<40}", target.name, target.address);

            // Some だけを key=value で並べる
            let opts: Vec<String> = [
                target.via.as_deref().map(|v| format!("via={}", v)),
                target.relay.as_deref().map(|v| format!("relay={}", v)),
                target.os.as_deref().map(|v| format!("os={}", v)),
                target.user.as_deref().map(|v| format!("user={}", v)),
                target.key.as_deref().map(|v| format!("key={}", v)),
                target.community.as_deref().map(|v| format!("community={}", v)),
                target.tcp_port.map(|p| format!("tcp_port={}", p)),
            ]
            .into_iter()
            .flatten()
            .collect();

            if !opts.is_empty() {
                print!("  {}", opts.join("  "));
            }

            println!();
        }
    }
}
