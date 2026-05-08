# Compiler Pipeline

Proto compiles source files to native executables through LLVM.

```text
Source -> Lexer -> Parser/AST -> HIR -> Typeck -> THIR -> Borrowck -> MIR -> LLVM IR -> Object -> Link -> Native exe
```

## Decisions

- Borrow v1: lexical RAII.
- IR stack: AST -> HIR -> THIR -> MIR -> LLVM.
- LLVM binding: `inkwell`.
- Runtime: tiny Proto runtime for panic, alloc, print, and drop glue.
- Memory v1: value/stack first; owning heap type later.
- Generics: monomorphize before LLVM.

## Stages

### Source

Input: `.pr` source files.

Output: file text plus source ids/spans.

Owns file loading, source maps, and diagnostics anchoring.

### Lexer

Input: source text.

Output: token stream with spans.

Owns lexical validation only. It must not resolve names, types, or syntax meaning.

### Parser / AST

Input: token stream.

Output: AST arenas.

AST is syntax only. It preserves parsed structure, spans, identifiers, literals, and user-written type syntax. It should not desugar, resolve names, infer types, or encode ownership decisions.

Near-term AST work: match `grammar.pest`, especially statements, assignments, `if`, `for`, methods, and blocks.

### HIR

Input: AST plus module context.

Output: HIR module graph.

HIR owns:

- module/import resolution
- item namespaces
- canonical paths
- method/function separation
- desugaring syntax into simpler forms
- stable ids for items, locals, params, fields, variants

HIR still has unresolved/inferred types where needed. It does not own final ownership facts.

### Typeck

Input: HIR.

Output: type tables, trait/capability facts, generic obligations, typed item signatures.

Typeck owns:

- name-to-definition binding
- type inference/checking
- generic argument validation
- `Copy`/drop capability classification
- function and method call resolution
- field and enum variant validation

Typeck must finish before borrow checking.

### THIR

Input: HIR plus typeck results.

Output: typed HIR.

THIR owns:

- typed expressions and statements
- explicit places for locals, fields, derefs later
- move/copy classification per use
- explicit temporary values
- ownership-relevant facts for borrowck

THIR is the main borrow checker input.

### Borrowck

Input: THIR.

Output: checked THIR plus move/borrow/drop facts.

Borrow v1 is lexical RAII:

- moving a non-`Copy` place invalidates it
- `Copy` places may be reused after copy
- many shared borrows may coexist
- mutable borrow is exclusive
- mutable borrow conflicts with all other borrows
- borrows end at lexical scope end
- owner drops happen at lexical scope end
- moved values are not dropped

Borrowck errors must report the original spans for move, borrow, use, and scope owner.

### MIR

Input: borrow-checked THIR.

Output: MIR bodies.

MIR owns:

- control-flow graph
- locals and temporaries
- assignments
- calls
- branches
- returns
- explicit drops
- lowered loops and conditionals

MIR should be simple enough for LLVM lowering and future optimizations. It should not need to redo typeck or borrowck.

### Monomorphization

Input: checked generic HIR/THIR/MIR plus discovered generic uses.

Output: concrete MIR instances.

Generics are compiled by monomorphization. Every LLVM-emitted function has concrete types.

### LLVM IR

Input: concrete MIR.

Output: LLVM module via `inkwell`.

LLVM lowering owns:

- target machine setup
- ABI layout decisions
- function declarations/definitions
- basic block emission
- value lowering
- calls into Proto runtime
- object file emission

LLVM lowering must assume typeck and borrowck already proved safety rules.

### Runtime / Link

Input: object files plus runtime objects/libs.

Output: native executable.

The tiny runtime owns:

- panic entrypoint
- allocation hooks
- print/debug hooks
- drop glue helpers if needed

The compiler links generated objects with runtime objects/libs to produce the final executable.

## Milestones

1. AST parity with grammar.
2. HIR lowering.
3. Typeck.
4. THIR ownership model.
5. Borrowck.
6. MIR.
7. LLVM codegen.
8. Runtime/linking.

## Future Acceptance Tests

- simple `main` compiles to native executable
- move-after-move rejected
- shared borrows accepted
- mutable/shared conflict rejected
- struct return codegen works
- generic function monomorphized

## Assumptions

- `proto` CLI will eventually compile a source file to a native executable.
- Existing parser/AST may evolve freely; no compatibility promise yet.
- Heap ownership enters through an owning standard type after value/stack semantics work.
