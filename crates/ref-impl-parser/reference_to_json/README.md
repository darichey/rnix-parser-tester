# reference_to_json
A small C++ library which uses the reference implementation of the Nix language parser to parse a Nix expression and output the AST as JSON. This is primarily intended to be used by the parent Rust project via FFI.

The main function of interest is...
```cpp
const char *nix_expr_to_json_str(struct Parser *parser, const char *nix_expr);
```

## Example
```c
#include "stdio.h"
#include "reference_to_json.h"

int main() {
    struct Parser *parser = init_parser();
    const char *nix_expr = "1-1";
    const char *ast_json = nix_expr_to_json_str(parser, nix_expr);
    printf("%s", ast_json);
    destroy_parser(parser);
}
```
```json
$ ./example | jq
{
  "args": [
    {
      "type": "Int",
      "value": 1
    },
    {
      "type": "Int",
      "value": 1
    }
  ],
  "fun": {
    "type": "Var",
    "value": "__sub"
  },
  "type": "Call"
}
```

## Build
```
make
```

Note that the parent Rust project takes care of building via `rs-cc` in [build.rs](../build.rs), so you only need to manually build this if you want to use it in another context.

Build the example with `make example`.
