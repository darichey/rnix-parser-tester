use std::{
    collections::HashSet,
    env,
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};

use clap::{clap_derive::ArgEnum, Parser, Subcommand};
use globwalk::GlobWalkerBuilder;

use parser_tester_cli::{assert_parses_eq_no_panic, get_ref_impl_json, get_rnix_json};
use serde::{Deserialize, Serialize};

/// Utility program to test/use various aspects of rnix-parser-tester
#[derive(Parser)]
#[clap()]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[clap()]
enum Commands {
    /// Dump information about the given Nix expression
    Dump {
        /// The Nix file, or directory of Nix files, to parse. If not given, will read from stdin
        #[clap(value_parser)]
        file: Option<String>,

        /// If the given file is a directory, recurse into subdirectories
        #[clap(short, long, value_parser)]
        recursive: bool,

        /// Which parser to use when parsing (can specify multiple!)
        #[clap(short, long, value_parser)]
        parser: Vec<ParserImpl>,
    },
    /// Report differences in serialization between the reference Nix parser and rnix-parser
    Compare {
        /// The Nix file, or directory of Nix files, to parse. If not given, will read from stdin
        #[clap(value_parser)]
        file: Option<String>,

        /// If the given file is a directory, recurse into subdirectories
        #[clap(short, long, value_parser)]
        recursive: bool,

        /// Save a machine-readable summary of the comparison results to the given file
        #[clap(long, value_parser)]
        save_summary: Option<PathBuf>,
    },
    /// Perform analysis of summaries generated by the compare subcommand
    Summary {
        #[clap(value_parser)]
        summary_before: PathBuf,

        #[clap(value_parser)]
        summary_after: PathBuf,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum ParserImpl {
    Reference,
    Rnix,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command {
        Commands::Dump {
            file,
            parser,
            recursive,
        } => {
            for (file, input) in walk(file, recursive)? {
                dump(file, &input?, &parser)?;
            }
        }
        Commands::Compare {
            file,
            recursive,
            save_summary,
        } => {
            let mut equal = HashSet::new();
            let mut not_equal = HashSet::new();

            for (file, input) in walk(file, recursive)? {
                print!("{file} ... ");
                io::stdout().flush()?;

                if let Err(_) = assert_parses_eq_no_panic(input?) {
                    println!("\x1b[31mNOT EQUAL\x1b[0m");
                    if save_summary.is_some() {
                        not_equal.insert(file);
                    }
                } else {
                    println!("\x1b[32mequal\x1b[0m");
                    if save_summary.is_some() {
                        equal.insert(file);
                    }
                }
            }

            match save_summary {
                Some(summary_file) => serde_json::to_writer_pretty(
                    File::create(summary_file)?,
                    &Summary { equal, not_equal },
                )?,
                None => {}
            }
        }
        Commands::Summary {
            summary_before,
            summary_after,
        } => {
            let summary_before: Summary = serde_json::from_reader(File::open(summary_before)?)?;
            let summary_after: Summary = serde_json::from_reader(File::open(summary_after)?)?;

            let progressions = summary_before.not_equal.intersection(&summary_after.equal);
            let regressions = summary_before.equal.intersection(&summary_after.not_equal);

            let mut num_progressions = 0;
            let mut num_regressions = 0;

            println!("== Progressions (not equal before, equal after) ==");
            for file in progressions {
                println!("{file}");
                num_progressions += 1;
            }

            println!();

            println!("== Regressions (equal before, not equal after) ==");
            for file in regressions {
                println!("{file}");
                num_regressions += 1;
            }

            println!();

            println!("== Summary ==");
            println!("# equal before: {}", summary_before.equal.len());
            println!("# not equal before: {}", summary_before.not_equal.len());
            println!();
            println!("# equal after: {}", summary_after.equal.len());
            println!("# not equal after: {}", summary_after.not_equal.len());
            println!();
            println!("# progressions: {num_progressions}");
            println!("# regressions: {num_regressions}");
        }
    }

    Ok(())
}

fn walk(
    file: Option<String>,
    recursive: bool,
) -> Result<Box<dyn Iterator<Item = (String, Result<String, io::Error>)>>, Box<dyn Error>> {
    match file {
        Some(file) => {
            let file = normalize(file)?;

            if recursive && !file.is_dir() {
                return Err(AppError::UsageError(format!(
                    "{} isn't a directory. Can't recurse.",
                    file.display()
                )))?;
            }

            if file.is_dir() {
                Ok(Box::new(
                    GlobWalkerBuilder::new(file, "*.nix")
                        .max_depth(if recursive { usize::MAX } else { 1 })
                        .build()?
                        .into_iter()
                        .filter_map(Result::ok)
                        .map(|nix_file| {
                            (
                                nix_file.path().display().to_string(),
                                fs::read_to_string(nix_file.path()),
                            )
                        }),
                ))
            } else {
                Ok(Box::new(std::iter::once((
                    file.display().to_string(),
                    fs::read_to_string(file),
                ))))
            }
        }
        None => Ok(Box::new(std::iter::once((
            "<input from stdin>".to_string(),
            read_stdin(),
        )))),
    }
}

fn normalize(file: String) -> Result<PathBuf, Box<dyn Error>> {
    if let Some(file) = file.strip_prefix("<") {
        if let Some(file) = file.strip_suffix(">") {
            let mut path = path_to_nixpkgs()?;
            path.push(file);
            return Ok(path);
        }
    }

    Ok(PathBuf::from(file))
}

fn path_to_nixpkgs() -> Result<PathBuf, Box<dyn Error>> {
    let path = env::var("NIX_PATH")?;
    let nixpkgs = path
        .split(':')
        .find(|s| s.starts_with("nixpkgs="))
        .ok_or(AppError::CantFindNixpkgs)?;

    Ok(PathBuf::from(&nixpkgs["nixpkgs=".len()..]))
}

fn dump(filename: String, input: &String, parser: &Vec<ParserImpl>) -> Result<(), Box<dyn Error>> {
    println!("{filename} ...");

    if parser.contains(&ParserImpl::Reference) {
        println!("==== Reference impl json ====");
        println!("{}", get_ref_impl_json(input));
        println!();
    }

    if parser.contains(&ParserImpl::Rnix) {
        println!("==== rnix-parser json ====");
        println!("{}", get_rnix_json(input)?);
        println!();
    }

    Ok(())
}

fn read_stdin() -> Result<String, io::Error> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(input)
}

#[derive(Debug)]
enum AppError {
    UsageError(String),
    CantFindNixpkgs,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::UsageError(err) => write!(f, "{err}"),
            AppError::CantFindNixpkgs => write!(f, "Can't find nixpkgs"),
        }
    }
}

impl std::error::Error for AppError {}

#[derive(Deserialize, Serialize)]
struct Summary {
    equal: HashSet<String>,
    not_equal: HashSet<String>,
}