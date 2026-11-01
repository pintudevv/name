use anyhow::{Context, Result};
use colored::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::rules::RenameRules;

#[derive(Debug, Clone)]
pub struct RenameItem {
    pub original_path: PathBuf,
    pub original_name: String,
    pub new_name: String,
    pub new_path: PathBuf,
    pub is_changed: bool,
    pub conflict_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RenamePlan {
    pub items: Vec<RenameItem>,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub error_count: usize,
}

pub fn plan_directory_rename(
    target_dir: &Path,
    rules: &RenameRules,
    recursive: bool,
) -> Result<RenamePlan> {
    if !target_dir.is_dir() {
        anyhow::bail!("Target path '{}' is not a directory", target_dir.display());
    }

    let mut entries: Vec<PathBuf> = Vec::new();
    if recursive {
        for entry in WalkDir::new(target_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                entries.push(entry.into_path());
            }
        }
    } else {
        for entry in fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                entries.push(path);
            }
        }
    }

    // Sort entries naturally for consistent numbering
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut items = Vec::new();
    let mut matching_index = 0;

    // Track targets within each parent directory to check for collisions
    let mut planned_names_per_dir: HashMap<PathBuf, HashSet<String>> = HashMap::new();

    for path in &entries {
        let original_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let parent = path.parent().unwrap_or(target_dir).to_path_buf();

        if !rules.matches_extension(&original_name) {
            items.push(RenameItem {
                original_path: path.clone(),
                original_name: original_name.clone(),
                new_name: original_name.clone(),
                new_path: path.clone(),
                is_changed: false,
                conflict_error: None,
            });
            continue;
        }

        let new_name = match rules.transform_filename(&original_name, matching_index) {
            Ok(name) => name,
            Err(e) => {
                items.push(RenameItem {
                    original_path: path.clone(),
                    original_name: original_name.clone(),
                    new_name: original_name.clone(),
                    new_path: path.clone(),
                    is_changed: false,
                    conflict_error: Some(format!("Rule error: {}", e)),
                });
                continue;
            }
        };

        matching_index += 1;
        let is_changed = new_name != original_name;
        let new_path = parent.join(&new_name);

        let mut conflict_error = None;

        if is_changed {
            let dir_set = planned_names_per_dir.entry(parent.clone()).or_default();
            let lower_new = new_name.to_lowercase();
            if dir_set.contains(&lower_new) {
                conflict_error = Some(format!("Collision: duplicate output name '{}'", new_name));
            } else {
                dir_set.insert(lower_new);
            }

            // Check if a file already exists at new_path that is NOT part of this renaming batch
            if conflict_error.is_none() && new_path.exists() {
                // If it exists, but it's the same file (e.g. only case change on Windows like a.txt -> A.txt), allow it
                let is_same_file = match (fs::canonicalize(path), fs::canonicalize(&new_path)) {
                    (Ok(c1), Ok(c2)) => c1 == c2,
                    _ => false,
                };
                if !is_same_file && !entries.contains(&new_path) {
                    conflict_error = Some(format!(
                        "Collision: target file '{}' already exists on disk",
                        new_name
                    ));
                }
            }
        }

        items.push(RenameItem {
            original_path: path.clone(),
            original_name,
            new_name,
            new_path,
            is_changed,
            conflict_error,
        });
    }

    let changed_count = items.iter().filter(|i| i.is_changed && i.conflict_error.is_none()).count();
    let unchanged_count = items.iter().filter(|i| !i.is_changed && i.conflict_error.is_none()).count();
    let error_count = items.iter().filter(|i| i.conflict_error.is_some()).count();

    Ok(RenamePlan {
        items,
        changed_count,
        unchanged_count,
        error_count,
    })
}

pub fn print_preview(plan: &RenamePlan, max_display: usize) {
    println!("\n{}", "=== RENAMING PREVIEW ===".bold().cyan());
    let changed_items: Vec<&RenameItem> = plan.items.iter().filter(|i| i.is_changed).collect();

    if changed_items.is_empty() {
        println!("{}", "No files will be renamed with current rules.".yellow());
        return;
    }

    let display_items = if max_display > 0 {
        &changed_items[..changed_items.len().min(max_display)]
    } else {
        &changed_items[..]
    };

    println!("{:<4} {:<40} -> {:<40}", "#".bold(), "Original Name".bold(), "New Name".bold());
    println!("{}", "-".repeat(88).bright_black());

    for (i, item) in display_items.iter().enumerate() {
        let orig_display = truncate_str(&item.original_name, 38);
        let new_display = truncate_str(&item.new_name, 38);

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
                "... and {} more files to rename (omitted in preview).",
                changed_items.len() - display_items.len()
            )
            .bright_black()
        );
    }

    println!("{}", "-".repeat(88).bright_black());
    println!(
        "Summary: {} to rename, {} unchanged, {} conflicts\n",
        plan.changed_count.to_string().green().bold(),
        plan.unchanged_count.to_string().yellow(),
        if plan.error_count > 0 {
            plan.error_count.to_string().red().bold()
        } else {
            plan.error_count.to_string().normal()
        }
    );
}

pub fn execute_directory_rename(plan: &RenamePlan) -> Result<usize> {
    if plan.error_count > 0 {
        anyhow::bail!("Cannot proceed: there are {} conflict errors.", plan.error_count);
    }

    let to_rename: Vec<&RenameItem> = plan.items.iter().filter(|i| i.is_changed).collect();
    if to_rename.is_empty() {
        return Ok(0);
    }

    // Two-pass rename to prevent cyclic collision and Windows case-insensitivity issues
    // Step 1: Rename everything to a unique temporary uuid/name
    let mut temp_mapping = Vec::new();
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    for (idx, item) in to_rename.iter().enumerate() {
        let parent = item.original_path.parent().unwrap();
        let temp_name = format!(".__nc_tmp_{}_{}.tmp", epoch, idx);
        let temp_path = parent.join(temp_name);
        fs::rename(&item.original_path, &temp_path).with_context(|| {
            format!(
                "Failed to move '{}' to temp staging '{}'",
                item.original_path.display(),
                temp_path.display()
            )
        })?;
        temp_mapping.push((temp_path, item.new_path.clone(), &item.original_name, &item.new_name));
    }

    // Step 2: Rename from temp to target name
    let mut success_count = 0;
    for (temp_path, final_path, _orig_name, _new_name) in temp_mapping {
        fs::rename(&temp_path, &final_path).with_context(|| {
            format!(
                "Failed to rename staging '{}' to final target '{}'",
                temp_path.display(),
                final_path.display()
            )
        })?;
        success_count += 1;
    }

    Ok(success_count)
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
