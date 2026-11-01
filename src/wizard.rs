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

    // 2. Ask for the New Name directly!
    println!(
        "\n{}",
        "👉 Enter the new name you want to give to your files:".cyan().bold()
    );
    println!(
        "{}",
        "   (Or simply press Enter to access advanced Custom Rules like Find & Replace)".bright_black()
    );

    let name_input = Text::new("Enter new name:")
        .with_help_message("e.g. 'Pintu_chaudhary_06_DS_sem5' or press Enter for custom rules")
        .prompt()?;

    if !name_input.trim().is_empty() {
        let base_name = name_input.trim().to_string();
        rules.replace_name = Some(base_name.clone());

        // Ask if numbering should be added
        let add_numbers = Confirm::new("Add numbering like _001, _002 to filenames?")
            .with_default(false)
            .prompt()?;

        if add_numbers {
            let start_str = Text::new("Start numbering from:")
                .with_default("1")
                .prompt()?;
            rules.number_start = Some(start_str.trim().parse().unwrap_or(1));

            let pad_str = Text::new("Zero-padding digits (e.g. 2 -> 01, 3 -> 001):")
                .with_default("3")
                .prompt()?;
            rules.number_padding = pad_str.trim().parse().unwrap_or(3);
            rules.number_prefix = Some("_".to_string());
        }

        if is_zip {
            let rename_zip = Confirm::new(&format!(
                "Also rename the .ZIP file to '{}.zip'?",
                base_name
            ))
            .with_default(true)
            .prompt()?;

            if rename_zip {
                let parent = target_path.parent().unwrap_or(Path::new("."));
                new_zip_output_path = Some(parent.join(format!("{}.zip", base_name)));
            }

            let folder_options = vec![
                "⚡ Clean single folder: Simplify into 1 clean folder with new name (Recommended)",
                "📁 Rename all folder layers: Rename every folder and subfolder to new name",
                "📂 Keep original folder names unchanged",
            ];

            let chosen_folder_mode = Select::new(
                "How should folders inside the ZIP be named?",
                folder_options,
            )
            .prompt()?;

            if chosen_folder_mode.starts_with("⚡ Clean single folder") {
                rules.rename_zip_folders = true;
                rules.flatten_zip_folders = true;
            } else if chosen_folder_mode.starts_with("📁 Rename all folder layers") {
                rules.rename_zip_folders = true;
                rules.flatten_zip_folders = false;
            } else {
                rules.rename_zip_folders = false;
                rules.flatten_zip_folders = false;
            }
        }
    } else {
        // Custom Rules Workflow
        println!("\n{}", "🔧 Advanced Custom Rules:".cyan().bold());

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
            "Select actions to apply (Space to toggle, Enter to confirm):",
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
                format!("Success! Renamed {} items in '{}'.", count, final_zip.display())
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
