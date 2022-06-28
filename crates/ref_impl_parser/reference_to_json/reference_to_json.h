#ifndef REFERENCE_TO_JSON_H
#define REFERENCE_TO_JSON_H

#ifdef __cplusplus
extern "C" {
#endif

struct Parser;

struct Parser *init_parser();
void destroy_parser(struct Parser *parser);
const char *nix_expr_to_json_str(struct Parser *parser, const char *nix_expr);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif