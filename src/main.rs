use core::arch;
use std::{path::{PathBuf, Path}, fs::{DirEntry, self}, io::{self, Write}, ops::Add};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, Arg, Args};

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
    Create(Create),
    Mv(Mv),
    MvDir(MvDir)
}

#[derive(Args,Debug)]
#[clap(about = r#"
The 'create' command automates the generation of a structured summary file for Markdown (md) documents within a book directory.
It sequentially reads numbered md files and directories (e.g., "0001_") to define their order in the summary. 
If any md files or directories do not have a number, they are ignored. If any directories do not have a 'README.md' they
are ignored, but the descendants are still searched.
This numbering resets for each subdirectory, reflecting their hierarchical structure in the summary.
Notably, 'README.md' files are exempt from numbering and are always placed first (index 0).
The command also translates directory names into section headers, by removing underscores, with 'README.md' as the content.
The depth of directories dictating the nesting of sections.
File names in the summary exclude these numeric prefixes, ensuring a clean, readable format.
"#)]
struct Create {
    /// A list of all files or directories (including their subdirectories) to ignore.
    #[arg(short,long)]
    ignore: Vec<PathBuf>,

    /// Path to the directory to source the md files
    #[arg(short,long, default_value = ".")]
    sourcing_dir: PathBuf,

    /// Path to the directory to place the SUMMARY.md file
    #[arg(short,long, default_value = ".")]
    output_dir: PathBuf,

    /// If set, directories without numbers will be placed in the summary in the order they are encountered.
    #[arg(long)]
    include_unnumbered_directories: bool,

    /// If set, if a directory is encountered without a readme, it and all it's descendants are ignored.
    #[arg(long)]
    skip_directories_without_readme: bool,
}


#[derive(Args,Debug)]
#[clap(about = r#"
The 'mv' command facilitates reorganizing md files within the book's structure. 
It allows moving a file to a specified position (index) in a different directory. 
The index numbering starts at 1, with 'README.md' being an exception as it's always considered the first item (index 0).
When a file is moved to an occupied index, the existing file, and those following it,
are automatically shifted down to accommodate the new file, and the original directory is updated as well.
Summary is not updated.
"#)]
struct Mv {
    /// The path to the file to move
    from_file: PathBuf,

    /// The path to the directory to move to
    to_dir: PathBuf,

    /// The index to put in the new directory. Must be greater than or equal to one. Note: "README.md" files are always first and considerd index 0.
    index: u32,

    /// If provided, at the end the summary file will not be updated
    #[arg(long)]
    do_not_update_summary: bool,
}

#[derive(Args,Debug)]
#[clap(about = r#"
Works in the same way as 'mv' command, except works on directories instead of files.
"#)]
struct MvDir {
    /// The path to the directory to move
    from_dir: PathBuf,

    /// The path to the directory to move to
    to_dir: PathBuf,

    /// The index to put in the new directoy. Must be greater than or equal to one.
    index: u32,

    /// If provided, at the end the summary file will not be updated
    #[arg(long)]
    do_not_update_summary: bool,
}

fn main() -> Result<()> {
    let cmd = Cli::parse();

    match cmd.command {
        Subcommands::Mv(mv) => mv_command(mv),
        Subcommands::Create(create) => create_command(create),
        Subcommands::MvDir(mv_dir) => mv_dir_command(mv_dir),
    };
    //todo
    
    Ok(())
}

fn create_command(create: Create) -> Result<()> {
    let absolute_path = match fs::canonicalize(&create.sourcing_dir) {
        Ok(absolute_path) => absolute_path,
        Err(e) => bail!("The provided path to the book is not a real path for: {}", e), //todo check book is there
    };
    let mut summary_content = String::new();

    fn process_directory(dir: &Path, create: &Create, summary_content: &mut String, nest_level: usize) -> io::Result<()> {
        if create.ignore.contains(&PathBuf::from(dir)) {
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

        let include_this_directory = (is_numbered_directory || create.include_unnumbered_directories) && readme_path.exists();
        if include_this_directory {
            next_nest_level += 1;
            summary_content.push_str(&format!("{}- [{}]({})\n", 
            indentation,
            section_header,
            readme_path.display()));
            indentation = indentation.add("\t");
        }


        let mut entries = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect::<Vec<DirEntry>>();

        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                process_directory(&path, &create,  summary_content, next_nest_level)?;
            } else if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
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

        Ok(())
    }

    process_directory(&absolute_path, &create, &mut summary_content, 0)?;

    let summary_path = absolute_path.join("SUMMARY.md");
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


fn mv_command (mv: Mv) -> Result<()> {
    todo!()
}

fn mv_dir_command (mv_dir: MvDir) -> Result<()> {
    todo!()
}