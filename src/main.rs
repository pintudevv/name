mod cli;
mod engine;
mod rules;
mod wizard;
mod zip_engine;

use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use inquire::Confirm;
use std::path::PathBuf;

use cli::CliArgs;
use engine::{execute_directory_rename, plan_directory_rename, print_preview};
use rules::{CaseMode, RenameRules};
use wizard::run_interactive_wizard;
use zip_engine::{execute_zip_rename, plan_zip_rename, print_zip_preview};

fn main() -> Result<()> {
    // Enable ANSI color escape codes on Windows consoles
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let args = CliArgs::parse();

    // Check if user launched by double-clicking without CLI args or requested interactive mode
    if args.is_empty_run() || args.interactive {
        return run_interactive_wizard();
    }

    // CLI Mode
    let target = match args.get_target() {
        Some(t) => t,
        None => {
            eprintln!("{}", "Error: No target path or directory provided.".red().bold());
            eprintln!("Run with --help for usage instructions, or run without flags for interactive wizard.");
            std::process::exit(1);
        }
    };

    if !target.exists() {
        eprintln!("{}", format!("Error: Target path '{}' does not exist.", target.display()).red().bold());
        std::process::exit(1);
    }

    let rules = RenameRules {
        replace_name: args.new_name,
        rename_zip_folders: true,
        flatten_zip_folders: args.flatten,
        find: args.find.clone(),
        replace: args.replace.or_else(|| if args.find.is_some() { Some(String::new()) } else { None }),
        is_regex: args.is_regex,
        case_insensitive: args.ignore_case,
        prefix: args.prefix,
        suffix: args.suffix,
        number_start: args.number,
        number_padding: args.pad,
        number_prefix: Some(args.num_sep),
        case_mode: args.case.as_deref().and_then(CaseMode::from_str),
        clean_spaces: args.clean,
        new_extension: args.new_ext,
        filter_extensions: args.filter_ext.map(|s| {
            s.split(',')
                .map(|item| item.trim().trim_start_matches('.').to_string())
                .filter(|item| !item.is_empty())
                .collect()
        }),
    };

    if rules.is_empty() {
        eprintln!("{}", "Warning: No renaming rules were specified (e.g. --find, --prefix, --number).".yellow());
        eprintln!("Run 'name_changer --help' to see all available options, or use without arguments for guided wizard.");
        std::process::exit(1);
    }

    let is_zip = is_zip_file(&target);

    if is_zip {
        println!("{}", format!("Processing ZIP archive: {}", target.display()).cyan());
        let plan = plan_zip_rename(&target, &rules)?;
        print_zip_preview(&plan, 50);

        if plan.changed_count == 0 {
            println!("{}", "No files inside ZIP matched the criteria.".yellow());
            return Ok(());
        }

        if plan.error_count > 0 {
            eprintln!("{}", format!("Aborting: {} naming conflict(s) detected.", plan.error_count).red().bold());
            std::process::exit(1);
        }

        if args.dry_run {
            println!("{}", "[DRY RUN] No files inside the ZIP were modified.".yellow().bold());
            return Ok(());
        }

        let new_zip_target = if let Some(ref nz) = args.rename_zip {
            let clean = nz.trim().trim_end_matches(".zip");
            let new_filename = format!("{}.zip", clean);
            Some(target.parent().unwrap_or(std::path::Path::new(".")).join(new_filename))
        } else {
            None
        };

        if let Some(ref nzt) = new_zip_target {
            println!("{}", format!("ZIP file will also be renamed to: '{}'", nzt.file_name().unwrap().to_string_lossy()).cyan());
        }

        if !args.force {
            let confirm = Confirm::new("Apply these changes to the ZIP archive?")
                .with_default(true)
                .prompt()?;
            if !confirm {
                println!("{}", "Operation cancelled.".yellow());
                return Ok(());
            }
        }

        let renamed = execute_zip_rename(&plan, new_zip_target.as_deref())
            .context("Failed executing zip rename")?;
        let final_display = new_zip_target.as_ref().unwrap_or(&target);
        println!("{}", format!("Successfully renamed {} files in '{}'!", renamed, final_display.display()).green().bold());
    } else if target.is_dir() {
        println!("{}", format!("Processing directory: {}", target.display()).cyan());
        let plan = plan_directory_rename(&target, &rules, args.recursive)?;
        print_preview(&plan, 50);

        if plan.changed_count == 0 {
            println!("{}", "No files matched the criteria.".yellow());
            return Ok(());
        }

        if plan.error_count > 0 {
            eprintln!("{}", format!("Aborting: {} naming conflict(s) detected.", plan.error_count).red().bold());
            std::process::exit(1);
        }

        if args.dry_run {
            println!("{}", "[DRY RUN] No files were renamed on disk.".yellow().bold());
            return Ok(());
        }

        if !args.force {
            let confirm = Confirm::new("Apply these renames?")
                .with_default(true)
                .prompt()?;
            if !confirm {
                println!("{}", "Operation cancelled.".yellow());
                return Ok(());
            }
        }

        let renamed = execute_directory_rename(&plan)
            .context("Failed executing directory rename")?;
        println!("{}", format!("Successfully renamed {} files in '{}'!", renamed, target.display()).green().bold());
    } else {
        eprintln!("{}", format!("'{}' is neither a directory nor a .zip file.", target.display()).red());
        std::process::exit(1);
    }

    Ok(())
}

fn is_zip_file(path: &PathBuf) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        ext.eq_ignore_ascii_case("zip")
    } else {
        false
    }
}
