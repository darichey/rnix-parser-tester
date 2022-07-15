use std::{
    env,
    error::Error,
    fs,
    io::{self, Read},
    path::PathBuf,
};

use clap::{clap_derive::ArgEnum, Parser, Subcommand};
use globwalk::GlobWalkerBuilder;

use parser_tester_cli::{assert_parses_eq_no_panic, get_ref_impl_json, get_rnix_json};

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

        /// Which parser to use when parsing (can specify multiple!)
        #[clap(short, long, value_parser)]
        parser: Vec<ParserImpl>,

        /// If the given file is a directory, recurse into subdirectories
        #[clap(short, long, value_parser)]
        recursive: bool,
    },
    /// Report differences in serialization between the reference Nix parser and rnix-parser
    Compare,
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
            if let Some(file) = file {
                let file = normalize(file)?;

                if file.is_dir() {
                    let walker = GlobWalkerBuilder::new(file, "*.nix")
                        .max_depth(if recursive { usize::MAX } else { 1 })
                        .build()?
                        .into_iter()
                        .filter_map(Result::ok);

                    for nix_file in walker {
                        let input = fs::read_to_string(nix_file.path())?;
                        dump(nix_file.path().display(), &input, &parser)?;
                    }
                } else {
                    if recursive {
                        return Err(AppError::UsageError(format!(
                            "{} isn't a directory. Can't recurse.",
                            file.display()
                        )))?;
                    }

                    let input = fs::read_to_string(&file)?;
                    dump(file.display(), &input, &parser)?;
                }
            } else {
                let input = read_stdin()?;
                dump("stdin", &input, &parser)?;
            }
        }
        Commands::Compare => {
            let input = read_stdin()?;
            match assert_parses_eq_no_panic(&input) {
                Ok(()) => println!("Parses equal!"),
                Err(e) => println!("{}", e.to_string()),
            }
        }
    }

    Ok(())
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

fn dump<S>(filename: S, input: &String, parser: &Vec<ParserImpl>) -> Result<(), Box<dyn Error>>
where
    S: std::fmt::Display,
{
    println!("{}", filename);

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
