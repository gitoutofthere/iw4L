# GSC compiler static audit — 2026-10-01

The confirmed defect is loss of token kind at parser decision points. String
contents were treated as EOF, operators, delimiters, or numeric case labels.
This rejected supported scripts and accepted malformed scripts. The patch fixes
that cause and validates assignment operators before advancing the parser.

This audit is source inspection and manual tracing only. No compiler, game,
build, test, formatter, generator, or probe was run; no dependency was installed.
The examples below are synthetic, not extracted game files. They are expected
outcomes inferred from source, not measured results.

## Revision, scope, and duplication check

The baseline is upstream `master` at
[`ff7369ffa64eaa2ad41be0fe9f0879424a98afec`](https://github.com/vladtrc/iw4L/commit/ff7369ffa64eaa2ad41be0fe9f0879424a98afec).
It was fetched before creating a separate worktree and the branch
`fix/gsc-compiler-audit-20261001`. Existing worktrees and changes were preserved.
On 2026-10-01 the upstream open PRs were #29, #33, #35, #38–#45; none changed
`script/compiler.rs` or its children. The fork's open PR #1 did not change them
either. These are snapshots, not claims about future PR state.

Scope: source tokenization, parsing, name resolution, and generation/linking of
executable IR. The project emits `Vec<(Location, Op)>`, not a serialized GSC
byte stream. Runtime instruction handlers were read only to check the existing
IR contract; VM execution, networking, FFI, and file handling were not changed.
All evidence is the project's source and documentation at the baseline; no
external implementation or newer language dialect is used to establish a defect.

## Path from source to compilation result

1. [`ScriptSources::capture`](../crates/assets/src/script_sources.rs) retains
   RawFile source bytes, strips trailing NUL terminators, and normalizes names.
   This ingestion path is context only, outside the patch.
2. [`match_apply`](../crates/session/src/match_apply.rs) wraps the retained
   sources in `SourceResolver`; `Iw4Startup` selects the roots and the session
   calls [`Program::load`](../crates/sim/src/script/program.rs).
3. [`compile`][compiler-base] normalizes roots, visits dependencies once via
   `pending`/`imports`, reads bytes, hashes them, and calls
   [`decode_source`](../crates/sim/src/script/source.rs): UTF-8 when valid,
   otherwise byte-to-character decoding. `lex` produces tokens plus an EOF token.
4. [`Parser::module`][compiler-base] parses directives, constants, parameters,
   and function bodies. The expression, assignment, statement, and call methods
   emit instructions directly. Function bodies get an implicit undefined return.
5. The linker resolves function references/calls, removes developer builtin
   statements and their arguments, remaps jumps, then replaces pending IDs with
   script/native IDs. Successful compilation returns a `Program`; syntax and
   linking errors return a located `Fault`.

The existing [runtime documentation](GSC-RUNTIME.md) establishes function-local
slots, string values, arrays/index lvalues, constants, conditional expressions,
and the linking order. [`Op` and `IR_VERSION = 3`](../crates/sim/src/script/ir.rs)
are the instruction vocabulary. None of these rules is extended by this patch.

## 1. Tokens, strings, comments, and EOF

[`lex`][lexer-base] stores decoded literal contents in `Token.text` and preserves
their kind with `Token.string`. Thus `""` and EOF both have empty text, but only
EOF has `string = false`. Likewise `"["` is one string value, not a bracket.
The existing `Parser::is` already enforced this distinction; several other
decision points bypassed it.

Manual lexer controls: `main(){return "/*";} // tail` produces a string literal
containing `/*`; the line comment reaches EOF without needing a newline.
`main(){return "\"";}` decodes a literal quote without ending the string early.
`/*` fails the block-comment lookahead guard and returns `unterminated comment`.
`main(){return "x` reaches `i == chars.len()` and returns `unterminated string`.
A final backslash inside a string goes through `chars.get(i)` and returns a
fault rather than indexing past EOF. Newlines in strings are rejected.
Each lexer loop either advances its cursor or returns an error; two-character
punctuation requires an available second character.

Identifiers accept ASCII letters/underscore initially and digits/backslashes
thereafter; qualified module paths are validated later. Integers wrap to i32
by design. A digit run containing multiple dots is consumed and subsequently
rejected by float parsing. Scientific notation and hexadecimal literals are
not implemented; their absence is not treated as a correctness defect.

## 2. Precedence and associativity

[`expression`][expressions-base] uses increasing binding powers:
`||`, `&&`, `|`, `^`, `&`, equality, comparisons, shifts, addition/subtraction,
multiplication/division/modulo. Binary RHS parsing uses `power + 1`, making
equal-power binary operators left associative. Prefix `!`, `~`, and general
negation recurse with power 12; postfix fields, indices, and calls are consumed
before binary operators. The conditional expression is parsed at power zero,
recursively in both arms.

Manual controls: `main(){return 8 - 3 - 1;}` emits the expression fragment
`C(8), C(3), Sub, C(1), Sub`; `main(){return 2 + 3 * 4;}` emits
`C(2), C(3), C(4), Mul, Add`. `main(){return !a == b;}` loads `a`, applies
`Not`, loads `b`, then compares. `a ? b : c ? d : e` nests the second
conditional in the false arm. No precedence-table change is warranted.

The defect here is the binary loop's raw text match: `main(){return 1 "+" 2;}`
emitted `C(1), C(2), Add, Return`. No branch consulted `string` for the supposed
operator. The same applied to every supported binary operator, including
short-circuit `"&&"` and `"||"`. The patch leaves the literal unconsumed;
`expect(";")` then returns a syntax fault.

## 3. Scope and name resolution

[`slot`, `load`, `module`, and `link`][compiler-base] normalize names and use
per-function slots, not block-local declarations. Slots are cleared between
functions. Repeated parameter names intentionally keep the first named slot
and reserve anonymous slots for subsequent arguments; this is explicitly
commented in the implementation, not a bug inferred from another language.

Manual control: `f(a){if(1){b=a;}return b;} g(){return b;}` gives `f` its own
`a`/`b` slots and a separate `b` slot in `g`. The latter is an undefined local,
not a reference to `f`'s local. Constants are held per module and evaluated
through literal/unary/binary instructions only; ordinary assignment rejects
their names.

For `main(){helper();} helper(){}`, the pending unqualified call resolves to
the same module's function before consulting the builtin catalog. For two
includes that both define `helper`, an unqualified call with no local/builtin
match fails as ambiguous; the candidate set deduplicates repeated includes.
`other::helper()` records `other` as a dependency and requires a script function.
Thread calls never fall back to natives. These paths match GSC-RUNTIME.md.

## 4. Confirmed failures and manual regression witnesses

Each row is an independent source module named `audit`, with root `audit` and
the existing IW4 catalog. No witness requires additional modules or natives.
`C(x)` below abbreviates `Op::Constant(x)`.

| Witness | Baseline trace/result | Patched expectation |
| --- | --- | --- |
| `#define EMPTY ""` followed by newline and `main(){return EMPTY;}` | The define boundary search sees empty literal text as EOF and inserts `;` before the value; expression parsing fails. | Accept; constant is `Value::String("")`. Also accept the directive at EOF without a final newline. |
| `main(){a[""] = 1;}` | Assignment lookahead sees empty literal text inside brackets and returns false; expression parsing leaves `=` where `;` is expected. | Accept; emit `EnsureLocalArray(a), Load(a), C(""), C(1), StoreIndex`. |
| `main(){a[b[""]] = 1;}` | The same false EOF occurs at bracket depth two. | Accept; nested index scanning reaches both actual closing brackets and finds `=`. |
| `main(){return a["["];}` | Raw lookahead classifies the string `"["` as the second bracket of an indirect call; `expect("[")` rejects it. | Accept; expression fragment `Load(a), C("["), LoadIndex`. |
| `main(){return 1 "+" 2;}` | String text is treated as `Binary::Add`; invalid source is accepted. | Reject at the string token when `;` is expected. |
| `main(){return 1 "&&" 2;}` | String text enters short-circuit generation; invalid source is accepted. | Reject without emitting a short-circuit branch for that literal. |
| `main(){for(i "++";;)break;}` | `assignment` consumes raw string text as increment; the whole function is accepted. | Reject with `expected assignment operator` before consuming the literal. |
| `main(){for(i ^ 1;;)break;}` | A plain binary `^` is accepted as compound assignment through the fallback mapping. | Reject; the existing assignment vocabulary contains `^=`, not `^`. |
| `main(){for(i` with EOF immediately after `i` | `ident` reaches EOF; assignment consumes the empty token as an operator, then `emit(Load)` calls `location` with `pos == tokens.len()`: an out-of-bounds panic. | Return `expected assignment operator` at EOF without advancing past it. |
| `main(){switch(0){case -"1":break;}}` | The token before consuming minus is non-string; the following string payload is fed to `number("-1")`, emitting an integer case label. | Reject the signed string; preserve ordinary string labels and signed integer labels. |
| `main(){prof_begin "(" "label");}` | Raw lookahead plus `pos += 2` skips the string as if it were `(`; invalid source is accepted and erased. | Reject; profile-statement recognition requires actual punctuation. |

Positive companion witnesses, also traced manually: `main(){return "";}`,
`main(){return "+";}`, `main(){return "[";}`,
`main(){a["]"] = 1;}`, `main(){a["["] += 1;}`,
`main(){for(i=0;i<1;i++)break;}`,
`main(){switch(0){case -1:break;case "1":break;}}`, and
`main(){prof_begin("label");prof_end("label");}` retain their existing parsing.
For the compound index assignment the fragment is
`EnsureLocalArray(a), Load(a), C("["), DupPair, LoadIndex, C(1), Add, StoreIndex`.
String arguments in `self notify("(", "::");` remain ordinary event values.

Other incomplete-input controls: `main(){` returns `unterminated block`;
`main(){return (1;}` fails `expect(")")`;
`main(){for(i=;` fails operand parsing at `;`;
`main(){a[""]` reaches real EOF, fails assignment lookahead, then returns a
fault from `expect(";")` after reading the index. Missing RHS, fields, call
arguments, and closing brackets use existing `ident`/`expect` faults and do not
consume EOF as syntax in the changed paths.

## 5. Jump addresses and operand counts

[`emit` and `patch`][compiler-base] use absolute instruction indices. Patches
refer to emitted `Jump`/`JumpFalse` instructions; forward targets at the current
end are followed by further instructions or the implicit return pair.
[`statements`][statements-base] sends `for`/`foreach` continues to increment,
`while` continues to condition, and breaks to the end. A switch adds a break
context but no continue context, so continue searches the enclosing real loop.

Manual trace of `main(){return 1 ? 2 : 3;}`:
`0:C(1), 1:JumpFalse(4), 2:C(2), 3:Jump(5), 4:C(3), 5:Return,
6:C(undefined), 7:Return`. Both paths supply one return value. For `1 && 2`,
the expression fragment has `JumpFalse(8)` at pc 3 and `Jump(9)` at pc 7;
pc 8 supplies zero, and pc 9 is the consuming instruction. Both paths leave
one normalized boolean. These are hand-computed indices, not disassembly.

During developer statement elimination, the existing linker builds `keep` and
a prefix-count `remap` of length `old_len + 1`. Surviving jumps are mapped to
the count of surviving instructions before their old targets, including an
old end target. Example `main(){if(1)println("x");return 2;}` resolves the
catalog's developer `println` statement to elision: old pc 1's `JumpFalse(5)`
becomes `JumpFalse(2)` after removing old pc 2–4 (argument, call, pop). The
branch lands at `C(2)`. No jump representation or elision logic is modified.

[`arguments` and `invocation`][calls-base] count each parsed expression once.
For `f(a,b){return a;} main(){return f(1,2);}`, argument order is
`C(1), C(2), Call(f,2,false)`. Receiver calls put the receiver below arguments;
indirect calls put the function reference between receiver and arguments.
Special event calls emit receiver, event name, then payload; their count
excludes the first two operands.

## 6. Consistency with the existing instruction contract

The read-only comparison with [`runtime::instruction` and
`stack_effect`](../crates/sim/src/script/runtime/mod.rs) confirms the relevant
orders: binary operators pop RHS then LHS; `LoadIndex` consumes receiver/key;
`StoreIndex` consumes receiver/key/value; `DupPair` supplies the two additional
operands needed for compound index assignment. Calls consume `argc` values
plus an optional receiver; indirect calls also consume the function reference.
Notify/AwaitMatch consume payload plus receiver/name. `JumpFalse` consumes the
condition, and each explicit/implicit return generated here has a value.
The patch changes recognition of source tokens, not the IR schema or opcodes.

## Unconfirmed questions and deliberate limits

These observations are separate from the proven token-kind failures and are
not repaired in this PR. They require a compatibility decision or further evidence.

| Question and minimal witness | Manual observation | Why no defect claim/fix |
| --- | --- | --- |
| `main(anim){return anim;}` | Parameter setup reserves a local slot, but `load("anim")` emits `Global::Anim`; the parameter checks explicitly reserve only self/level/game. | The project's intended policy for this additional global name is not documented; possible reserved-name diagnostic gap. |
| `X=1; main(){foreach(X in []){}return X;}` | Foreach writes a local slot named x, while expression reads resolve the module constant first. Ordinary assignment would reject x. | Possible missing constant-name validation; whether foreach should reject or shadow needs an explicit language rule. |
| `/ # main(){return 1;} # / main(){return 2;}` | Developer exclusion matches adjacent tokens after whitespace/comments have been discarded, so it excludes the first function despite spaced delimiter characters. | It is unclear whether adjacency of characters, rather than tokens, is a promised rule of this implementation. |
| `/# main(){return "unterminated; } #/ main(){}` | String lexing faults before the developer token-removal pass can discard the disabled region. | Disabled-region lexical validity is unspecified here. |
| `main(){return "\q";}` | The escape fallback removes the backslash and keeps q. | Permissive unknown escapes may be intentional; no compatible version was consulted to assert otherwise. |

No claim of a complete grammar audit or full IW4 conformance follows from this
review. Integer wrap, function scope, duplicate parameter handling, native
thread restrictions, and the limited literal grammar are existing policies,
not targets for adding language features.

## Static verification and remaining limits

- The lexer still appends exactly one non-string EOF token for successful
  production lexing. `Token::is` distinguishes it from an empty literal.
- Assignment lookahead uses `get`; bracket depth changes only on non-string
  delimiters. Empty/bracket-containing strings advance the scan without
  changing depth. Real EOF exits; exhausting lookahead returns false.
- Assignment consumes an operator only after matching the same existing
  whitelist used by lookahead. EOF, quoted operators, and bare binary operators
  return a fault first. The later `op[..1]` slice is consequently a nonempty
  ASCII operator, and its XOR fallback is reachable only for `^=`.
- Binary parsing exits on a string without advancing, allowing its caller to
  diagnose the unexpected token; valid unquoted operators retain their old
  binding powers. Postfix/call/profile lookahead uses kind-aware matching.
- `#define` inserts its separator at the next source line or actual EOF,
  leaving an empty string in the expression. The signed-case branch rejects
  strings before numeric conversion; ordinary string and integer cases remain.
- Changed cursor increments were checked against the EOF sentinel. No new
  success path consumes that sentinel, and changed loops progress or return.
  Corrected index instructions have the receiver/key/value counts traced above.
- `git diff --check` passed. No executable validation was performed. In
  particular the contribution guide's publish-check, formatting, and clippy
  commands were not run, honoring the audit's memory constraint.

There is no existing compiler regression-test format in the tracked tree.
[`approved_tests`](../crates/approved_tests/README.md) contains only an approved
gameplay lifecycle scenario and prohibits unapproved permanent unit tests.
The small manual regression witnesses above are retained here instead of
introducing a new harness or changing that scenario. Build/type-checking and
executable regression validation remain unverified.

[compiler-base]: https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/sim/src/script/compiler.rs
[lexer-base]: https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/sim/src/script/compiler/lexer.rs
[expressions-base]: https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/sim/src/script/compiler/expressions.rs
[statements-base]: https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/sim/src/script/compiler/statements.rs
[calls-base]: https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/sim/src/script/compiler/calls.rs
