# Contributing

The project is intended to be as Rust-standard in its approach as possible. The overall goal is
to produce a useful commandline tool suitable for those comfortable running a service on their
local computer or a machine they control.

## Branching Strategy

PRs to `main` only please.

## Commits

It should be obvious from the commit content what is being achieved.

### Messages

The commit message should use [AP Style Headline Capitalization][headlinecaps] (4 characters or
longer get capitalized), and be phrased in the present tense.

Subsequent lines should be written as ordinary English sentences.

### Code Changes

Should compile and pass tests. This permits easy bisecting. Try to use correct formatting in
each commit; Avoid large commits that simply correct formatting - these obscure the original
context of earlier code commits.

## Coding Standard

In all applicable cases, the output of [`rustfmt`][rustfmt]'s
`cargo fmt` is the correct formatting option.

In cases where `rustfmt` is unopinionated, such as with this markdown, the following apply:

* maximum line length of 100 characters
    * Where introducing newlines this is not possible, no length limit applies.
* No trailing whitespace.
* Trailing newline at EOF.

### Intellij

Compliant Rust formatting can be added to Intellij using the [`rustfmt` instructions]
[intellijrustfmt].

[headlinecaps]: https://headlinecapitalization.com/

[intellijrustfmt]: https://github.com/rust-lang/rustfmt/blob/master/intellij.md

[rustfmt]: https://github.com/rust-lang/rustfmt
