# 🏷️ Name Changer

> **A blazing-fast, safe, and portable Bulk File & ZIP Archive Renamer for Windows and CLI.**  
> Built with native Rust. Zero dependencies. ~2.3 MB standalone executable.

---

## ✨ Features

- ⚡ **Instant & Lightweight**: Single native `.exe` binary (~2.3 MB) that starts up in 5 milliseconds. No Python, .NET, or runtime installation required.
- 📦 **Direct ZIP Archive Renaming**: Rename files **inside `.zip` archives** directly without having to extract, rename, and re-compress manually.
- 🧙 **Interactive Wizard Mode**: Double-click `name_changer.exe` in Windows Explorer or run without arguments to get a guided interactive prompt.
- 💻 **Full CLI Support**: Automate batch renaming in scripts, terminal, or scheduled jobs with clean flags.
- 🔍 **Find & Replace / Regex**: Replace words, patterns, or remove tags (e.g. `[1080p]`, `(copy)`). Supports case-insensitive matching and Regular Expressions.
- 🔢 **Sequential Numbering**: Auto-number files with customizable zero-padding (e.g., `img_001.png`, `img_002.png`).
- 🔤 **Case Conversions**: Easily convert filenames to `lowercase`, `UPPERCASE`, `Title Case`, `snake_case`, or `kebab-case`.
- 🧹 **Sanitize & Clean**: Automatically replace spaces and problematic symbols with clean underscores or dashes.
- 🎯 **Extension Filtering**: Target only specific extensions (e.g. `--filter-ext "jpg,png"`).
- 🛡️ **Safe Dry-Run & Conflict Detection**: Preview all planned changes side-by-side (Old Name $\rightarrow$ New Name) before touching a single byte on disk. Detects and blocks collisions.

---

## 🚀 Quick Start

### 1. Interactive Mode (No commands needed)
Simply **double-click** `name_changer.exe` or run:
```bash
name_changer.exe
```
The wizard will guide you through:
1. Entering or dragging-and-dropping a folder or `.zip` file.
2. Selecting which operations to apply (Find & Replace, Prefix/Suffix, Numbering, etc.).
3. Viewing a colored live preview table.
4. Confirming before applying changes.

---

### 2. Command Line (CLI) Examples

#### 📁 Folders & Directories

- **⚡ Quick Rename (Replace All & Number):**
  ```bash
  # Names every file Vacation_001.jpg, Vacation_002.jpg...
  name_changer.exe "D:\photos" --new-name "Vacation" --number 1

  # Inside a ZIP: renames files to Episode_001, Episode_002... AND renames the zip file to 'Season1.zip'
  name_changer.exe "D:\pack.zip" --new-name "Episode" --number 1 --rename-zip "Season1"
  ```

- **Find & Replace text:**
  ```bash
  name_changer.exe "D:\photos" --find "IMG_" --replace "Vacation_"
  ```

- **Remove unwanted text (leave replace empty to delete):**
  ```bash
  name_changer.exe "D:\downloads" --find " (raw)"
  ```

- **Add Prefix & Suffix:**
  ```bash
  name_changer.exe "D:\documents" --prefix "ProjectA_" --suffix "_v1"
  ```

- **Sequential Numbering (001, 002...):**
  ```bash
  name_changer.exe "D:\wallpapers" --prefix "Wall_" --number 1 --pad 3
  ```

- **Sanitize filenames (replace spaces & weird symbols):**
  ```bash
  name_changer.exe "D:\music" --clean --case lower
  ```

- **Target specific file types only:**
  ```bash
  name_changer.exe "D:\assets" --filter-ext "png,jpg" --prefix "web_"
  ```

- **Recursive subfolder processing:**
  ```bash
  name_changer.exe "D:\nested_folder" --recursive --case lower
  ```

- **Dry-run preview (safe test without touching files):**
  ```bash
  name_changer.exe "D:\photos" --find "temp" --replace "final" --dry-run
  ```

- **Force apply without confirmation prompt (for scripts):**
  ```bash
  name_changer.exe "D:\photos" --clean -y
  ```

---

#### 📦 Direct ZIP Archive Renaming

Rename entries directly inside a `.zip` archive in place:

```bash
# Clean up file names and add numbering inside a zip
name_changer.exe "D:\assets.zip" --clean --number 1

# Find & replace text in filenames inside a zip
name_changer.exe "D:\archive.zip" --find "draft" --replace "final"

# Preview changes inside a zip without modifying it
name_changer.exe "D:\archive.zip" --prefix "backup_" --dry-run
```

---

## 🛠️ CLI Options Reference

| Flag | Description |
| :--- | :--- |
| `[TARGET]` or `-t, --target <PATH>` | Target directory or `.zip` file |
| `-f, --find <SEARCH>` | Text or regex to find in filenames |
| `-r, --replace <TEXT>` | Replacement text (omit to delete match) |
| `--regex` | Treat search string as a regular expression |
| `-I, --ignore-case` | Case-insensitive search |
| `-p, --prefix <TEXT>` | Prepend text to filename |
| `-s, --suffix <TEXT>` | Append text before file extension |
| `-n, --number <START>` | Start sequential numbering from `START` |
| `--pad <DIGITS>` | Padding digits for numbers (default: `3`) |
| `--num-sep <SEP>` | Separator before number (default: `_`) |
| `-c, --case <MODE>` | Casing: `lower`, `upper`, `title`, `snake`, `kebab` |
| `--clean` | Replace spaces and symbols with underscores |
| `-e, --ext <EXT>` | Change file extension (e.g., `png`) |
| `--filter-ext <EXTS>` | Filter file types (e.g., `jpg,png,pdf`) |
| `-R, --recursive` | Include subdirectories recursively |
| `-d, --dry-run` | Preview changes without modifying files |
| `-y, --yes` | Apply immediately without confirmation |
| `-i, --interactive` | Force launch interactive wizard |

---

## 📦 Building from Source

To compile the standalone release `.exe` yourself:

```bash
# Clone repository
git clone https://github.com/yourusername/name_changer.git
cd name_changer

# Build optimized binary
cargo build --release
```

The resulting binary will be located at:
```
target/release/name_changer.exe
```

---

## 📄 License

MIT License. Free to use, modify, and distribute.
