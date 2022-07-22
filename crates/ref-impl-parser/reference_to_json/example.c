#include "stdio.h"
#include "reference_to_json.h"

int main() {
    struct Parser *parser = init_parser();
    const char *nix_expr = "./foo/${\"bar\"}";
    const char *ast_json = parse_from_str(parser, nix_expr);
    printf("%s", ast_json);
    destroy_parser(parser);
}
