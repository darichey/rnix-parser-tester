#ifndef REFERENCE_TO_JSON_H
#define REFERENCE_TO_JSON_H

extern "C"
{
    struct NixExpr;

    struct AttrName;
    struct AttrDef;
    struct DynamicAttrDef;
    struct Formal;
    struct Formals;

    extern "C" NixExpr *mk_int(int64_t value);
    extern "C" NixExpr *mk_float(double_t value);
    extern "C" NixExpr *mk_string(const char *value);
    extern "C" NixExpr *mk_path(const char *value);
    extern "C" NixExpr *mk_var(const char *value);
    extern "C" NixExpr *mk_select(NixExpr *subject, NixExpr *or_default, AttrName **path, size_t path_len);
    extern "C" NixExpr *mk_op_has_attr(NixExpr *subject, AttrName **path, size_t path_len);
    extern "C" NixExpr *mk_attrs(bool rec, AttrDef **attrs, size_t attrs_len, DynamicAttrDef **dynamic_attrs, size_t dynamic_attrs_len);
    extern "C" NixExpr *mk_list(NixExpr **elems, size_t elems_len);
    extern "C" NixExpr *mk_lambda(const char **arg, Formals *formals, NixExpr *body);
    extern "C" NixExpr *mk_call(NixExpr *fun, NixExpr **args, size_t args_len);
    extern "C" NixExpr *mk_let(NixExpr *attrs, NixExpr *body);
    extern "C" NixExpr *mk_with(NixExpr *attrs, NixExpr *body);
    extern "C" NixExpr *mk_if(NixExpr *cond, NixExpr *then, NixExpr *else_);
    extern "C" NixExpr *mk_assert(NixExpr *cond, NixExpr *body);
    extern "C" NixExpr *mk_op_not(NixExpr *expr);
    extern "C" NixExpr *mk_op_eq(NixExpr *lhs, NixExpr *rhs);
    extern "C" NixExpr *mk_op_neq(NixExpr *lhs, NixExpr *rhs);
    extern "C" NixExpr *mk_op_and(NixExpr *lhs, NixExpr *rhs);
    extern "C" NixExpr *mk_op_or(NixExpr *lhs, NixExpr *rhs);
    extern "C" NixExpr *mk_op_impl(NixExpr *lhs, NixExpr *rhs);
    extern "C" NixExpr *mk_op_update(NixExpr *lhs, NixExpr *rhs);
    extern "C" NixExpr *mk_op_concat_lists(NixExpr *lhs, NixExpr *rhs);
    extern "C" NixExpr *mk_op_concat_strings(bool force_strings, NixExpr **exprs, size_t exprs_len);

    extern "C" AttrName *mk_attr_name_symbol(const char *symbol);
    extern "C" AttrName *mk_attr_name_expr(NixExpr *expr);
    extern "C" AttrDef *mk_attr_def(const char *name, bool inherited, NixExpr *expr);
    extern "C" DynamicAttrDef *mk_dynamic_attr_def(NixExpr *name_expr, NixExpr *value_expr);
    extern "C" Formal *mk_formal(const char *name, NixExpr *def);
    extern "C" Formals *mk_formals(bool ellipsis, Formal **entries, size_t entries_len);

    struct Parser;

    struct Parser *init_parser();
    void destroy_parser(struct Parser *parser);
    NixExpr *parse_nix_expr(struct Parser *parser, const char *nix_expr);
}

#endif