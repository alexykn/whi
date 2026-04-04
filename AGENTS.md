# AGENTS.md

Default rules for working in this repository. These apply to all languages and components unless a task or language-specific section explicitly overrides them.

If a task explicitly asks for something else, follow the task but note the deviation.

## Subagent Usage (Plan & Build)

- Primary agents for planning and implementation must use built-in subagents for efficiency and context hygiene.
- Use `@explore` for codebase exploration, file discovery, pattern matching, grep-like searches, and "where is X?" questions. Prefer it over manual file reads when possible.
- Use `@general` for complex research, multi-step analysis, external knowledge, or open-ended reasoning that would otherwise bloat the primary context.
- Delegate early and often: before any non-trivial plan, ask `@explore` to gather relevant context.
- When planning needs deep research or external reasoning, delegate to `@general`.
- During implementation, use `@explore` again for targeted verification before editing when it will reduce primary-agent context.
- Do not duplicate subagent work in the primary agent. Delegate first, then incorporate the result.

## Core Principles

- Preserve existing behavior unless the task explicitly requires a change.
- Make the smallest correct change possible unless the task explicitly asks for something else.
- Prefer composition and explicit dependency injection over other patterns.
- Keep code readable and maintainable for both humans and future agents.
- Flatten control flow; avoid deep nesting.
- Separate pure logic from I/O and side effects.
- Never modify unrelated files, reformat out-of-scope code, or update dependencies unless the task requires it.

## Workflow & Verification

### Verification

- Prefer single-file checks when they are sufficient and faster.
- Run the full test and lint suite only when explicitly requested or before marking the task complete.

## Boundaries

### Always

- Run the relevant linter and type checker on changed files before finishing.
- Preserve existing behavior unless the task explicitly requires a change.

### Ask First

- Adding new third-party dependencies.
- Changes that touch multiple files or public APIs.
- Any modification to `AGENTS.md` itself.

### Never

- Rewrite or reformat code outside the explicit scope of the task.
- Add tests unless explicitly asked.
- Swallow exceptions or add silent error handling.

## 1. Logic & Control Flow

### Do

- Flatten control flow (prefer max 2 levels of nesting, 3 absolute max). Do not count `def`/`try`/`while`/`async with` toward nesting depth.
- Use guard clauses and fail-fast early in functions.
- Invert conditions to avoid nested branching when it improves readability.
- Evaluate specific cases first, leave default behavior at the bottom.
- Extract small helpers if they meaningfully flatten the main path.

### Don't

- Bury logic in deep `if/else` trees.
- Use `else` when an early return/continue/break would keep the code flatter.
- Trade readability for cleverness.

## 2. Architecture & Design

### Do

- Prefer the smallest correct change.
- Avoid long parameter lists; if a function needs more than 3-4 arguments, group related parameters into logical structures.
- Prefer strongly typed objects over massive dictionaries when passing complex state.
- Use composition + explicit constructor injection (especially for orchestration/coordinating layers).
- Use ABCs and `abstractmethod` when components need to be swappable.
- Split files, classes, or functions before they violate single responsibility.
- Keep modules and files tightly scoped and cohesive.
- Keep pure logic separate from network, file I/O, and other side effects.

### Don't

- Add abstraction layers unless they clearly reduce real complexity.
- Build deep inheritance hierarchies.
- Create god classes, god files, or god functions.
- Introduce DI containers or service locators without a clear, justified need.
- Don't blindly append new code to existing structures; extract and reorganize instead of letting them grow indefinitely.

## 3. Type System & Documentation

### Do

- Prefer modern native annotations (`list[str]`, `dict[str, int]`, `X | Y`, etc.).
- Use `Protocol`, `TypedDict`, `TypeVar`, or `Any` only when they add clear value.
- Use generics when they add real value, especially for interchangeable injectable components and DTOs.
- Add precise type hints on public interfaces and non-trivial variables.
- Write concise docstrings for public APIs when they clarify intent, side effects, or invariants.
- Add comments explaining why for complex or non-obvious decisions.

### Don't

- Overuse generics or indirection unless they demonstrably pay for themselves.
- Use `Any` for structured data.

## 4. Reliability, I/O & Errors

### Do

- Validate inputs at boundaries.
- Make failure modes explicit and fail fast.
- Use explicit timeouts on network/external calls.
- Clean up resources deterministically.
- Add contextual logging at key boundaries and failure points (never log secrets, keys, or PII).
- Handle transient failures gracefully with retries only when appropriate.

### Don't

- Swallow exceptions silently.
- Block the main thread or async event loop with sync I/O or heavy CPU work.

## 5. Dependencies & Environment

### Do

- Inject configuration and secrets via environment variables or config files (never hardcode).
- Prefer standard library solutions when they are sufficient.
- Use established ecosystem standards when they clearly reduce boilerplate or improve maintainability.
- Add a third-party dependency only when it provides clear, ongoing value in maintainability or functionality.

### Don't

- Add dependencies just because they are popular or convenient.

## 6. Testing & Maintenance

### Do

- If tests already exist, run the relevant suite before you start coding (to establish a baseline and catch pre-existing breakages) and after (to ensure no regressions).
- Ensure your code passes all currently present tests.
- When explicitly asked to write tests, target only the complex, high-risk logic. Plan them out and treat them as a distinct, separate step.
- Mock external dependencies and I/O in tests; test pure logic where possible.

### Don't

- Don't write new tests unless explicitly asked to do so by the user.
- Don't pester the user about missing tests or test coverage.
- Don't practice implicit Test-Driven Development (TDD) or bundle haphazardly written tests alongside feature code.
- Don't refactor purely for aesthetics.
- Don't change behavior silently.
- Don't modify files outside the explicit scope of the task.

## Python

### Setup & Commands

- Use `uv` exclusively for dependency and environment management.
- Manage configuration strictly via `pyproject.toml`.
- Always work inside `.venv`; create it with `uv venv` if it is missing.
- Sync dependencies with `uv sync`.
- Run the linter and formatter with `uv run ruff check --fix` and `uv run ruff format`.
- Type-check with `uv run ty check`.
- Use `pytest` for all test suites.

### Syntax & Patterns

- Rely on modern Python 3.12+ features.
- Prefer native type annotations (`list[str]`, `dict[str, Any]`, `X | Y`) over the legacy `typing` module unless constrained by an older codebase.
- Use `@dataclass` or Pydantic to encapsulate related data and execution contexts instead of creating functions with massive signatures.

### Don't

- Use the legacy `typing` module when native annotations are sufficient.

## Rust

To be added.

## Frontend

To be added.
