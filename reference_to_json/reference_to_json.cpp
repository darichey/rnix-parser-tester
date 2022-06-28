#include <memory>
#include <iostream>
#include <nlohmann/json.hpp>
#include <nix/config.h>
#include <nix/eval.hh>
#include <nix/store-api.hh>

using namespace nix;

nlohmann::json nix_expr_to_json(Expr *expr, const SymbolTable &symbols);

class NotImplemented : public std::logic_error
{
public:
    NotImplemented() : std::logic_error("Function not yet implemented"){};
};

nlohmann::json attr_defs_to_json(ExprAttrs::AttrDefs attrDefs, const SymbolTable &symbols)
{
    auto res = nlohmann::json::object();
    for (const auto &[key, value] : attrDefs)
    {
        res[symbols[key]] = {value.inherited, nix_expr_to_json(value.e, symbols)};
    }
    return res;
}

nlohmann::json dynamic_attr_defs_to_json(ExprAttrs::DynamicAttrDefs attrDefs, const SymbolTable &symbols)
{
    auto res = nlohmann::json::array();
    for (const auto &attr : attrDefs)
    {
        res.push_back({nix_expr_to_json(attr.nameExpr, symbols), nix_expr_to_json(attr.valueExpr, symbols)});
    }
    return res;
}

nlohmann::json formals_to_json(Formals *formals, const SymbolTable &symbols)
{
    if (formals == nullptr)
    {
        return nullptr;
    }

    auto res = nlohmann::json::array();
    for (const auto formal : formals->formals)
    {
        res.push_back({
            symbols[formal.name],
            nix_expr_to_json(formal.def, symbols),
        });
    }

    return {formals->ellipsis, res};
}

nlohmann::json nix_exprs_to_json(std::vector<Expr *> exprs, const SymbolTable &symbols)
{
    auto res = nlohmann::json::array();
    for (const auto expr : exprs)
    {
        res.push_back(nix_expr_to_json(expr, symbols));
    }
    return res;
}

nlohmann::json string_concat_exprs_to_json(std::vector<std::pair<PosIdx, Expr *>> *exprs, const SymbolTable &symbols)
{
    auto res = std::vector<Expr *>();
    for (const auto &[pos, e] : *exprs)
    {
        res.push_back(e);
    }

    return nix_exprs_to_json(res, symbols);
}

nlohmann::json nix_expr_to_json(Expr *expr, const SymbolTable &symbols)
{
    if (expr == nullptr)
    {
        return nullptr;
    }
    else if (auto exprInt = dynamic_cast<ExprInt *>(expr))
    {
        return {"Int", exprInt->n};
    }
    else if (auto exprFloat = dynamic_cast<ExprFloat *>(expr))
    {
        return {"Float", exprFloat->nf};
    }
    else if (auto exprString = dynamic_cast<ExprString *>(expr))
    {
        return {"String", exprString->s};
    }
    else if (auto exprPath = dynamic_cast<ExprPath *>(expr))
    {
        return {"Path", exprPath->s};
    }
    else if (auto exprVar = dynamic_cast<ExprVar *>(expr))
    {
        return {"Var", symbols[exprVar->name]};
    }
    else if (auto exprSelect = dynamic_cast<ExprSelect *>(expr))
    {
        return {
            "Select",
            nix_expr_to_json(exprSelect->e, symbols),
            nix_expr_to_json(exprSelect->def, symbols),
            showAttrPath(symbols, exprSelect->attrPath),
        };
    }
    else if (auto exprOpHasAttr = dynamic_cast<ExprOpHasAttr *>(expr))
    {
        return {
            "OpHasAttr",
            nix_expr_to_json(exprOpHasAttr->e, symbols),
            showAttrPath(symbols, exprOpHasAttr->attrPath),
        };
    }
    else if (auto exprAttrs = dynamic_cast<ExprAttrs *>(expr))
    {
        return {
            "Attrs",
            exprAttrs->recursive,
            attr_defs_to_json(exprAttrs->attrs, symbols),
            dynamic_attr_defs_to_json(exprAttrs->dynamicAttrs, symbols),
        };
    }
    else if (auto exprList = dynamic_cast<ExprList *>(expr))
    {
        return {
            "List",
            nix_exprs_to_json(exprList->elems, symbols),
        };
    }
    else if (auto exprLambda = dynamic_cast<ExprLambda *>(expr))
    {
        return {
            "Lambda",
            exprLambda->name ? (std::string)symbols[exprLambda->name] : "",
            exprLambda->arg ? (std::string)symbols[exprLambda->arg] : "",
            formals_to_json(exprLambda->formals, symbols),
            nix_expr_to_json(exprLambda->body, symbols),
        };
    }
    else if (auto exprCall = dynamic_cast<ExprCall *>(expr))
    {
        return {
            "Call",
            nix_expr_to_json(exprCall->fun, symbols),
            nix_exprs_to_json(exprCall->args, symbols),
        };
    }
    else if (auto exprLet = dynamic_cast<ExprLet *>(expr))
    {
        return {
            "Let",
            nix_expr_to_json(exprLet->attrs, symbols),
            nix_expr_to_json(exprLet->body, symbols),
        };
    }
    else if (auto exprWith = dynamic_cast<ExprWith *>(expr))
    {
        return {
            "With",
            nix_expr_to_json(exprWith->attrs, symbols),
            nix_expr_to_json(exprWith->body, symbols),
        };
    }
    else if (auto exprIf = dynamic_cast<ExprIf *>(expr))
    {
        return {
            "If",
            nix_expr_to_json(exprIf->cond, symbols),
            nix_expr_to_json(exprIf->then, symbols),
            nix_expr_to_json(exprIf->else_, symbols),
        };
    }
    else if (auto exprAssert = dynamic_cast<ExprAssert *>(expr))
    {
        return {
            "Assert",
            nix_expr_to_json(exprAssert->cond, symbols),
            nix_expr_to_json(exprAssert->body, symbols),
        };
    }
    else if (auto exprOpNot = dynamic_cast<ExprOpNot *>(expr))
    {
        return {"OpNot", nix_expr_to_json(exprOpNot->e, symbols)};
    }
    else if (auto exprOpEq = dynamic_cast<ExprOpEq *>(expr))
    {
        return {"OpEq", nix_expr_to_json(exprOpEq->e1, symbols), nix_expr_to_json(exprOpEq->e2, symbols)};
    }
    else if (auto exprOpNEq = dynamic_cast<ExprOpNEq *>(expr))
    {
        return {"OpNEq", nix_expr_to_json(exprOpNEq->e1, symbols), nix_expr_to_json(exprOpNEq->e2, symbols)};
    }
    else if (auto exprOpAnd = dynamic_cast<ExprOpAnd *>(expr))
    {
        return {"OpAnd", nix_expr_to_json(exprOpAnd->e1, symbols), nix_expr_to_json(exprOpAnd->e2, symbols)};
    }
    else if (auto exprOpOr = dynamic_cast<ExprOpOr *>(expr))
    {
        return {"OpOr", nix_expr_to_json(exprOpOr->e1, symbols), nix_expr_to_json(exprOpOr->e2, symbols)};
    }
    else if (auto exprOpImpl = dynamic_cast<ExprOpImpl *>(expr))
    {
        return {"OpImpl", nix_expr_to_json(exprOpImpl->e1, symbols), nix_expr_to_json(exprOpImpl->e2, symbols)};
    }
    else if (auto exprOpUpdate = dynamic_cast<ExprOpUpdate *>(expr))
    {
        return {"OpUpdate", nix_expr_to_json(exprOpUpdate->e1, symbols), nix_expr_to_json(exprOpUpdate->e2, symbols)};
    }
    else if (auto exprOpConcatLists = dynamic_cast<ExprOpConcatLists *>(expr))
    {
        return {"OpConcatLists", nix_expr_to_json(exprOpConcatLists->e1, symbols), nix_expr_to_json(exprOpConcatLists->e2, symbols)};
    }
    else if (auto exprConcatStrings = dynamic_cast<ExprConcatStrings *>(expr))
    {
        return {
            "ConcatStrings",
            exprConcatStrings->forceString,
            string_concat_exprs_to_json(exprConcatStrings->es, symbols),
        };
    }
    else if (auto exprPos = dynamic_cast<ExprPos *>(expr))
    {
        throw NotImplemented();
    }

    throw NotImplemented();
}

extern "C" const char *nix_expr_to_json_str(const char *nix_expr)
{
    initGC();

    Strings searchPath = {};
    auto store = openStore();
    auto state = std::make_unique<EvalState>(searchPath, store);
    auto expr = state->parseExprFromString(nix_expr, absPath("."));

    auto s = nix_expr_to_json(expr, state->symbols).dump();

    char* foo = (char*) malloc(sizeof(char) * 1000);
    std::size_t length = s.copy(foo, s.length());
    foo[length] = '\0';

    return foo;
}
