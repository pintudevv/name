use anyhow::Result;
use colored::*;
use inquire::{Confirm, MultiSelect, Select, Text};
use std::io::{stdin, stdout, Read, Write};
use std::path::{Path, PathBuf};

use crate::engine::{execute_directory_rename, plan_directory_rename, print_preview};
use crate::rules::{CaseMode, RenameRules};
use crate::zip_engine::{execute_zip_rename, plan_zip_rename, print_zip_preview};

pub fn run_interactive_wizard() -> Result<()> {
    print_banner();

    println!(
        "{}\n",
        "Welcome! Follow the simple steps below to bulk rename your files or ZIP archive.".cyan()
    );

    // 1. Ask for Target path
    let target_path = loop {
        let input = Text::new("Enter folder or .ZIP file path (or drag & drop here):")
            .with_help_message("Drag and drop a folder or .zip file into this window, then press Enter")
            .prompt()?;

        let clean_path = clean_user_path(&input);
        if clean_path.is_empty() {
            println!("{}", "Path cannot be empty. Please try again.".red());
            continue;
        }

        let path = PathBuf::from(&clean_path);
        if !path.exists() {
            println!(
                "{}",
                format!("Error: Path '{}' does not exist. Please try again.", path.display()).red()
            );
            continue;
        }

        break path;
    };

    let is_zip = is_zip_file(&target_path);
    if is_zip {
        println!(
            "{}",
            format!("📦 Detected ZIP archive: {}", target_path.display()).green().bold()
        );
    } else if target_path.is_dir() {
        println!(
            "{}",
            format!("📁 Detected directory: {}", target_path.display()).green().bold()
        );
    } else {
        println!(
            "{}",
            format!("📄 Selected single file: {}", target_path.display()).yellow()
        );
    }

    let mut rules = RenameRules::default();
    let mut recursive = false;
    let mut new_zip_output_path: Option<PathBuf> = None;

    // Ask renaming workflow mode
    let workflow_modes = vec![
        "⚡ Quick Rename: Give all files a brand new name + numbering (e.g. MyFiles_001, MyFiles_002...)",
        "🔧 Custom Rules: Find & replace, prefix/suffix, case formatting, clean symbols, etc.",
    ];

    let chosen_mode = Select::new("Choose how you want to rename:", workflow_modes).prompt()?;

    if chosen_mode.starts_with("⚡ Quick Rename") {
        let base_name = Text::new("Enter the new name for the files:")
            .with_help_message("Every file will be named using this base name + number (e.g. 'Photo', 'Episode')")
            .prompt()?;

        let start_str = Text::new("Start numbering from:")
            .with_default("1")
            .prompt()?;
        let start: usize = start_str.trim().parse().unwrap_or(1);

        let pad_str = Text::new("Number of digits for zero-padding (e.g. 2 -> 01, 3 -> 001):")
            .with_default("3")
            .prompt()?;
        let pad: usize = pad_str.trim().parse().unwrap_or(3);

        let sep = Text::new("Separator before number:")
            .with_default("_")
            .prompt()?;

        rules.replace_name = Some(base_name.clone());
        rules.number_start = Some(start);
        rules.number_padding = pad;
        rules.number_prefix = Some(sep);

        // If it's a ZIP, ask if user also wants to rename the ZIP file itself
        if is_zip {
            let rename_zip_confirm = Confirm::new(&format!(
                "Also rename the .ZIP file itself to '{}.zip'?",
                base_name
            ))
            .with_default(true)
            .prompt()?;

            if rename_zip_confirm {
                let parent = target_path.parent().unwrap_or(Path::new("."));
                new_zip_output_path = Some(parent.join(format!("{}.zip", base_name)));
            }
        }
    } else {
        // Custom Rules Mode
        if !is_zip && target_path.is_dir() {
            recursive = Confirm::new("Include files in subfolders recursively?")
                .with_default(false)
                .prompt()?;
        }

        let filter_choice = Confirm::new("Filter specific file extensions only? (e.g., only .jpg, .png)")
            .with_default(false)
            .prompt()?;

        if filter_choice {
            let exts_input = Text::new("Enter extensions separated by commas (e.g. jpg, png, pdf):")
                .prompt()?;
            let exts: Vec<String> = exts_input
                .split(',')
                .map(|s| s.trim().trim_start_matches('.').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !exts.is_empty() {
                rules.filter_extensions = Some(exts);
            }
        }

        let options = vec![
            "1. Find and Replace text",
            "2. Add Prefix (text at start of filename)",
            "3. Add Suffix (text before file extension)",
            "4. Sequential Auto-Numbering (001, 002...)",
            "5. Change Case (lowercase, UPPERCASE, Title Case...)",
            "6. Sanitize Filenames (replace spaces/symbols with underscores)",
            "7. Change File Extension",
        ];

        let chosen_ops = MultiSelect::new(
            "Select renaming actions to apply (Space to toggle, Enter to confirm):",
            options,
        )
        .prompt()?;

        if chosen_ops.is_empty() {
            println!("{}", "No actions selected. Exiting.".yellow());
            pause_and_exit();
            return Ok(());
        }

        for op in chosen_ops {
            if op.starts_with("1.") {
                let find = Text::new("Text to find:").prompt()?;
                let replace = Text::new("Replace with (leave empty to delete matches):").prompt()?;
                let case_insensitive = Confirm::new("Case-insensitive search?")
                    .with_default(true)
                    .prompt()?;
                let is_regex = Confirm::new("Treat as regular expression (Regex)?")
                    .with_default(false)
                    .prompt()?;

                rules.find = Some(find);
                rules.replace = Some(replace);
                rules.case_insensitive = case_insensitive;
                rules.is_regex = is_regex;
            } else if op.starts_with("2.") {
                let prefix = Text::new("Enter prefix text to add:").prompt()?;
                if !prefix.is_empty() {
                    rules.prefix = Some(prefix);
                }
            } else if op.starts_with("3.") {
                let suffix = Text::new("Enter suffix text to add:").prompt()?;
                if !suffix.is_empty() {
                    rules.suffix = Some(suffix);
                }
            } else if op.starts_with("4.") {
                let start_str = Text::new("Start numbering from:")
                    .with_default("1")
                    .prompt()?;
                let start: usize = start_str.trim().parse().unwrap_or(1);
                let pad_str = Text::new("Number of digits for zero-padding (e.g. 3 -> 001):")
                    .with_default("3")
                    .prompt()?;
                let pad: usize = pad_str.trim().parse().unwrap_or(3);
                let sep = Text::new("Separator before number (e.g. '_' or '-'):")
                    .with_default("_")
                    .prompt()?;

                rules.number_start = Some(start);
                rules.number_padding = pad;
                rules.number_prefix = Some(sep);
            } else if op.starts_with("5.") {
                let case_modes = vec!["lowercase", "UPPERCASE", "Title Case", "snake_case", "kebab-case"];
                let chosen_case = Select::new("Choose casing format:", case_modes).prompt()?;
                rules.case_mode = CaseMode::from_str(chosen_case);
            } else if op.starts_with("6.") {
                rules.clean_spaces = true;
            } else if op.starts_with("7.") {
                let ext = Text::new("Enter new extension (e.g. png, jpg, webp):").prompt()?;
                if !ext.trim().is_empty() {
                    rules.new_extension = Some(ext.trim().to_string());
                }
            }
        }

        if is_zip {
            let rename_zip = Confirm::new("Also rename the outer .ZIP file itself?")
                .with_default(false)
                .prompt()?;
            if rename_zip {
                let new_zip_name = Text::new("Enter new name for the .ZIP file (without .zip):").prompt()?;
                if !new_zip_name.trim().is_empty() {
                    let parent = target_path.parent().unwrap_or(Path::new("."));
                    new_zip_output_path = Some(parent.join(format!("{}.zip", new_zip_name.trim())));
                }
            }
        }
    }

    // 3. Generate and Show Preview
    if is_zip {
        let plan = plan_zip_rename(&target_path, &rules)?;
        print_zip_preview(&plan, 30);

        if let Some(ref nzo) = new_zip_output_path {
            println!(
                "{}",
                format!("ZIP File Rename: '{}' -> '{}'", target_path.file_name().unwrap().to_string_lossy(), nzo.file_name().unwrap().to_string_lossy()).cyan().bold()
            );
        }

        if plan.changed_count == 0 && new_zip_output_path.is_none() {
            println!("{}", "Nothing to rename. Exiting.".yellow());
            pause_and_exit();
            return Ok(());
        }

        if plan.error_count > 0 {
            println!("{}", "Please fix conflict errors before renaming.".red().bold());
            pause_and_exit();
            return Ok(());
        }

        let confirm = Confirm::new("Apply these changes to the ZIP archive?")
            .with_default(true)
            .prompt()?;

        if confirm {
            println!("{}", "Processing ZIP archive...".cyan());
            let count = execute_zip_rename(&plan, new_zip_output_path.as_deref())?;
            let final_zip = new_zip_output_path.as_ref().unwrap_or(&target_path);
            println!(
                "{}",
                format!("Success! Renamed {} files in '{}'.", count, final_zip.display())
                    .green()
                    .bold()
            );
        } else {
            println!("{}", "Operation cancelled by user.".yellow());
        }
    } else {
        let plan = plan_directory_rename(&target_path, &rules, recursive)?;
        print_preview(&plan, 30);

        if plan.changed_count == 0 {
            println!("{}", "Nothing to rename. Exiting.".yellow());
            pause_and_exit();
            return Ok(());
        }

        if plan.error_count > 0 {
            println!("{}", "Please fix conflict errors before renaming.".red().bold());
            pause_and_exit();
            return Ok(());
        }

        let confirm = Confirm::new("Apply these renames to your files?")
            .with_default(true)
            .prompt()?;

        if confirm {
            println!("{}", "Renaming files...".cyan());
            let count = execute_directory_rename(&plan)?;
            println!(
                "{}",
                format!("Success! Successfully renamed {} files in '{}'.", count, target_path.display())
                    .green()
                    .bold()
            );
        } else {
            println!("{}", "Operation cancelled by user.".yellow());
        }
    }

    pause_and_exit();
    Ok(())
}

fn clean_user_path(input: &str) -> String {
    let mut s = input.trim().to_string();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s = s[1..s.len() - 1].to_string();
    }
    s.trim().to_string()
}

fn is_zip_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        ext.eq_ignore_ascii_case("zip")
    } else {
        false
    }
}

pub fn print_banner() {
    println!("{}", "=========================================================".bright_cyan());
    println!("{}", "    NAME CHANGER - Bulk File & ZIP Renamer Tool         ".bold().cyan());
    println!("{}", "    Fast, Safe, and Portable for Windows & CLI          ".bright_black());
    println!("{}\n", "=========================================================".bright_cyan());
}

pub fn pause_and_exit() {
    println!("\nPress Enter to exit...");
    let _ = stdout().flush();
    let mut buf = [0u8; 1];
    let _ = stdin().read(&mut buf);
}
