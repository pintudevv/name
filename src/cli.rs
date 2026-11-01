use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "name_changer",
    author = "Name Changer Contributors",
    version = "0.1.0",
    about = "Lightning-fast bulk file & ZIP archive renamer for Windows & cross-platform",
    long_about = "A high-performance CLI and interactive tool to bulk rename files in directories and inside ZIP archives.\nSupports find & replace, regex, prefix/suffix, sequential numbering, casing conversions, and safe dry-run preview."
)]
pub struct CliArgs {
    /// Target directory or .zip file to process (if omitted, launches interactive wizard)
    #[arg(value_name = "TARGET")]
    pub target: Option<PathBuf>,

    /// Target directory or .zip file (flag alternative)
    #[arg(short = 't', long = "target", value_name = "PATH")]
    pub target_flag: Option<PathBuf>,

    /// Text or pattern to search for in filenames
    #[arg(short = 'f', long = "find", value_name = "SEARCH")]
    pub find: Option<String>,

    /// Replacement text for matches (defaults to empty string to delete matches)
    #[arg(short = 'r', long = "replace", value_name = "TEXT", default_missing_value = "", num_args = 0..=1)]
    pub replace: Option<String>,

    /// Treat find pattern as a regular expression
    #[arg(long = "regex")]
    pub is_regex: bool,

    /// Case-insensitive search
    #[arg(short = 'I', long = "ignore-case")]
    pub ignore_case: bool,

    /// Add a prefix to the start of filenames
    #[arg(short = 'p', long = "prefix", value_name = "PREFIX")]
    pub prefix: Option<String>,

    /// Add a suffix to filenames (before the extension)
    #[arg(short = 's', long = "suffix", value_name = "SUFFIX")]
    pub suffix: Option<String>,

    /// Add sequential numbers starting at START (e.g., --number 1)
    #[arg(short = 'n', long = "number", value_name = "START")]
    pub number: Option<usize>,

    /// Number of digits for zero-padding in numbering (default: 3, e.g. 001)
    #[arg(long = "pad", default_value = "3", value_name = "DIGITS")]
    pub pad: usize,

    /// Separator placed before sequential number (default: '_')
    #[arg(long = "num-sep", default_value = "_", value_name = "SEP")]
    pub num_sep: String,

    /// Convert filename casing: lower, upper, title, snake, kebab
    #[arg(short = 'c', long = "case", value_name = "MODE")]
    pub case: Option<String>,

    /// Sanitize filenames: replace spaces and weird symbols with underscores
    #[arg(long = "clean")]
    pub clean: bool,

    /// Change or standardize file extension (e.g., 'png' or 'jpg')
    #[arg(short = 'e', long = "ext", value_name = "EXTENSION")]
    pub new_ext: Option<String>,

    /// Comma-separated list of extensions to filter (e.g., 'jpg,png,pdf')
    #[arg(long = "filter-ext", value_name = "EXTENSIONS")]
    pub filter_ext: Option<String>,

    /// Recursively process subdirectories (for folders)
    #[arg(short = 'R', long = "recursive")]
    pub recursive: bool,

    /// Preview changes without modifying any files (dry-run)
    #[arg(short = 'd', long = "dry-run")]
    pub dry_run: bool,

    /// Apply renames immediately without interactive confirmation prompt
    #[arg(short = 'y', long = "yes")]
    pub force: bool,

    /// Force launch interactive wizard even if target or flags are specified
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,
}

impl CliArgs {
    pub fn get_target(&self) -> Option<PathBuf> {
        self.target.clone().or_else(|| self.target_flag.clone())
    }

    /// Determines if no arguments were passed, which should trigger interactive mode
    pub fn is_empty_run(&self) -> bool {
        self.target.is_none()
            && self.target_flag.is_none()
            && self.find.is_none()
            && self.replace.is_none()
            && self.prefix.is_none()
            && self.suffix.is_none()
            && self.number.is_none()
            && self.case.is_none()
            && !self.clean
            && self.new_ext.is_none()
    }
}
