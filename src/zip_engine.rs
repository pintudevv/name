use anyhow::{Context, Result};
use colored::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::rules::RenameRules;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ZipRenameItem {
    pub original_entry_name: String,
    pub original_file_name: String,
    pub new_entry_name: String,
    pub new_file_name: String,
    pub is_dir: bool,
    pub is_changed: bool,
    pub conflict_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ZipRenamePlan {
    pub zip_path: std::path::PathBuf,
    pub items: Vec<ZipRenameItem>,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub error_count: usize,
}

pub fn plan_zip_rename(zip_path: &Path, rules: &RenameRules) -> Result<ZipRenamePlan> {
    if !zip_path.is_file() {
        anyhow::bail!("ZIP file not found: '{}'", zip_path.display());
    }

    let file = File::open(zip_path).with_context(|| format!("Cannot open zip file '{}'", zip_path.display()))?;
    let mut archive = ZipArchive::new(file).context("Invalid or corrupted ZIP archive")?;

    let mut items = Vec::new();
    let mut matching_index = 0;
    let mut target_names_set = HashSet::new();

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        let is_dir = entry.is_dir() || entry_name.ends_with('/');

        if is_dir {
            items.push(ZipRenameItem {
                original_entry_name: entry_name.clone(),
                original_file_name: entry_name.clone(),
                new_entry_name: entry_name.clone(),
                new_file_name: entry_name,
                is_dir: true,
                is_changed: false,
                conflict_error: None,
            });
            continue;
        }

        // Separate directory prefix inside zip (e.g. "subfolder/image.png")
        let (dir_prefix, file_name) = match entry_name.rfind('/') {
            Some(idx) => (entry_name[..=idx].to_string(), entry_name[idx + 1..].to_string()),
            None => (String::new(), entry_name.clone()),
        };

        if !rules.matches_extension(&file_name) {
            items.push(ZipRenameItem {
                original_entry_name: entry_name.clone(),
                original_file_name: file_name.clone(),
                new_entry_name: entry_name,
                new_file_name: file_name,
                is_dir: false,
                is_changed: false,
                conflict_error: None,
            });
            continue;
        }

        let new_file_name = match rules.transform_filename(&file_name, matching_index) {
            Ok(name) => name,
            Err(e) => {
                items.push(ZipRenameItem {
                    original_entry_name: entry_name.clone(),
                    original_file_name: file_name.clone(),
                    new_entry_name: entry_name,
                    new_file_name: file_name,
                    is_dir: false,
                    is_changed: false,
                    conflict_error: Some(format!("Rule error: {}", e)),
                });
                continue;
            }
        };

        matching_index += 1;
        let new_entry_name = format!("{}{}", dir_prefix, new_file_name);
        let is_changed = new_entry_name != entry_name;

        let mut conflict_error = None;
        if is_changed {
            let lower_key = new_entry_name.to_lowercase();
            if target_names_set.contains(&lower_key) {
                conflict_error = Some(format!("Collision: duplicate entry '{}'", new_entry_name));
            } else {
                target_names_set.insert(lower_key);
            }
        }

        items.push(ZipRenameItem {
            original_entry_name: entry_name,
            original_file_name: file_name,
            new_entry_name,
            new_file_name,
            is_dir: false,
            is_changed,
            conflict_error,
        });
    }

    let changed_count = items.iter().filter(|i| i.is_changed && i.conflict_error.is_none()).count();
    let unchanged_count = items.iter().filter(|i| !i.is_changed && i.conflict_error.is_none()).count();
    let error_count = items.iter().filter(|i| i.conflict_error.is_some()).count();

    Ok(ZipRenamePlan {
        zip_path: zip_path.to_path_buf(),
        items,
        changed_count,
        unchanged_count,
        error_count,
    })
}

pub fn print_zip_preview(plan: &ZipRenamePlan, max_display: usize) {
    println!("\n{}", format!("=== ZIP RENAMING PREVIEW ({}) ===", plan.zip_path.display()).bold().cyan());
    let changed_items: Vec<&ZipRenameItem> = plan.items.iter().filter(|i| i.is_changed).collect();

    if changed_items.is_empty() {
        println!("{}", "No files inside the ZIP will be renamed with current rules.".yellow());
        return;
    }

    let display_items = if max_display > 0 {
        &changed_items[..changed_items.len().min(max_display)]
    } else {
        &changed_items[..]
    };

    println!("{:<4} {:<40} -> {:<40}", "#".bold(), "Original Zip Entry".bold(), "New Zip Entry".bold());
    println!("{}", "-".repeat(88).bright_black());

    for (i, item) in display_items.iter().enumerate() {
        let orig_display = truncate_str(&item.original_entry_name, 38);
        let new_display = truncate_str(&item.new_entry_name, 38);

        if let Some(ref err) = item.conflict_error {
            println!(
                "{:<4} {:<40} -> {:<40} [{}]",
                (i + 1).to_string().bright_black(),
                orig_display.strikethrough(),
                new_display.red(),
                err.red().bold()
            );
        } else {
            println!(
                "{:<4} {:<40} -> {:<40}",
                (i + 1).to_string().bright_black(),
                orig_display.dimmed(),
                new_display.green().bold()
            );
        }
    }

    if changed_items.len() > display_items.len() {
        println!(
            "{}",
            format!(
                "... and {} more entries to rename (omitted in preview).",
                changed_items.len() - display_items.len()
            )
            .bright_black()
        );
    }

    println!("{}", "-".repeat(88).bright_black());
    println!(
        "Summary: {} to rename in ZIP, {} unchanged, {} conflicts\n",
        plan.changed_count.to_string().green().bold(),
        plan.unchanged_count.to_string().yellow(),
        if plan.error_count > 0 {
            plan.error_count.to_string().red().bold()
        } else {
            plan.error_count.to_string().normal()
        }
    );
}

pub fn execute_zip_rename(plan: &ZipRenamePlan, output_path: Option<&Path>) -> Result<usize> {
    if plan.error_count > 0 {
        anyhow::bail!("Cannot proceed: there are {} conflict errors.", plan.error_count);
    }

    let target_out = match output_path {
        Some(p) => p.to_path_buf(),
        None => plan.zip_path.clone(),
    };

    let parent_dir = plan.zip_path.parent().unwrap_or(Path::new("."));
    let temp_file_path = parent_dir.join(format!(
        ".__nc_tmp_zip_{}.tmp",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    // Open original zip
    let src_file = File::open(&plan.zip_path)?;
    let mut archive = ZipArchive::new(src_file)?;

    // Create temp output zip
    let temp_file = File::create(&temp_file_path)?;
    let mut writer = ZipWriter::new(temp_file);

    let mut renamed_count = 0;
    let mut buffer = Vec::new();

    for (idx, item) in plan.items.iter().enumerate() {
        let mut entry = archive.by_index(idx)?;
        let mut options = SimpleFileOptions::default()
            .compression_method(entry.compression());

        if let Some(mod_time) = entry.last_modified() {
            options = options.last_modified_time(mod_time);
        }

        if item.is_dir {
            writer.add_directory(&item.new_entry_name, options)?;
        } else {
            writer.start_file(&item.new_entry_name, options)?;
            buffer.clear();
            entry.read_to_end(&mut buffer)?;
            writer.write_all(&buffer)?;
            if item.is_changed {
                renamed_count += 1;
            }
        }
    }

    writer.finish()?;
    drop(archive);

    // Atomically replace target
    if temp_file_path.exists() {
        if target_out == plan.zip_path {
            // Overwriting original zip file safely
            std::fs::rename(&temp_file_path, &target_out).or_else(|_| {
                // If direct rename fails due to lock, copy and delete
                std::fs::copy(&temp_file_path, &target_out)?;
                std::fs::remove_file(&temp_file_path)
            })?;
        } else {
            // Renaming the zip file to a new target name
            std::fs::rename(&temp_file_path, &target_out).or_else(|_| {
                std::fs::copy(&temp_file_path, &target_out)?;
                std::fs::remove_file(&temp_file_path)
            })?;
            // Remove original zip file since it has been renamed
            if plan.zip_path.exists() && plan.zip_path != target_out {
                let _ = std::fs::remove_file(&plan.zip_path);
            }
        }
    }

    Ok(renamed_count)
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
