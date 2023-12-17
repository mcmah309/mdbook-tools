use core::arch;
use std::{path::{PathBuf, Path}, fs::{DirEntry, self}, io::{self, Write}};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, Arg, Args};

#[derive(Parser, Debug)]
#[command(author = "mcmah309", version="0.1", about=r#"
mdbook tools for organizing books.
"#)]
struct Cli {
    /// Path to the book directory.
    #[arg(short,long, default_value = ".")]
    book_dir: PathBuf,

    /// A list of all files or directories to ignore.
    #[arg(short,long)]
    ignore: Vec<PathBuf>,

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
are ignored.
This numbering resets for each subdirectory, reflecting their hierarchical structure in the summary.
Notably, 'README.md' files are exempt from numbering and are always placed first (index 0).
The command also translates directory names into section headers, by removing underscores, with 'README.md' as the content.
The depth of directories dictating the nesting of sections.
File names in the summary exclude these numeric prefixes, ensuring a clean, readable format.
"#)]
struct Create {

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
Afterwards "Create" is called to create a new summary file.
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
    let rel_path = cmd.book_dir;
    let ignore = cmd.ignore;

    match cmd.command {
        Subcommands::Mv(mv) => mv_command(mv,rel_path,ignore),
        Subcommands::Create(create) => create_command(create, rel_path,ignore),
        Subcommands::MvDir(mv_dir) => mv_dir_command(mv_dir, rel_path, ignore),
    };
    //todo
    
    Ok(())
}





fn mv_command (mv: Mv, rel_path: PathBuf, ignore: Vec<PathBuf>) -> Result<()> {
    //todo
}

fn mv_dir_command (mv_dir: MvDir, rel_path: PathBuf, ignore: Vec<PathBuf>) -> Result<()> {
    //todo
}