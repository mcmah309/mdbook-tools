use std::{path::{PathBuf, Path}, fs::{DirEntry, self}, io::{self, Write}, ops::Add};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, Args};

#[derive(Parser, Debug)]
#[command(author = "mcmah309", version="0.1", about=r#"
mdbook tools for organizing books.
"#)]
struct Cli {
    /// commands
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand,Debug)]
enum Subcommands {
    Summary(Summary),
    Mv(Mv),
}

#[derive(Args,Debug)]
#[clap(about = r#"
The 'summary' command automates the generation of a structured summary file.
It sequentially reads numbered md files and directories (e.g., "0001_") to define their order in the summary. 
If any md files or directories do not have a number, they are ignored by default. 
Directories use their 'README.md' file to define their content in the summary. As such, directories without a 'README.md' are ignored,
but the descendants are still searched.
Numbering resets for each directory, reflecting their hierarchical structure in the summary.
'README.md' files are exempt from numbering.
Section headers are created by removing underscores and capitalizing the first letter of each word.
Section names exclude numeric prefixes.
"#)]
struct Summary {
    /// A list of all files or directories (including their subdirectories) to ignore.
    #[arg(short,long)]
    ignore: Vec<PathBuf>,

    /// Path to the directory to source the md files
    #[arg(short,long, default_value = ".")]
    sourcing_dir: PathBuf,

    /// Path to the directory to place the SUMMARY.md file
    #[arg(short,long, default_value = ".")]
    output_dir: PathBuf,

    /// If set, directories without numbers, will be placed in the summary in the order they are encountered, 
    /// if other conditions hold.
    #[arg(long)]
    include_unnumbered_directories: bool,

    /// If set, if a section was not created for a directory, the directory's content will be included in the summary,
    /// if other conditions hold.
    #[arg(long)]
    include_directory_content_without_section: bool,
}


#[derive(Args,Debug)]
#[clap(about = r#"
The 'mv' command facilitates reorganizing md files or directories within the book's structure. 
It allows moving a file or directory to a specified position (index) in a different directory. 
The index numbering starts at 1, with 'README.md' is exempt.
When a file is moved to an occupied index, the existing file or directory, and those following it,
are automatically shifted down to accommodate the new file or directory, and the original directory is updated as well.
Summary is not updated.
"#)]
struct Mv {
    /// The path of the file or directory to move
    from: PathBuf,

    /// The path to the directory to move to
    to_dir: PathBuf,

    /// The index to put in the new directory. Must be greater than or equal to one. Note: "README.md" files are always first and considerd index 0.
    index: u32,

    /// Level is used to indicate how many numbers on prefixes of names and files. e.g. "-l 4" indicates "0001_" is the 
    /// type of prefix to expect.
    #[arg(short,long, default_value = "2")]
    level: usize,
}

fn main() -> Result<()> {
    let cmd = Cli::parse();

    match cmd.command {
        Subcommands::Mv(mv) => mv_command(mv),
        Subcommands::Summary(create) => summary_command(create),
    }?;
    //todo
    
    Ok(())
}

fn summary_command(sm: Summary) -> Result<()> {
    let absolute_path = match fs::canonicalize(&sm.sourcing_dir) {
        Ok(absolute_path) => absolute_path,
        Err(e) => bail!("The provided path to the source is not a real path for: {}", e), //todo check book is there
    };

    let summary_file = sm.sourcing_dir.join("SUMMARY.md");
    if summary_file.exists() {
        fs::remove_file(summary_file)?;
    }

    let mut summary_content = String::new();

    fn process_directory(dir: &Path, create: &Summary, summary_content: &mut String, nest_level: usize) -> io::Result<()> {
        if create.ignore.contains(&PathBuf::from(dir)) {
            println!("Skipping directory, ignored: {}", dir.display());
            return Ok(());
        }

        let dir_name = dir.file_name().unwrap().to_str().unwrap();
        let section_header;
        let is_numbered_directory;
        if let Some((_,dir_name_without_number)) = split_number_from_name(dir_name) {
            is_numbered_directory = true;
            section_header = capitalize_first_letter_of_each_word(&dir_name_without_number.replace("_", " "));
        }
        else {
            section_header = capitalize_first_letter_of_each_word(&dir_name.replace("_", " "));
            is_numbered_directory = false;
        }

        let mut indentation = "\t".repeat(nest_level);
        let mut next_nest_level = nest_level;
        let readme_path = dir.join("README.md");

        let include_this_directory_as_section = (is_numbered_directory || create.include_unnumbered_directories) && readme_path.exists();
        if include_this_directory_as_section {
            next_nest_level += 1;
            summary_content.push_str(&format!("{}- [{}]({})\n", 
            indentation,
            section_header,
            readme_path.display()));
            indentation = indentation.add("\t");
        }
        else {
            println!("Skipping directory as section: {}", dir.display());
        }

        let include_this_directory_content = (is_numbered_directory || create.include_unnumbered_directories) && (readme_path.exists() || create.include_directory_content_without_section);

        if !include_this_directory_content {
            println!("Skipping directory's content: {}", dir.display());
        }


        let mut entries = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect::<Vec<DirEntry>>();

        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                process_directory(&path, &create,  summary_content, next_nest_level)?;
            } 
            else if include_this_directory_content {
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    if file_name.ends_with(".md") && file_name != "README.md" {
                        if let Some((_, mut name)) = split_number_from_name(file_name) {
                            name.truncate(name.len() - 3);
                            if let Some((_,name_without_number)) = split_number_from_name(&name) {
                                name = capitalize_first_letter_of_each_word(&name_without_number.replace("_", " "));
                            }
                            else {
                                name = capitalize_first_letter_of_each_word(&name.replace("_", " "));
                            }
                            summary_content.push_str(&format!("{}- [{}]({})\n", indentation, capitalize_first_letter_of_each_word(&name), path.display()));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    process_directory(&absolute_path, &sm, &mut summary_content, 0)?;

    let summary_path = match fs::canonicalize(&sm.output_dir) {
        Ok(absolute_path) => absolute_path.join("SUMMARY.md"),
        Err(e) => bail!("The provided path to the summary output is not a real path for: {}", e), //todo check book is there
    };

    let mut file = fs::File::create(summary_path)?;
    file.write_all(summary_content.as_bytes())?;

    Ok(())
}

/// Splits the prefix if it exists and returns the number and the string. Otherwise none
fn split_number_from_name(name: &str) -> Option<(u32, String)> {
    let parts: Vec<&str> = name.splitn(2, '_').collect();
    if parts.len() == 2 {
        if let Ok(num) = parts[0].parse::<u32>() {
            return Some((num, parts[1].to_string()));
        }
    }
    None
}

fn capitalize_first_letter_of_each_word(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

///////////

fn mv_command(mv: Mv) -> Result<()>{
    if !mv.from.exists() {
        bail!("Source does not exist");
    }

    if !mv.to_dir.exists() || !mv.to_dir.is_dir() {
        bail!("Target directory does not exist or is not a directory");
    }

    if mv.index < 1 {
        bail!("Index must be greater than or equal to one");
    }

    if  !mv.from.is_file() && !mv.from.is_dir() {
        bail!("Source not a file or directory.")
    }

    let old_name = mv.from.file_name().unwrap().to_str().unwrap().to_owned();
    let old_entry: (u32, String, PathBuf) = match split_number_from_name(&old_name) {
        Some((num,name)) => (num, name, mv.from.clone()),
        None => (0,old_name, mv.from.clone()),
    };

    let numbered_entries = get_numbered_entries(&mv.to_dir)?;

    if mv.index as usize > numbered_entries.len() + 1 {
        bail!("Index is greater than one more than the number of current ordered files and directories in the target directory.")
    }

    insert_at( &old_entry, &mv.to_dir, mv.index as usize, numbered_entries, mv.level)?;

    reorder(mv.from.parent().unwrap(), mv.level)?;

    Ok(())
}

/// Gets the number entries sorted and validates that they are in order
fn get_numbered_entries(dir: &Path) -> Result<Vec<(u32, String, DirEntry)>> {
    let mut entries = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let split = split_number_from_name(e.file_name().to_str().unwrap());
            let Some((num, name)) = split else {
                return None;
            };
            Some((num, name, e))
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.0);
    let mut last_num = match entries.first() {
        Some(last) => last.0,
        None => return Ok(entries)
    };
    for (num, _, entry) in entries.iter().skip(1) {
        if  last_num + 1 !=  *num {
            bail!(format!("Numbered entries are not continous. Expected entry number {} got {} for {}", last_num + 1, num, entry.path().canonicalize().unwrap().to_str().unwrap()));
        }
        last_num += 1;
    }

    Ok(entries)
}

fn insert_at(old_dir_entry: &(u32, String, PathBuf), new_dir: &Path, index: usize, prefixed_entries: Vec<(u32, String, DirEntry)>, level: usize) -> Result<()> {
    if index == prefixed_entries.len() + 1 {
        let new_name =  format!("{:0width$}_{}", index, old_dir_entry.1, width = level);
        let new_path = new_dir.join(new_name);
        fs::rename(&old_dir_entry.2, &new_path).map_err(|e| anyhow::anyhow!(e))?;
        return Ok(())
    }
    let mut order_count = 1;
    for (num, name, entry) in prefixed_entries.into_iter() {
        // entry already exists in dest. So do not count this position.
        if old_dir_entry.2 == entry.path() && order_count != index {
            assert!(old_dir_entry.0 == num && old_dir_entry.1 == name);
            continue;
        }
        // This is the file or folder we want to insert at
        if order_count == index {
            let new_name = format!("{:0width$}_{}", index, old_dir_entry.1, width = level);
            let new_path = new_dir.join(new_name);
            fs::rename(&old_dir_entry.2, &new_path).map_err(|e| anyhow::anyhow!(e))?;
            order_count += 1;
        }
        let new_name = format!("{:0width$}_{}", index, old_dir_entry.1, width = level);
        let new_path = new_dir.join(new_name);
        fs::rename(entry.path(), &new_path).map_err(|e| anyhow::anyhow!(e))?;
        order_count += 1;
    }

    Ok(())
}

fn reorder(dir: &Path, level: usize) -> Result<()> {
    let prefixed_entries = get_numbered_entries(dir)?;
    for (index, (_, name, entry)) in prefixed_entries.into_iter().enumerate() {
        let new_name = format!("{:0width$}_{}", index + 1, name, width = level);
        let new_path = dir.join(new_name);
        fs::rename(entry.path(), &new_path).map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok(())
}