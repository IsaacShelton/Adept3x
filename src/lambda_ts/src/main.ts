
type Term = {
    kind: "lambda",
    body: Term,
} | {
    kind: "pi",
    base: Term,
    body: Term,
} | {
    kind: "apply",
    f: Term,
    arg: Term,
} | {
    kind: "annotate",
    term: Term,
    has_type: Term,
} | {
    kind: "variable",
    index: number,
} | {
    kind: "type"
} | {
    kind: "type1"
} | {
    kind: "nat",
} | {
    kind: "void"
};

function levels(term: Term): number {
    switch (term.kind) {
        case "lambda":
        case "pi":
            return 1 + levels(term.body);
        default:
            return 0;
    }
}

function variable(index: number): Term {
    return { kind: "variable", index };
}

function main() {
    const true_value: Term = {
        kind: "lambda",
        body: {
            kind: "lambda",
            body: variable(1),
        }
    };

    const false_value: Term = {
        kind: "lambda",
        body: {
            kind: "lambda",
            body: variable(0),
        }
    };

    const type_chooser: Term = {
        kind: "pi",
        base: { kind: "type" },
        body: {
            kind: "apply",
            f: {
                kind: "apply",
                f: variable(0),
                arg: { kind: "nat" },
            },
            arg: { kind: "void" },
        }
    };

    const testing: Term = {
        kind: "apply",
        f: type_chooser,
        arg: true_value,
    };

    log("FINAL", reduce(testing));
}

function reduce(term: Term): Term {
    switch (term.kind) {
        case "lambda":
            return term;
        case "pi":
            return term;
        case "apply":
            return reduceApply(term);
        case "type":
            return term;
        case "type1":
            return term;
        case "annotate":
            return reduce(term.term);
        case "void":
            return term;
        case "nat":
            return term;
        case "variable":
            return term;
        default:
            term satisfies never;
    }

    throw new Error("Unreachable");
}

function reduceApply(apply: Term & { kind: "apply" }): Term {
    log("reducing", apply);
    const f = reduce(apply.f);
    log("reduced", apply);

    switch (f.kind) {
        case "lambda":
            log("body", f.body);
            log("arg", apply.arg);
            console.log("levels = " + levels(f.body))
            const result = substitute(f.body, levels(f.body), apply.arg);
            if (result.kind == "apply") {
                log("result", result);
                return reduceApply(result);
            }
            log("done", result);
            return result;
        case "pi": {
            log("body", f.body);
            log("arg", apply.arg);
            const _base = substitute(f.base, levels(f.base), apply.arg);
            const result = substitute(f.body, levels(f.body), apply.arg);
            if (result.kind == "apply") {
                log("result", result);
                return reduceApply(result);
            }
            log("done", result);
            return result;
        }
        default:
            throw new Error("Cannot apply function that is not a lambda or pi - " + JSON.stringify(apply));
    }
}

function log(title: string, term: Term) {
    console.log("[" + title + "]");
    console.log(stringify(term));
}

function stringify(term: Term): string {
    switch (term.kind) {
        case "void":
            return "Void";
        case "nat":
            return "Nat";
        case "variable":
            return "" + term.index;
        case "apply":
            return `(${stringify(term.f)} ${stringify(term.arg)})`;
        case "lambda":
            return `λ ${stringify(term.body)}`;
        case "pi":
            return `Π ${stringify(term.body)}`;
        case "type":
            return "Type";
        case "type1":
            return "Type[1]";
        case "annotate":
            return `[${stringify(term.term)} ${stringify(term.has_type)}]`;
        default:
            term satisfies never;
    }
    throw new Error("Unreachable");
}

function substitute(body: Term, index: number, value: Term): Term {
    if (index < 0) throw new Error("Level should be >= 0");

    switch (body.kind) {
        case "lambda": {
            return {
                kind: "lambda",
                body: substitute(body.body, index, value),
            };
        }
        case "pi": {
            return {
                kind: "pi",
                base: substitute(body.base, index, value),
                body: substitute(body.body, index, value),
            };
        }
        case "apply":
            return {
                kind: "apply",
                f: substitute(body.f, index, value),
                arg: substitute(body.arg, index, value),
            };
        case "variable":
            if (body.index == index) {
                return value;
            } else {
                return body;
            }
        case "annotate":
            return substitute(body.term, index, value);
        case "type":
        case "type1":
        case "nat":
        case "void":
            return body;
        default:
            body satisfies never;
    }

    throw new Error("unreachable");
}

main();
