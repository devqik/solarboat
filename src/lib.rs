pub mod cli;
pub mod commands;
pub mod config;
pub mod utils;

use clap::Parser;
use colored::*;
use std::env;

fn print_banner() {
    // ASCII art for 'Solarboat' (user-provided, each line a different color)
    let ascii = vec![
        "   _____         _               _                    _   ",
        "  / ____|       | |             | |                  | |  ",
        " | (___    ___  | |  __ _  _ __ | |__    ___    __ _ | |_ ",
        "  \\___ \\  / _ \\ | | / _` || '__|| '_ \\  / _ \\  / _` || __|",
        "  ____) || (_) || || (_| || |   | |_) || (_) || (_| || |_ ",
        " |_____/  \\___/ |_| \\__,_||_|   |_.__/  \\___/  \\__,_| \\__|",
    ];
    let colors = [Color::Red, Color::Yellow, Color::Green, Color::Cyan, Color::Blue, Color::Magenta];
    println!();
    for (i, line) in ascii.iter().enumerate() {
        println!("{}", line.color(colors[i % colors.len()]).bold());
    }
    println!();
    println!("  {}  {}", "🚤".bold().blue(), "Solarboat CLI".bold().bright_yellow());
    println!("  {}", "Modern Terraform Automation for DevOps & GitOps".italic().bright_blue());
    println!("  {}", "Happy infrastructure automation journey!".italic().bright_green());
    println!("  {}", "https://github.com/devqik/solarboat".italic().bright_purple());
    println!("  {}", "License: BSD-3-Clause".italic().bright_cyan());
    println!();
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    // Show banner only if --help or help is present as a top-level arg
    let show_banner = args.iter().any(|a| a == "--help" || a == "-h" || a == "help");
    if show_banner {
        print_banner();
    }
    let cli = cli::Args::parse();
    Ok(commands::handle_command(cli)?)
}
