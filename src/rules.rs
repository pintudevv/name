use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    Lower,
    Upper,
    Title,
    Snake,
    Kebab,
}

impl CaseMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lower" | "lowercase" => Some(Self::Lower),
            "upper" | "uppercase" => Some(Self::Upper),
            "title" | "titlecase" => Some(Self::Title),
            "snake" | "snake_case" => Some(Self::Snake),
            "kebab" | "kebab-case" => Some(Self::Kebab),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenameRules {
    pub replace_name: Option<String>,
    pub find: Option<String>,
    pub replace: Option<String>,
    pub is_regex: bool,
    pub case_insensitive: bool,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub number_start: Option<usize>,
    pub number_padding: usize,
    pub number_prefix: Option<String>,
    pub case_mode: Option<CaseMode>,
    pub clean_spaces: bool,
    pub new_extension: Option<String>,
    pub filter_extensions: Option<Vec<String>>,
}

impl RenameRules {
    pub fn is_empty(&self) -> bool {
        self.replace_name.is_none()
            && self.find.is_none()
            && self.prefix.is_none()
            && self.suffix.is_none()
            && self.number_start.is_none()
            && self.case_mode.is_none()
            && !self.clean_spaces
            && self.new_extension.is_none()
    }

    pub fn matches_extension(&self, filename: &str) -> bool {
        if let Some(ref allowed) = self.filter_extensions {
            if allowed.is_empty() {
                return true;
            }
            let ext = std::path::Path::new(filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            allowed.iter().any(|a| a.trim_start_matches('.').to_lowercase() == ext)
        } else {
            true
        }
    }

    /// Transforms an existing filename (with extension) into the new filename.
    pub fn transform_filename(&self, original: &str, index: usize) -> Result<String, String> {
        let path = std::path::Path::new(original);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(original);
        let ext = path.extension().and_then(|e| e.to_str());

        let mut new_stem = if let Some(ref base) = self.replace_name {
            base.clone()
        } else {
            stem.to_string()
        };

        // 1. Find & Replace
        if let Some(ref find_pattern) = self.find {
            let replace_with = self.replace.as_deref().unwrap_or("");
            if self.is_regex {
                let re = if self.case_insensitive {
                    Regex::new(&format!("(?i){}", find_pattern))
                } else {
                    Regex::new(find_pattern)
                }.map_err(|e| format!("Invalid regex: {}", e))?;
                new_stem = re.replace_all(&new_stem, replace_with).to_string();
            } else if self.case_insensitive {
                let lower_pattern = find_pattern.to_lowercase();
                let mut result = String::new();
                let mut search_from = 0;
                let lower_stem = new_stem.to_lowercase();
                while let Some(pos) = lower_stem[search_from..].find(&lower_pattern) {
                    let actual_pos = search_from + pos;
                    result.push_str(&new_stem[search_from..actual_pos]);
                    result.push_str(replace_with);
                    search_from = actual_pos + find_pattern.len();
                }
                result.push_str(&new_stem[search_from..]);
                new_stem = result;
            } else {
                new_stem = new_stem.replace(find_pattern, replace_with);
            }
        }

        // 2. Clean spaces and special characters
        if self.clean_spaces {
            // Replace spaces with underscores and remove problematic symbols
            let re_spaces = Regex::new(r"[\s\-]+").unwrap();
            new_stem = re_spaces.replace_all(&new_stem, "_").to_string();
            let re_chars = Regex::new(r"[^\w\.\-]").unwrap();
            new_stem = re_chars.replace_all(&new_stem, "").to_string();
            // Trim underscores
            new_stem = new_stem.trim_matches('_').to_string();
            if new_stem.is_empty() {
                new_stem = "file".to_string();
            }
        }

        // 3. Case transformation
        if let Some(mode) = self.case_mode {
            new_stem = match mode {
                CaseMode::Lower => new_stem.to_lowercase(),
                CaseMode::Upper => new_stem.to_uppercase(),
                CaseMode::Title => to_title_case(&new_stem),
                CaseMode::Snake => to_snake_case(&new_stem),
                CaseMode::Kebab => to_kebab_case(&new_stem),
            };
        }

        // 4. Prefix & Suffix
        if let Some(ref p) = self.prefix {
            new_stem = format!("{}{}", p, new_stem);
        }
        if let Some(ref s) = self.suffix {
            new_stem = format!("{}{}", new_stem, s);
        }

        // 5. Numbering
        if let Some(start) = self.number_start {
            let num = start + index;
            let pad = self.number_padding.max(1);
            let num_str = format!("{:0>width$}", num, width = pad);
            let sep = self.number_prefix.as_deref().unwrap_or("_");
            new_stem = format!("{}{}{}", new_stem, sep, num_str);
        }

        // 6. Extension
        let final_ext = if let Some(ref new_e) = self.new_extension {
            let trimmed = new_e.trim_start_matches('.');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else {
            ext.map(|e| e.to_string())
        };

        let result = match final_ext {
            Some(e) if !e.is_empty() => format!("{}.{}", new_stem, e),
            _ => new_stem,
        };

        Ok(result)
    }
}

fn to_title_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                result.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                result.extend(c.to_lowercase());
            }
        } else {
            result.push(c);
            capitalize_next = true;
        }
    }
    result
}

fn to_snake_case(s: &str) -> String {
    let re = Regex::new(r"[\s\-]+").unwrap();
    let s = re.replace_all(s, "_").to_lowercase();
    let re_chars = Regex::new(r"[^\w\.]").unwrap();
    re_chars.replace_all(&s, "").to_string()
}

fn to_kebab_case(s: &str) -> String {
    let re = Regex::new(r"[\s_]+").unwrap();
    let s = re.replace_all(s, "-").to_lowercase();
    let re_chars = Regex::new(r"[^\w\.\-]").unwrap();
    re_chars.replace_all(&s, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_suffix() {
        let rules = RenameRules {
            prefix: Some("pre_".to_string()),
            suffix: Some("_post".to_string()),
            ..Default::default()
        };
        let res = rules.transform_filename("photo.jpg", 0).unwrap();
        assert_eq!(res, "pre_photo_post.jpg");
    }

    #[test]
    fn test_find_replace() {
        let rules = RenameRules {
            find: Some("temp".to_string()),
            replace: Some("final".to_string()),
            ..Default::default()
        };
        let res = rules.transform_filename("my_temp_file.png", 0).unwrap();
        assert_eq!(res, "my_final_file.png");
    }

    #[test]
    fn test_case_and_clean() {
        let rules = RenameRules {
            clean_spaces: true,
            case_mode: Some(CaseMode::Snake),
            ..Default::default()
        };
        let res = rules.transform_filename("My Special File! (2024).txt", 0).unwrap();
        assert_eq!(res, "my_special_file_2024.txt");
    }

    #[test]
    fn test_change_extension() {
        let rules = RenameRules {
            new_extension: Some("webp".to_string()),
            ..Default::default()
        };
        let res = rules.transform_filename("photo.jpeg", 0).unwrap();
        assert_eq!(res, "photo.webp");
    }

    #[test]
    fn test_filter_extension() {
        let rules = RenameRules {
            filter_extensions: Some(vec!["png".to_string(), "jpg".to_string()]),
            ..Default::default()
        };
        assert!(rules.matches_extension("picture.PNG"));
        assert!(rules.matches_extension("picture.jpg"));
        assert!(!rules.matches_extension("document.pdf"));
    }
}
