#include <memory>
#include <iostream>
#include <nlohmann/json.hpp>
#include <nix/config.h>
#include <nix/eval.hh>
#include <nix/store-api.hh>

#include "reference_to_json.h"

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
        res[symbols[key]] = {
            {"inherited", value.inherited},
            {"e", nix_expr_to_json(value.e, symbols)},
        };
    }
    return res;
}

nlohmann::json dynamic_attr_defs_to_json(ExprAttrs::DynamicAttrDefs attrDefs, const SymbolTable &symbols)
{
    auto res = nlohmann::json::array();
    for (const auto &attr : attrDefs)
    {
        res.push_back({
            {"name_expr", nix_expr_to_json(attr.nameExpr, symbols)},
            {"value_expr", nix_expr_to_json(attr.valueExpr, symbols)},
        });
    }
    return res;
}

nlohmann::json formals_to_json(Formals *formals, const SymbolTable &symbols)
{
    if (formals == nullptr)
    {
        return nullptr;
    }

    auto entries = nlohmann::json::array();
    for (const auto formal : formals->formals)
    {
        entries.push_back({
            {"name", symbols[formal.name]},
            {"default", nix_expr_to_json(formal.def, symbols)},
        });
    }

    return {
        {"ellipsis", formals->ellipsis},
        {"entries", entries},
    };
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
        return {
            {"type", "Int"},
            {"value", exprInt->n},
        };
    }
    else if (auto exprFloat = dynamic_cast<ExprFloat *>(expr))
    {
        return {
            {"type", "Float"},
            {"value", exprFloat->nf},
        };
    }
    else if (auto exprString = dynamic_cast<ExprString *>(expr))
    {
        return {
            {"type", "String"},
            {"value", exprString->s},
        };
    }
    else if (auto exprPath = dynamic_cast<ExprPath *>(expr))
    {
        return {
            {"type", "Path"},
            {"value", exprPath->s},
        };
    }
    else if (auto exprVar = dynamic_cast<ExprVar *>(expr))
    {
        return {
            {"type", "Var"},
            {"value", symbols[exprVar->name]},
        };
    }
    else if (auto exprSelect = dynamic_cast<ExprSelect *>(expr))
    {
        return {
            {"type", "Select"},
            {"subject", nix_expr_to_json(exprSelect->e, symbols)},
            {"or_default", nix_expr_to_json(exprSelect->def, symbols)},
            {"path", showAttrPath(symbols, exprSelect->attrPath)},
        };
    }
    else if (auto exprOpHasAttr = dynamic_cast<ExprOpHasAttr *>(expr))
    {
        return {
            {"type", "OpHasAttr"},
            {"subject", nix_expr_to_json(exprOpHasAttr->e, symbols)},
            {"path", showAttrPath(symbols, exprOpHasAttr->attrPath)},
        };
    }
    else if (auto exprAttrs = dynamic_cast<ExprAttrs *>(expr))
    {
        return {
            {"type", "Attrs"},
            {"rec", exprAttrs->recursive},
            {"attrs", attr_defs_to_json(exprAttrs->attrs, symbols)},
            {"dynamic_attrs", dynamic_attr_defs_to_json(exprAttrs->dynamicAttrs, symbols)},
        };
    }
    else if (auto exprList = dynamic_cast<ExprList *>(expr))
    {
        return {
            {"type", "List"},
            {"elems", nix_exprs_to_json(exprList->elems, symbols)},
        };
    }
    else if (auto exprLambda = dynamic_cast<ExprLambda *>(expr))
    {
        nlohmann::json arg(nullptr);
        if (exprLambda->arg) {
            arg = (std::string)symbols[exprLambda->arg];
        }

        return {
            {"type", "Lambda"},
            // TODO: decide if this is important. It seems unlikely that we can easily get this on the rnix side
            // {"name", exprLambda->name ? (std::string)symbols[exprLambda->name] : ""},
            {"arg", arg},
            {"formals", formals_to_json(exprLambda->formals, symbols)},
            {"body", nix_expr_to_json(exprLambda->body, symbols)},
        };
    }
    else if (auto exprCall = dynamic_cast<ExprCall *>(expr))
    {
        return {
            {"type", "Call"},
            {"fun", nix_expr_to_json(exprCall->fun, symbols)},
            {"args", nix_exprs_to_json(exprCall->args, symbols)},
        };
    }
    else if (auto exprLet = dynamic_cast<ExprLet *>(expr))
    {
        return {
            {"type", "Let"},
            {"attrs", nix_expr_to_json(exprLet->attrs, symbols)},
            {"body", nix_expr_to_json(exprLet->body, symbols)},
        };
    }
    else if (auto exprWith = dynamic_cast<ExprWith *>(expr))
    {
        return {
            {"type", "With"},
            {"attrs", nix_expr_to_json(exprWith->attrs, symbols)},
            {"body", nix_expr_to_json(exprWith->body, symbols)},
        };
    }
    else if (auto exprIf = dynamic_cast<ExprIf *>(expr))
    {
        return {
            {"type", "If"},
            {"cond", nix_expr_to_json(exprIf->cond, symbols)},
            {"then", nix_expr_to_json(exprIf->then, symbols)},
            {"else", nix_expr_to_json(exprIf->else_, symbols)},
        };
    }
    else if (auto exprAssert = dynamic_cast<ExprAssert *>(expr))
    {
        return {
            {"type", "Assert"},
            {"cond", nix_expr_to_json(exprAssert->cond, symbols)},
            {"body", nix_expr_to_json(exprAssert->body, symbols)},
        };
    }
    else if (auto exprOpNot = dynamic_cast<ExprOpNot *>(expr))
    {
        return {
            {"type", "OpNot"},
            {"e", nix_expr_to_json(exprOpNot->e, symbols)},
        };
    }
    else if (auto exprOpEq = dynamic_cast<ExprOpEq *>(expr))
    {
        return {
            {"type", "OpEq"},
            {"e1", nix_expr_to_json(exprOpEq->e1, symbols)},
            {"e2", nix_expr_to_json(exprOpEq->e2, symbols)},
        };
    }
    else if (auto exprOpNEq = dynamic_cast<ExprOpNEq *>(expr))
    {
        return {
            {"type", "OpNEq"},
            {"e1", nix_expr_to_json(exprOpNEq->e1, symbols)},
            {"e2", nix_expr_to_json(exprOpNEq->e2, symbols)},
        };
    }
    else if (auto exprOpAnd = dynamic_cast<ExprOpAnd *>(expr))
    {
        return {
            {"type", "OpAnd"},
            {"e1", nix_expr_to_json(exprOpAnd->e1, symbols)},
            {"e2", nix_expr_to_json(exprOpAnd->e2, symbols)},
        };
    }
    else if (auto exprOpOr = dynamic_cast<ExprOpOr *>(expr))
    {
        return {
            {"type", "OpOr"},
            {"e1", nix_expr_to_json(exprOpOr->e1, symbols)},
            {"e2", nix_expr_to_json(exprOpOr->e2, symbols)},
        };
    }
    else if (auto exprOpImpl = dynamic_cast<ExprOpImpl *>(expr))
    {
        return {
            {"type", "OpImpl"},
            {"e1", nix_expr_to_json(exprOpImpl->e1, symbols)},
            {"e2", nix_expr_to_json(exprOpImpl->e2, symbols)},
        };
    }
    else if (auto exprOpUpdate = dynamic_cast<ExprOpUpdate *>(expr))
    {
        return {
            {"type", "OpUpdate"},
            {"e1", nix_expr_to_json(exprOpUpdate->e1, symbols)},
            {"e2", nix_expr_to_json(exprOpUpdate->e2, symbols)},
        };
    }
    else if (auto exprOpConcatLists = dynamic_cast<ExprOpConcatLists *>(expr))
    {
        return {
            {"type", "OpConcatLists"},
            {"e1", nix_expr_to_json(exprOpConcatLists->e1, symbols)},
            {"e2", nix_expr_to_json(exprOpConcatLists->e2, symbols)},
        };
    }
    else if (auto exprConcatStrings = dynamic_cast<ExprConcatStrings *>(expr))
    {
        return {
            {"type", "ConcatStrings"},
            {"force_string", exprConcatStrings->forceString},
            {"es", string_concat_exprs_to_json(exprConcatStrings->es, symbols)},
        };
    }
    else if (auto exprPos = dynamic_cast<ExprPos *>(expr))
    {
        throw NotImplemented();
    }

    throw NotImplemented();
}

struct Parser
{
    EvalState *state;

    ~Parser()
    {
        delete state;
    }
};

extern "C" Parser *init_parser()
{
    initGC();

    auto searchPath = Strings{};
    auto store = openStore();
    auto state = new EvalState(searchPath, store);

    return new Parser{state};
}

extern "C" void destroy_parser(Parser *parser)
{
    delete parser;
}

extern "C" const char *nix_expr_to_json_str(Parser *parser, const char *nix_expr)
{
    auto expr = parser->state->parseExprFromString(nix_expr, absPath("."));

    auto json_str = nix_expr_to_json(expr, parser->state->symbols).dump();
    auto c_str = json_str.c_str();

    return strdup(c_str);
}
