# Velra bootstrap

Velra uses a staged, reproducible self-hosting model.

## Production boundary

The released `velra` executable embeds `compiler/bootstrap.velc`, a compiled copy of the compiler implemented in `compiler/main.vel`.

User-facing source operations (`run`, `check`, `compile`, and the REPL) execute that embedded Velra compiler. The Rust lexer/parser/encoder are not used to compile user source in the production CLI.

Rust remains below the language boundary as the portable runtime/artifact loader and trusted stage-0 bootstrap seed. That is analogous to a self-hosted compiler targeting a VM whose implementation is written in another systems language: the language compiler is self-hosted even though the execution engine is not.

## Stage 0 — trusted Rust seed

The Rust implementation owns the minimum facilities required to bootstrap and execute Velra:

1. a compatibility lexer/parser and deterministic artifact encoder,
2. the `VELRA-AST-1` artifact decoder,
3. execution of Velra semantics,
4. file/list/string primitives required by compiler code,
5. the native CLI/runtime host and bootstrap verification.

Stage 0 is retained so the project can reproduce the compiler from auditable source without trusting a previously compiled compiler binary.

## Stage 1 — compiler written in Velra

`compiler/main.vel` contains the production source compiler: lexer, recursive-descent parser, precedence parser, and deterministic artifact emitter.

Rust stage 0 compiles this source into `compiler/bootstrap.velc`. The checked-in artifact is embedded directly into release binaries.

## Stage 2 — self reproduction

The stage-1 compiler executes through the runtime, reads Velra source, and can compile `compiler/main.vel` itself. The result must be byte-for-byte identical to the stage-1 artifact.

The bootstrap contract is therefore:

1. `stage0(compiler/main.vel) -> stage1.velc`
2. load and execute `stage1.velc` without invoking the Rust source parser
3. `stage1(compiler/main.vel) -> stage2.velc`
4. require `stage1.velc == stage2.velc`
5. require `compiler/bootstrap.velc == stage1.velc`
6. run the complete language and runtime test suite

Any change to `compiler/main.vel` that does not regenerate the exact embedded bootstrap artifact fails the test suite.

## Distribution

Release binaries contain everything needed to compile and run Velra source:

```text
velra executable
├── native CLI/runtime host
├── VELRA-AST-1 loader
└── embedded compiler/bootstrap.velc
    └── compiler implemented in Velra
```

End users do not need Rust, Cargo, a C compiler, LLVM, or another language toolchain.

## Trust model

There are two supported ways to trust a release:

- **Normal installation:** trust the signed/tagged release process and the published SHA-256 checksum.
- **Bootstrap verification:** build stage 0 from Rust source, regenerate `compiler/bootstrap.velc`, and run the stage1→stage2 byte-equality tests.

The Rust frontend is intentionally kept as a compatibility/bootstrap implementation rather than a competing production compiler. New language compilation behavior should be implemented in `compiler/main.vel` first and reflected in stage 0 only where required to keep the bootstrap seed compatible.

## Future native code generation

The current compiled format is deterministic `VELRA-AST-1` executed by the portable runtime. A future bytecode or native-code backend can replace that backend without changing the self-host trust chain: the compiler remains written in Velra, and stage1→stage2 reproducibility remains the acceptance criterion.
