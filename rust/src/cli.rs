use crate::config::{CommandItem, CustomConfig, InfoItem};
use console::{style, Term};
use dialoguer::{Confirm, Input, Select};

const MAIN_MENU: &[&str] = &[
    "Info Items        - View & manage info broadcasts",
    "Commands          - View & manage custom commands",
    "Save & Exit",
    "Exit Without Saving",
];

const INFO_MENU: &[&str] = &[
    "Add Info Item",
    "Edit Info Item",
    "Remove Info Item",
    "Back",
];

const CMD_MENU: &[&str] = &[
    "Add Command",
    "Edit Command",
    "Remove Command",
    "Back",
];

fn print_header(term: &Term, config_path: &str) {
    let _ = term.clear_screen();
    println!(
        "{}",
        style("┌──────────────────────────────────────────┐").cyan()
    );
    println!(
        "{}",
        style("│     Sugar WiFi Config Editor             │").cyan()
    );
    println!(
        "{}",
        style("└──────────────────────────────────────────┘").cyan()
    );
    println!("  Config: {}\n", style(config_path).yellow());
}

fn print_info_table(config: &CustomConfig) {
    if config.info.is_empty() {
        println!("  {}\n", style("(no info items)").dim());
        return;
    }
    println!(
        "  {:<4} {:<15} {:<40} {}",
        style("#").bold(),
        style("Label").bold(),
        style("Command").bold(),
        style("Interval").bold()
    );
    println!("  {}", style("─".repeat(70)).dim());
    for (i, item) in config.info.iter().enumerate() {
        let cmd_display = if item.command.len() > 38 {
            format!("{}…", &item.command[..37])
        } else {
            item.command.clone()
        };
        println!(
            "  {:<4} {:<15} {:<40} {}s",
            style(i + 1).green(),
            item.label,
            cmd_display,
            item.interval
        );
    }
    println!();
}

fn print_cmd_table(config: &CustomConfig) {
    if config.commands.is_empty() {
        println!("  {}\n", style("(no commands)").dim());
        return;
    }
    println!(
        "  {:<4} {:<15} {}",
        style("#").bold(),
        style("Label").bold(),
        style("Command").bold()
    );
    println!("  {}", style("─".repeat(50)).dim());
    for (i, item) in config.commands.iter().enumerate() {
        println!(
            "  {:<4} {:<15} {}",
            style(i + 1).green(),
            item.label,
            item.command
        );
    }
    println!();
}

fn select_info_item(config: &CustomConfig, prompt: &str) -> Option<usize> {
    if config.info.is_empty() {
        println!("  {}", style("No info items.").yellow());
        return None;
    }
    let items: Vec<String> = config
        .info
        .iter()
        .map(|i| format!("{} ({}s)", i.label, i.interval))
        .collect();
    Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact_opt()
        .unwrap_or(None)
}

fn select_cmd_item(config: &CustomConfig, prompt: &str) -> Option<usize> {
    if config.commands.is_empty() {
        println!("  {}", style("No commands.").yellow());
        return None;
    }
    let items: Vec<String> = config
        .commands
        .iter()
        .map(|c| format!("{} → {}", c.label, c.command))
        .collect();
    Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact_opt()
        .unwrap_or(None)
}

fn add_info(config: &mut CustomConfig) -> bool {
    println!("\n  {}", style("── Add Info Item ──").cyan());
    let label: String = match Input::new()
        .with_prompt("  Label")
        .interact_text()
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let command: String = match Input::new()
        .with_prompt("  Shell command")
        .interact_text()
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let interval: u64 = Input::new()
        .with_prompt("  Interval (seconds)")
        .default(10u64)
        .interact_text()
        .unwrap_or(10);
    config.info.push(InfoItem {
        label,
        command,
        interval,
    });
    println!("  {}", style("✓ Info item added.").green());
    true
}

fn edit_info(config: &mut CustomConfig) -> bool {
    println!("\n  {}", style("── Edit Info Item ──").cyan());
    let idx = match select_info_item(config, "  Select item to edit") {
        Some(i) => i,
        None => return false,
    };
    let item = &config.info[idx];
    let label: String = Input::new()
        .with_prompt("  Label")
        .default(item.label.clone())
        .interact_text()
        .unwrap_or_else(|_| item.label.clone());
    let command: String = Input::new()
        .with_prompt("  Shell command")
        .default(item.command.clone())
        .interact_text()
        .unwrap_or_else(|_| item.command.clone());
    let interval: u64 = Input::new()
        .with_prompt("  Interval (seconds)")
        .default(item.interval)
        .interact_text()
        .unwrap_or(item.interval);
    config.info[idx] = InfoItem {
        label,
        command,
        interval,
    };
    println!("  {}", style("✓ Info item updated.").green());
    true
}

fn remove_info(config: &mut CustomConfig) -> bool {
    println!("\n  {}", style("── Remove Info Item ──").cyan());
    let idx = match select_info_item(config, "  Select item to remove") {
        Some(i) => i,
        None => return false,
    };
    let label = config.info[idx].label.clone();
    if Confirm::new()
        .with_prompt(format!("  Remove \"{}\"?", label))
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        config.info.remove(idx);
        println!("  {}", style(format!("✓ Removed \"{}\".", label)).green());
        return true;
    }
    false
}

fn add_command(config: &mut CustomConfig) -> bool {
    println!("\n  {}", style("── Add Command ──").cyan());
    let label: String = match Input::new()
        .with_prompt("  Label")
        .interact_text()
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let command: String = match Input::new()
        .with_prompt("  Shell command")
        .interact_text()
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    config.commands.push(CommandItem { label, command });
    println!("  {}", style("✓ Command added.").green());
    true
}

fn edit_command(config: &mut CustomConfig) -> bool {
    println!("\n  {}", style("── Edit Command ──").cyan());
    let idx = match select_cmd_item(config, "  Select command to edit") {
        Some(i) => i,
        None => return false,
    };
    let item = &config.commands[idx];
    let label: String = Input::new()
        .with_prompt("  Label")
        .default(item.label.clone())
        .interact_text()
        .unwrap_or_else(|_| item.label.clone());
    let command: String = Input::new()
        .with_prompt("  Shell command")
        .default(item.command.clone())
        .interact_text()
        .unwrap_or_else(|_| item.command.clone());
    config.commands[idx] = CommandItem { label, command };
    println!("  {}", style("✓ Command updated.").green());
    true
}

fn remove_command(config: &mut CustomConfig) -> bool {
    println!("\n  {}", style("── Remove Command ──").cyan());
    let idx = match select_cmd_item(config, "  Select command to remove") {
        Some(i) => i,
        None => return false,
    };
    let label = config.commands[idx].label.clone();
    if Confirm::new()
        .with_prompt(format!("  Remove \"{}\"?", label))
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        config.commands.remove(idx);
        println!(
            "  {}",
            style(format!("✓ Removed \"{}\".", label)).green()
        );
        return true;
    }
    false
}

fn info_submenu(term: &Term, config: &mut CustomConfig, config_path: &str) -> bool {
    let mut modified = false;
    loop {
        print_header(term, config_path);
        println!("  {}\n", style("── Info Items ──").cyan().bold());
        print_info_table(config);

        let sel = match Select::new()
            .with_prompt("  Action")
            .items(INFO_MENU)
            .default(0)
            .interact_opt()
        {
            Ok(Some(s)) => s,
            _ => return modified,
        };

        match sel {
            0 => {
                if add_info(config) {
                    modified = true;
                }
            }
            1 => {
                if edit_info(config) {
                    modified = true;
                }
            }
            2 => {
                if remove_info(config) {
                    modified = true;
                }
            }
            _ => return modified,
        }
    }
}

fn cmd_submenu(term: &Term, config: &mut CustomConfig, config_path: &str) -> bool {
    let mut modified = false;
    loop {
        print_header(term, config_path);
        println!("  {}\n", style("── Commands ──").cyan().bold());
        print_cmd_table(config);

        let sel = match Select::new()
            .with_prompt("  Action")
            .items(CMD_MENU)
            .default(0)
            .interact_opt()
        {
            Ok(Some(s)) => s,
            _ => return modified,
        };

        match sel {
            0 => {
                if add_command(config) {
                    modified = true;
                }
            }
            1 => {
                if edit_command(config) {
                    modified = true;
                }
            }
            2 => {
                if remove_command(config) {
                    modified = true;
                }
            }
            _ => return modified,
        }
    }
}

pub fn run_config_editor(config_path: &str) {
    let term = Term::stdout();
    let mut config = CustomConfig::load(config_path);
    let mut modified = false;

    loop {
        print_header(&term, config_path);
        println!(
            "  Info items: {}    Commands: {}\n",
            style(config.info.len()).green(),
            style(config.commands.len()).green()
        );

        let sel = match Select::new()
            .with_prompt("  Main Menu")
            .items(MAIN_MENU)
            .default(0)
            .interact_opt()
        {
            Ok(Some(s)) => s,
            _ => break,
        };

        match sel {
            0 => {
                if info_submenu(&term, &mut config, config_path) {
                    modified = true;
                }
            }
            1 => {
                if cmd_submenu(&term, &mut config, config_path) {
                    modified = true;
                }
            }
            2 => {
                // Save & Exit
                match config.save(config_path) {
                    Ok(()) => {
                        let _ = term.clear_screen();
                        println!("{}", style("✓ Config saved to:").green().bold());
                        println!("  {}", config_path);
                        println!(
                            "\n  Run {} to apply changes.",
                            style("sudo systemctl restart sugar-wifi-config").yellow()
                        );
                    }
                    Err(e) => eprintln!("{} {}", style("Error:").red().bold(), e),
                }
                return;
            }
            3 => {
                // Exit without saving
                if modified {
                    if !Confirm::new()
                        .with_prompt("  Discard unsaved changes?")
                        .default(false)
                        .interact()
                        .unwrap_or(false)
                    {
                        continue;
                    }
                }
                let _ = term.clear_screen();
                println!("{}", style("Exited without saving.").dim());
                return;
            }
            _ => {}
        }
    }
}
