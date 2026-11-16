# Start and project failure cases

Written before implementation and tests.

## Start

- Opening the app or returning to New leaves the editor outside Insert.
- New loses text, ignores the named project or sidebar target, or replaces a remembered workflow.
- Launch chooses a project before the catalogue arrives and never resolves it afterward.
- A workflow absent from the server appears in the picker. A model search cannot choose a bare agent.
- Whitespace or oversized input creates a conversation. Enter sends instead of adding a newline.
- Repeated start, project, workflow or text keys change the pending submission.
- A create timeout clears the draft, resends create, or starts input.
- A refusal loses the draft or options_invalid fails to open the first erroneous option.
- Success clears the draft before create succeeds, fails to remember choices, or steals the stage after navigation.
- Device-local choices or draft text are lost across launch or leak between servers.
- Missing options, invalid options, missing workflow, no models, no projects and unreceived data look ready.

## Project and floats

- A field cursor uses an index and moves when fields are inserted above it.
- Fields lose schema order, reveal a secret, prefill a secret, or edit an unsupported schema.
- Edits send immediately. Reset sends a default value instead of a null draft.
- Save sends unchanged keys, combines extensions, duplicates an in-flight write, or clears drafts on refusal.
- A successful reply discards a newer edit made while saving.
- bad_options errors appear under the wrong field or disappear when the draft is kept.
- Work counts differ from the model's polls or unreceived counts show as zero.
- Projects cannot open their Project view, or work links fail to name the project.
- Bind submits from a text field, allows an invalid name or relative server path, or sends without Confirm.
- Cancelling Confirm sends a write. Add extensions includes already-bound extensions.
- Escape in a bind field closes instead of going to the checklist. Hints disagree with the focused part.
- Picker navigation loses the selected identity, cannot filter, or lets global keys reach the page.
- Draft marks or the save indicator stay visible after success or disappear on close.

## Implementation and verification sequence

1. Ground the screen, key, model, and frame boundaries.
2. Compare screen-owned state with a shared controller. Keep screen-owned state because the Screen trait already owns dispatch and lifecycle.
3. Keep shared writes limited to float registration and the save-state hook. One owner implements the coupled selection flows.
4. Write screen tests before their implementation.
5. Verify the focused tests, render and compare each named scene, then run the workspace test and lint gates.
6. Review and commit each verified unit. No history rewrite or push.

Model the Domain selects typed submission state, keyed option drafts, and frozen picker rows. Prove It Works requires rendered scenes and command dispatch tests rather than compilation alone.
