#include "stdio.h"
#include "reference_to_json.h"

int main() {
    struct Parser *parser = init_parser();
    const char *nix_expr = "x: x ? y.z";
    const char *ast_json = nix_expr_to_json_str(parser, nix_expr);
    printf("%s", ast_json);
    destroy_parser(parser);
}
