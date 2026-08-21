# Velra bootstrap

Velra uses a conventional staged self-hosting plan.

## Stage 0 — Rust bootstrap

The Rust implementation owns only the facilities required to get the language off the ground:

1. lexical analysis
2. parsing into the stable Velra AST
3. execution of the language semantics
4. minimal file/list/string primitives needed by compiler code
5. CLI diagnostics and bootstrap verification

This stage is implemented in `src/`.

## Stage 1 — compiler written in Velra

The self-hosted compiler should live under `compiler/` and consume the same syntax accepted by stage 0. Its first backend should be deliberately small: a portable Velra bytecode format plus a Rust VM. That avoids binding the language to LLVM, Cranelift, or a platform toolchain during bootstrap.

A self-hosting milestone is complete only when all of the following hold:

1. stage 0 compiles the stage-1 compiler,
2. the resulting stage-1 compiler compiles its own source,
3. stage-1 and stage-2 compiler outputs are byte-for-byte identical or semantically equivalent under a documented reproducibility rule,
4. the language test suite passes under the stage-1 output.

Until those checks pass, the project should say **bootstrap-capable**, not fully self-hosted.

## Keep the boundary small

Do not copy the Rust parser into several implementations. The stage-1 compiler should become the single source of truth for language compilation once it can reproduce itself. The Rust stage 0 then remains only as a trusted bootstrap seed and compatibility oracle.

The next bootstrap unit should introduce:

- a compact bytecode instruction set,
- a VM in Rust,
- AST-to-bytecode lowering,
- compiler-facing string/list utilities,
- then the equivalent compiler frontend in Velra.

Do not add JIT or native-code dependencies before the self-host cycle is proven.
