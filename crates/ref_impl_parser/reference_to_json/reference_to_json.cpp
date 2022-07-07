#include <memory>
#include <iostream>
#include <nix/config.h>
#include <nix/eval.hh>
#include <nix/store-api.hh>

#include "reference_to_json.h"

class NotImplemented : public std::logic_error
{
public:
    NotImplemented() : std::logic_error("Function not yet implemented"){};
};

NixExpr *normalize_nix_expr(nix::Expr *expr, const nix::SymbolTable &symbols);

std::vector<AttrDef*> normalize_attr_defs(nix::ExprAttrs::AttrDefs attrDefs, const nix::SymbolTable &symbols)
{
    auto res = std::vector<AttrDef*>{};
    for (const auto &[key, value] : attrDefs)
    {
        res.push_back(mk_attr_def(
            ((std::string)symbols[key]).c_str(),
            value.inherited,
            normalize_nix_expr(value.e, symbols)));
    }
    return res;
}

std::vector<DynamicAttrDef*> normalize_dynamic_attr_defs(nix::ExprAttrs::DynamicAttrDefs attrDefs, const nix::SymbolTable &symbols)
{
    auto res = std::vector<DynamicAttrDef*>{};
    for (const auto &attr : attrDefs)
    {
        res.push_back(mk_dynamic_attr_def(
            normalize_nix_expr(attr.nameExpr, symbols),
            normalize_nix_expr(attr.valueExpr, symbols)));
    }
    return res;
}

Formals *normalize_formals(nix::Formals *formals, const nix::SymbolTable &symbols)
{
    if (formals == nullptr)
    {
        return nullptr;
    }

    auto entries = std::vector<Formal*>{};
    for (const auto formal : formals->formals)
    {
        entries.push_back(mk_formal(
            ((std::string)symbols[formal.name]).c_str(),
            normalize_nix_expr(formal.def, symbols)));
    }

    return mk_formals(formals->ellipsis, entries.data(), entries.size());
}

std::vector<NixExpr *> normalize_nix_exprs(std::vector<nix::Expr *> exprs, const nix::SymbolTable &symbols)
{
    auto res = std::vector<NixExpr *>{};
    for (const auto expr : exprs)
    {
        res.push_back(normalize_nix_expr(expr, symbols));
    }
    return res;
}

std::vector<NixExpr *> normalize_string_concat_exprs(std::vector<std::pair<nix::PosIdx, nix::Expr *>> *exprs, const nix::SymbolTable &symbols)
{
    auto res = std::vector<nix::Expr *>();
    for (const auto &[pos, e] : *exprs)
    {
        res.push_back(e);
    }

    return normalize_nix_exprs(res, symbols);
}

std::vector<AttrName*> normalize_attr_path(nix::AttrPath attrPath, const nix::SymbolTable &symbols)
{
    auto res = std::vector<AttrName*>{};
    for (const auto attr : attrPath)
    {
        if (attr.symbol)
        {
            res.push_back(mk_attr_name_symbol(((std::string)symbols[attr.symbol]).c_str()));
        }
        else
        {
            res.push_back(mk_attr_name_expr(normalize_nix_expr(attr.expr, symbols)));
        }
    }
    return res;
}

NixExpr *normalize_nix_expr(nix::Expr *expr, const nix::SymbolTable &symbols)
{
    if (expr == nullptr)
    {
        return nullptr;
    }
    else if (auto exprInt = dynamic_cast<nix::ExprInt *>(expr))
    {
        return mk_int(exprInt->n);
    }
    else if (auto exprFloat = dynamic_cast<nix::ExprFloat *>(expr))
    {
        return mk_float(exprFloat->nf);
    }
    else if (auto exprString = dynamic_cast<nix::ExprString *>(expr))
    {
        return mk_string(exprString->s.c_str());
    }
    else if (auto exprPath = dynamic_cast<nix::ExprPath *>(expr))
    {
        return mk_path(exprPath->s.c_str());
    }
    else if (auto exprVar = dynamic_cast<nix::ExprVar *>(expr))
    {
        return mk_var(((std::string)symbols[exprVar->name]).c_str());
    }
    else if (auto exprSelect = dynamic_cast<nix::ExprSelect *>(expr))
    {
        auto attrPath = normalize_attr_path(exprSelect->attrPath, symbols);

        return mk_select(
            normalize_nix_expr(exprSelect->e, symbols),
            normalize_nix_expr(exprSelect->def, symbols),
            attrPath.data(),
            attrPath.size());
    }
    else if (auto exprOpHasAttr = dynamic_cast<nix::ExprOpHasAttr *>(expr))
    {
        auto attrPath = normalize_attr_path(exprOpHasAttr->attrPath, symbols);

        return mk_op_has_attr(
            normalize_nix_expr(exprOpHasAttr->e, symbols),
            attrPath.data(),
            attrPath.size());
    }
    else if (auto exprAttrs = dynamic_cast<nix::ExprAttrs *>(expr))
    {
        auto attr_defs = normalize_attr_defs(exprAttrs->attrs, symbols);
        auto dynamic_attr_defs = normalize_dynamic_attr_defs(exprAttrs->dynamicAttrs, symbols);

        return mk_attrs(
            exprAttrs->recursive,
            attr_defs.data(),
            attr_defs.size(),
            dynamic_attr_defs.data(),
            dynamic_attr_defs.size());
    }
    else if (auto exprList = dynamic_cast<nix::ExprList *>(expr))
    {
        auto elems = normalize_nix_exprs(exprList->elems, symbols);
        return mk_list(elems.data(), elems.size());
    }
    else if (auto exprLambda = dynamic_cast<nix::ExprLambda *>(expr))
    {
        const char *arg = nullptr;
        if (exprLambda->arg)
        {
            arg = ((std::string)symbols[exprLambda->arg]).c_str();
        }

        return mk_lambda(
            &arg,
            normalize_formals(exprLambda->formals, symbols),
            normalize_nix_expr(exprLambda->body, symbols));
    }
    else if (auto exprCall = dynamic_cast<nix::ExprCall *>(expr))
    {
        auto args = normalize_nix_exprs(exprCall->args, symbols);
        return mk_call(
            normalize_nix_expr(exprCall->fun, symbols),
            args.data(),
            args.size());
    }
    else if (auto exprLet = dynamic_cast<nix::ExprLet *>(expr))
    {
        return mk_let(
            normalize_nix_expr(exprLet->attrs, symbols),
            normalize_nix_expr(exprLet->body, symbols));
    }
    else if (auto exprWith = dynamic_cast<nix::ExprWith *>(expr))
    {
        return mk_with(
            normalize_nix_expr(exprWith->attrs, symbols),
            normalize_nix_expr(exprWith->body, symbols));
    }
    else if (auto exprIf = dynamic_cast<nix::ExprIf *>(expr))
    {
        return mk_if(
            normalize_nix_expr(exprIf->cond, symbols),
            normalize_nix_expr(exprIf->then, symbols),
            normalize_nix_expr(exprIf->else_, symbols));
    }
    else if (auto exprAssert = dynamic_cast<nix::ExprAssert *>(expr))
    {
        return mk_assert(
            normalize_nix_expr(exprAssert->cond, symbols),
            normalize_nix_expr(exprAssert->body, symbols));
    }
    else if (auto exprOpNot = dynamic_cast<nix::ExprOpNot *>(expr))
    {
        return mk_op_not(normalize_nix_expr(exprOpNot->e, symbols));
    }
    else if (auto exprOpEq = dynamic_cast<nix::ExprOpEq *>(expr))
    {
        return mk_op_eq(normalize_nix_expr(exprOpEq->e1, symbols), normalize_nix_expr(exprOpEq->e2, symbols));
    }
    else if (auto exprOpNEq = dynamic_cast<nix::ExprOpNEq *>(expr))
    {
        return mk_op_neq(normalize_nix_expr(exprOpNEq->e1, symbols), normalize_nix_expr(exprOpNEq->e2, symbols));
    }
    else if (auto exprOpAnd = dynamic_cast<nix::ExprOpAnd *>(expr))
    {
        return mk_op_and(normalize_nix_expr(exprOpAnd->e1, symbols), normalize_nix_expr(exprOpAnd->e2, symbols));
    }
    else if (auto exprOpOr = dynamic_cast<nix::ExprOpOr *>(expr))
    {
        return mk_op_or(normalize_nix_expr(exprOpOr->e1, symbols), normalize_nix_expr(exprOpOr->e2, symbols));
    }
    else if (auto exprOpImpl = dynamic_cast<nix::ExprOpImpl *>(expr))
    {
        return mk_op_impl(normalize_nix_expr(exprOpImpl->e1, symbols), normalize_nix_expr(exprOpImpl->e2, symbols));
    }
    else if (auto exprOpUpdate = dynamic_cast<nix::ExprOpUpdate *>(expr))
    {
        return mk_op_update(normalize_nix_expr(exprOpUpdate->e1, symbols), normalize_nix_expr(exprOpUpdate->e2, symbols));
    }
    else if (auto exprOpConcatLists = dynamic_cast<nix::ExprOpConcatLists *>(expr))
    {
        return mk_op_concat_lists(normalize_nix_expr(exprOpConcatLists->e1, symbols), normalize_nix_expr(exprOpConcatLists->e2, symbols));
    }
    else if (auto exprConcatStrings = dynamic_cast<nix::ExprConcatStrings *>(expr))
    {
        auto exprs = normalize_string_concat_exprs(exprConcatStrings->es, symbols);

        return mk_op_concat_strings(
            exprConcatStrings->forceString,
            exprs.data(),
            exprs.size());
    }
    else if (auto exprPos = dynamic_cast<nix::ExprPos *>(expr))
    {
        throw NotImplemented();
    }

    throw NotImplemented();
}

struct Parser
{
    nix::EvalState *state;

    ~Parser()
    {
        delete state;
    }
};

extern "C" Parser *init_parser()
{
    nix::initGC();

    auto searchPath = nix::Strings{};
    auto store = nix::openStore();
    auto state = new nix::EvalState(searchPath, store);

    return new Parser{state};
}

extern "C" void destroy_parser(Parser *parser)
{
    delete parser;
}

extern "C" NixExpr *parse_nix_expr(Parser *parser, const char *nix_expr)
{
    auto expr = parser->state->parseExprFromString(nix_expr, nix::absPath("."));
    return normalize_nix_expr(expr, parser->state->symbols);
}
