# cli
A CLI tool for interacting with rnix-parser-tester.

All subcommands accept either files or directories (possibly from NIX_PATH using angle brackets) or reading from stdin.

```
cli
Utility program to test/use various aspects of rnix-parser-tester

USAGE:
    cli <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    compare    Report differences in serialization between the reference Nix parser and
                   rnix-parser
    dump       Dump information about the given Nix expression
    help       Print this message or the help of the given subcommand(s)
    summary    Perform analysis of summaries generated by the compare subcommand
```

## `compare`
```
cli-compare 
Report differences in serialization between the reference Nix parser and rnix-parser

USAGE:
    cli compare [OPTIONS] [FILE]

ARGS:
    <FILE>    The Nix file, or directory of Nix files, to parse. If not given, will read from
              stdin

OPTIONS:
    -h, --help
            Print help information

    -r, --recursive
            If the given file is a directory, recurse into subdirectories

        --save-summary <SAVE_SUMMARY>
            Save a machine-readable summary of the comparison results to the given file
```

The output is a list of file paths and the result of comparing the reference impl and rnix-parser parses of that file. The result is one of...

* Equal: the parses were the same
* Not equal: the parses were not the same
* Reference impl error: an error was thrown while parsing using the reference impl
* rnix-parser error: an error was thrown while parsing using rnix-parser

The saved summary is simply a json object containing arrays of paths for each result.

## `dump`
```
cli-dump 
Dump information about the given Nix expression

USAGE:
    cli dump [OPTIONS] [FILE]

ARGS:
    <FILE>    The Nix file, or directory of Nix files, to parse. If not given, will read from
              stdin

OPTIONS:
    -h, --help               Print help information
    -p, --parser <PARSER>    Which parser to use when parsing (can specify multiple!) [possible
                             values: reference, rnix]
    -r, --recursive          If the given file is a directory, recurse into subdirectories
```

The output is the JSON representation of the normalized AST for the given parsers.

## `summary`
```
cli-summary 
Perform analysis of summaries generated by the compare subcommand

USAGE:
    cli summary <SUMMARY_BEFORE> <SUMMARY_AFTER>

ARGS:
    <SUMMARY_BEFORE>    
    <SUMMARY_AFTER>     

OPTIONS:
    -h, --help    Print help information
```