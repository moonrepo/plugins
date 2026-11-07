---
status: proposed
date: 2026-11-06
applies-to: [docs/adr/**]
---

# Record architecture decisions

## Context

Plugins in this repository share behavior that moon and proto users rely on: how versions resolve,
when installs run, and what gets written to their files. These decisions have lived in audit plans
(`docs/todo`), PR descriptions and changelogs, so each plugin decides them again, sometimes
differently. For example, the Rust and Go audits gave opposite answers on whether setup should check
pinned bins against their installed versions.

Most changes here are made by agents, and an agent only follows rules it can find. Architecture
decision records (ADRs) give these rules one place, a stable way to cite them, and a record of why
they were made.

## Decision

### When to write an ADR

1. Write an ADR for a decision that applies to more than one plugin, to every plugin of a type
   (tools, backends, toolchains or extensions), or that sets a behavior users rely on.
2. Decisions that only affect one plugin stay in that plugin's code, changelog or audit plan.

### Files

3. ADRs live at `docs/adr/NNNN-kebab-title.md`. Numbers have 4 digits, are assigned in order, and
   are never reused.
4. A new ADR starts as `docs/adr/draft-kebab-title.md` with `status: proposed`. It's numbered when
   it's accepted, so drafts on parallel branches don't collide.
5. ADRs MUST follow the template below.

### Rules

6. The Decision section is a numbered list of rules, using MUST, MUST NOT, SHOULD and MAY as defined
   in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119). A rule is cited as `ADR-NNNN.N`, for
   example `ADR-0001.6`. Numbering continues across subheadings, so each rule's citation is unique.
7. Code that enforces a rule, or is an exception to one, SHOULD cite it in a comment, for example
   `// ADR-0003.1`.
8. A rule SHOULD be enforced by a shared helper in `crates/common` (or the matching `*-common`
   crate) when it can be, so complying means calling the helper.
9. Exceptions MUST be listed in the ADR's Exceptions section with their reason. Code that isn't
   listed MUST comply.

### Lifecycle

10. Only a maintainer accepts an ADR. Agents MAY draft ADRs, and MUST NOT set `status: accepted`.
11. An ADR is accepted in the same PR that brings existing code into compliance. Code that can't
    comply is listed under Exceptions instead.
12. Once accepted, an ADR's rules don't change. Wording MAY be clarified. To change a rule, write a
    new ADR that supersedes the old one, and set the old one's status to `superseded by NNNN`. An
    ADR retired without a replacement is `deprecated`.
13. The ADR index in `AGENTS.md` MUST be updated in the same PR that adds an ADR or changes its
    status.

### Following ADRs

14. Before changing a plugin, read every accepted ADR whose `applies-to` matches a changed path.
15. When a change can't be made without breaking an ADR, stop and ask a maintainer. Don't deviate
    silently, and don't edit the ADR to fit the change.

### Template

````md
---
status: proposed # proposed | accepted | superseded by NNNN | deprecated
date: YYYY-MM-DD # date of the last status change
applies-to: [toolchains/**] # paths whose changes this ADR governs
---

# Title, as a short statement of the decision

## Context

The problem, and why it needs a shared decision. Link the issues, PRs and audits behind it.

## Decision

1. Numbered rules, using MUST, MUST NOT, SHOULD and MAY.

## Exceptions

Plugins or cases that are exempt from a rule, and why. Otherwise "None."

## Consequences

What gets easier or harder, and the trade-offs accepted.
````

## Exceptions

None.

## Consequences

- Cross-plugin rules live in one place, and code comments point back to them.
- Accepting an ADR includes the work to bring existing code into compliance, so ADRs land slower,
  but an accepted ADR always describes the code as it is.
- The index in `AGENTS.md` is maintained by hand, so it can drift from the ADR files.
