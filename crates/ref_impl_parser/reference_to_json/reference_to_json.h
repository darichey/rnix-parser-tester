#ifndef REFERENCE_TO_JSON_H
#define REFERENCE_TO_JSON_H

#ifdef __cplusplus
extern "C" {
#endif

struct Parser;

struct Parser *init_parser();
void destroy_parser(struct Parser *parser);
const char *parse_from_str(struct Parser *parser, const char *nix_expr, bool* ok);
const char *parse_from_file(Parser *parser, const char *file_path, bool* ok);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif