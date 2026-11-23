# Agent notes

## Issue and PR text

- Write issue and PR text in English.
- Use ASD-STE100 style: short sentences, active voice, simple words. Use one
  idea per sentence.
- Put the summary first. Put long logs, code, and optional details in a
  `<details><summary>...</summary>` block.
- Use the issue templates in `.github/ISSUE_TEMPLATE/`.
- Use `.github/pull_request_template.md` for every PR.
- Link the issue in the PR. Add external sources only when they are relevant.

## Stacked PRs

- Keep one concern per PR.
- Use 1 to 3 commits per PR.
- Keep the diff at or below 150 to 200 changed lines. Do not count
  `package-lock.json` or other generated lock files.
- Base each PR on the previous branch in the stack.
- Add a short verification step and show the result.
- Use branch names like `feat/session-telemetry-core`.

## Code

- Keep the candidate UI quiet. Do not add debug text or internal terms.
- The app is local-first. Do not send candidate data to a network service
  without explicit consent.
- Keep the VM overlay out of session archives.
- Run `npm run build` and `npm run lint` before you push.
