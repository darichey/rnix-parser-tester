# reference_to_json
A small C++ program which uses the reference implementation of the Nix language parser to parse a Nix expression and output the AST as JSON.

## Example
```
$ ./reference_to_json <<< "1-1"
["Call",["Var","__sub"],[["Int",1],["Int",1]]]
```

## Build
```
make
```