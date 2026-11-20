# tau desktop: prototype states, calm pass

47 states (49 renders: `narrow-800` is a sheet of three frames) to render headlessly from the package fixture, in the calm direction. Each state names its window and theme, its mode and which region has the keys, its fixture content, and what it must show. The package text wins over this list if they disagree: `../../design/spec.md` (screens), `reference/visual-language.md` and `tokens.json` (look), `reference/grammar.md` and `keymap.toml` (keys), `client.md` (calls), `fixture.json` (data). The end of this list maps each state to the file name it had in the design's first pass.

**Calm challenge (2026-11-15).** The rulings on the calm challenge (summarised in `../../design/evidence.md` §6), and one more from the prototype's checks (no `global` under a float, every float's own badge: called *the float-keys ruling* below), change what some states must show. Each such state carries a line *Changed by …*; states 44 to 47 are new. Changed: 1, 2, 3, 4, 5, 7, 8, 9, 13, 15, 16, 17, 18, 19, 20, 21, 22, 24, 27, 28, 29, 30, 31, 34, 35, 36, 37, 39, 41, 42, 43. New: 44 `home-connecting`, 45 `start-starting`, 46 `sidebar-focus-decision`, 47 `conversation-hostile`.

## Defaults for every state

- **Window** 1512 × 982 logical px, **Ember dark**, unless the state says otherwise.
- **Frame:** the sidebar shown (`local.sidebarVisible`), the details panel hidden (`local.detailsVisible`), no notice band, no float.
- **Time** is `fixture.now` (2026-11-14 16:40 UTC). Ages, and the count of a running tool call, are computed from it and from the entries' `timestamp`s. A finished tool row shows no duration (D12).
- **Connection:** connected, every stream live, the server line calm: `laptop → tau.tail4e2f.ts.net ⌄`.
- **Data:** inbox = `streams.inbox.first`; timelines = `streams.timeline *`; conversation states = `calls.conversations *`; faults = `calls.faults *`; labels = `local.firstInput`; models and extensions of conversations = `local.modelOf`, `local.extensionOf`.
- **Sidebar:** the places; `OPEN` from `local.openItems` (notes-cli: `#4`; render-core: `Review the design #2`, `#5`, `render-core`; forecasts: `Resolve forecast #1`), each with its mark; plus the state's own item when it opens another object (it joins the end of its project's group). The current row is the state's view. Count tags `◆ 3 × 4` on the Home row; `?` only where said.
- **Mode** `NORMAL`, the stage has the keys, unless said. A float that takes the keys names its own mode in the badge (`COMMAND`, `READBACK`, `OUTBOX`, `SERVER`, `PEEK`), never `NORMAL`, and nothing outside it acts.
- **Overrides** say exactly what changes in the fixture for that state.

## Checks to run on every render

- **CORAL:** solid coral only on the `◆` tag, the stage's Needs you head while the stage has the keys, `◆ ␣ad Answer` in Normal with no float open, the readback's tag and badge, the sidebar's avatar and the wordmark's τ; coral text only in `title.page`. Outside a readback at most one solid mass besides the tags and the avatar; none in calm states. Never in agent or extension text.
- **CURSOR:** measured on visible pixels, not on styles: exactly one coral cursor (a 2 px bar inside the column, the coral Needs you head, or a caret with its container's coral edge), in the region that has the keys; every other cursor a neutral band. Zero only under a float that takes the keys and has no selection (send-again, a reply, Confirm, Server).
- **COUNTS:** `◆` equals the Needs you count and the inbox size; `×` equals the Failed head (failed conversations plus faults not hidden); the same wherever the tags sit (sidebar or bottom bar).
- **PRESENCE:** a count above 0 has exactly one visible tag (the Home row, or the bottom bar while the sidebar is not drawn); an abnormal link (`○ connecting` included) shows the server's words in exactly one visible place. Two seeded faults: one removes a tag, one removes the server's words; each must fail.
- **CHIPS:** at most 3 chips in the stage (header included) and 1 in the bottom bar at first sight, the sidebar's 2 fixed, none in the echo; floats, footers, the start line and container hints exempt.
- **RESOLVE:** every lit chip and hint resolves in its active context, generated from `keymap.toml` with its builtin sets. The context and mode come from the region that has the keys and the badge the render shows, never from a label the render declares. Strict in Insert.
- **DIM:** one rule (`spec.md` §4.6): a chip, hint or segment is lit only when its key resolves for the region and mode that have the keys; otherwise `fg.muted`. The Answer is solid only when `␣ad` would open the readback. Checked in Insert, under every float, and with the sidebar holding the keys (46).
- **CONTRAST** ≥ 4.5:1 for text and 3:1 for marks on the surface they sit on, in all three variants.
- **IDS:** `#id`, `#n`, decision IDs, subjects, task kinds and paths' leaves are never cut, in rows and in the sidebar; a faulted task's `after <decision ID>` span shows whole or not at all.
- **SPOOF:** extension and agent text in rows and titles is one plain line; Markdown only in transcript, basis and peek bodies; `title.page` only on the stage's first line; content `h1` rose with its rule. Run on the fixture and on the `hostile` block (47).
- **MEASURE:** prose lines ≤ 600 px.

---

## First run

1. **`connect`** · 1512 × 982, Ember · `INSERT` in the code field.
   - Data: no server yet. Server `http://127.0.0.1:6283`; code `K7QM-3XRT` typed.
   - *Changed by TC-25, TC-26:* no sidebar toggle and no breadcrumb; the two fields are text fields (`surface.sunken`, hairline, `radius.pill`, 26 px), the code field's edge coral with the caret.
   - Must show: the sidebar not drawn; the window controls in the stage header; the coral wordmark (`tau-wordmark-on-dark-coral`, 44 px); `Connect to a tau server` in `title.page`; the paragraph naming `tau device pair <name>`; the neutral `⏎ connect`; no count tags; the bottom bar `INSERT`.

2. **`connect-http-warning`** · as 1, Server `http://192.168.1.20:6283`.
   - *Changed by TC-07 and the float-keys ruling:* `⏎` pressed; the Confirm float open.
   - Must show: the warning of `client.md` §7 under the Server field; no `y` anywhere in the form; the Confirm float: head `Confirm`, body `Pair over plain HTTP? Anyone on the network path can read this device's token.`, hints `y pairs · n cancels`; badge `READBACK` in its plain form (coral edge, not solid); no coral cursor (the float has no selection), the code field's edge back to the hairline.

3. **`bind-float`** · `COMMAND`, the bind float has the keys (its extensions checklist).
   - Override: `calls."projects (first run)"`: no projects. Home behind: `No projects yet · ␣pb binds one`; the sidebar without `OPEN` and without tags.
   - *Changed by TC-08, TC-26 and the float-keys ruling:* the hints follow the focused part; the fields are text fields.
   - The float shell: head `Bind a project`; key-value grid: name `scratch` and working directory `/home/vrod/src/scratch` (text fields, `a path on the server` beside the second), extensions checklist from `calls.extensions` (`websearch tool`, `w2 workflow`, `w1 workflow`), `w1` checked, cursor on `w1` (coral bar); badge `COMMAND`; hints `Tab toggle · ⏎ bind · Esc cancel`.

4. **`project-options-unset`** · `NORMAL`, the stage.
   - Data: `calls."bind scratch"`, `calls."options scratch"`; a `scratch` group in `OPEN` with its project item, current.
   - *Changed by TC-26:* `3 to set` in rose.
   - Must show: breadcrumb `Projects › scratch`; title `scratch`; `working directory /home/vrod/src/scratch · a path on the server`; Work rows with zeros; Extensions `w1 workflow · 3 to set` (rose); Options · w1: `designer`, `implementer`, `reviewer` `not set` with `required` in rose, `base` `default main` in `fg.muted`; cursor on `designer`.

## The start page

5. **`start`** · `INSERT`, the start composer.
   - Data: `local.lastProject` render-core, `local.lastWorkflow` w1; no `local.startDraft`.
   - Must show: breadcrumb `New conversation`; the sidebar's `+ New` current; the wordmark 44 px, 36 px above the 640 px composer, whose top is 296 px below the header; placeholder `What do you want to work on?`; the composer's coral edge and caret; `⌘⏎ start` inside at the bottom right; the line `render-core M-p · w1 workflow M-w`, its two hints lit. Nothing else in the stage.
   - *Changed by TC-04, TC-09:* in Insert the leader is inactive, so the sidebar's `␣cn` and `␣.` are muted; `M-p` and `M-w` stay lit.

6. **`start-project-picker`** · `COMMAND`, the Project float has the keys.
   - Over 5 after `M-p`.
   - Must show: the float shell: head `Project` · `for the new conversation`; rows `render-core` (`w1 · websearch`, `/home/vrod/src/render`, `last used`, cursor with the coral bar), `notes-cli` (`bare agent only`), `forecasts` (`w2 · websearch`); foot input with `3 projects`, hints `⏎ choose · Esc back`. The composer's edge back to the hairline, no caret; the sidebar's chips muted.

7. **`start-options-unset`** · `INSERT`.
   - Data: `calls."bind scratch"` added to the projects, `calls."options scratch"`; the start draft `local.startDraftExample` with project `scratch`.
   - *Changed by TC-03, TC-04:* the line comes from the `options` item's errors; the sidebar's chips muted (Insert).
   - Must show: the typed text; the line `scratch M-p · w1 workflow · needs options: designer, implementer, reviewer M-w` with the last part in rose; the hint `⌘⏎ set its options`.

8. **`start-no-reply`** · `INSERT`.
   - Data: as 5 with the text of `local.startDraftExample`; `⌘⏎` pressed and `calls."create w1 render-core"` timed out (no reply).
   - *Changed by TC-02:* after the timeout the text is editable again and the hint is back to `⌘⏎ start`.
   - Must show: the page still the start page with the text; under the start line the mauve `?` and `No reply · a conversation may have been created; it would show as empty conversation in render-core`; the Home row's tags `◆ 3 × 4 ? 1`.

9. **`start-light`** · as 5 in **Ember Light**.
   - *Changed by TC-27, TC-04:* Light's new `surface.side` (`#ede3d0`) and `surface.current` (`#d8ccb0`) on `+ New`; the sidebar's chips muted.
   - Must show: `tau-wordmark-on-light-coral`; the composer on Light `surface.raised` with its coral edge; the avatar unchanged.

## Home

10. **`home`** · `NORMAL`, the stage.
    - Must show: breadcrumb `Home › Needs you › 1 of 3`; title `Needs you` with the 18 px `◆ 3` tag; the since-line `Since 12:30 on this device · away 4 h 10 m · 3 new for you · 1 new fault · 5 events`.
    - Needs you (oldest first, coral diamonds): `Review the design #2` / `render-core · w1:3:design` / 2 h 45 m; `Resolve forecast #1` / `forecasts · w2:2:pi-durable-2` / 2 h 40 m; `Resolve forecast #2` / `forecasts · w2:2:rune-release` / 1 h 38 m. Cursor on the first: solid coral head with `⏎`, byline on the cursor band.
    - Failed 4 (rose `×`, slots project · `#id` · extension · reason · time): `w1.design_answer after w1:6:design#1` render-core #6 w1 task faulted 14:10; `Write a migration test for v3 notes…` notes-cli #5 run failed; `The owner approved this design in decision…` render-core #5 w1 run failed; `w2.due` forecasts #2 w2 task faulted Nov 12 08:03.
    - Running 2 (steel triangles): `Port the config loader…` notes-cli #4 claude-sonnet-4-5; `Design a retention policy…` render-core #7 w1 claude-opus-4-5.
    - Activity: the 5 events after the base in `ui.strong`, then older ones to 12 rows, then `N older in the timeline`.
    - The peek (440 px, hairline at its left): decision `render-core/w1:3:design#2`, the basis cut at the peek's foot with `the rest is in the decision`.
    - The sidebar: Home current (neutral fill, no bar).

11. **`home-peek-running`** · as 10, cursor on Running `#4` (band with the coral bar).
    - Data: `streams.open notes-cli #4`.first.
    - Must show the peek: label; `notes-cli · #4 · claude-sonnet-4-5 · medium · project default`; `running · turn 6`; the last agent text (`The network is closed here…`); the running `bash npm test -- test/config.test.ts` with its last 4 output lines; `1 queued · steer`.

12. **`home-calm`** · `NORMAL`, the stage.
    - Overrides: inbox `{"forecasts": {}, "notes-cli": {}, "render-core": {}}`; render-core #5 and notes-cli #5 `idle`; `local.hiddenFaults` holds both fault task IDs; no since-line (never left).
    - Must show: `Nothing needs you` (coral title, no tag); no tags on the Home row; no diamonds and no `×` on the open items; Running 2 and Activity; cursor on the first Running row; peek as 11. The only coral: the title, the avatar.

13. **`home-1300`** · **1300 × 982**, Ember, as 10.
    - *Changed by TC-15:* the `after` span whole and the kind cut first; the since-line drops whole parts.
    - Must show: the peek still a 440 px column (the stage is 1052 px); the list column 532 px; Failed keeps project, `#id` and reason (time and extension dropped); Running keeps project, `#id` and extension (model dropped); labels cut with `…`; the first Failed row reads `w1.design_an… after w1:6:design#1`, never a cut ID; the since-line ends on a whole part (`… · 3 new for you`), never inside a word.

14. **`sidebar-focus`** · `NORMAL`, **the sidebar has the keys**.
    - As 10, then `C-h`, then `j` to `Review the design #2`.
    - Must show: the sidebar's cursor on `Review the design #2` (fill and 2 px coral bar); Home still the current row (neutral fill, no bar); the stage's Needs you head drawn as the neutral band, its diamond coral, no solid coral fill; one coral cursor in all.

## Conversation

15. **`conversation-streaming`** · `NORMAL`, the stage; `#4` current in the sidebar.
    - Data: `streams.open notes-cli #4`: first frame, then `later[0]` and `later[1]`.
    - Must show: breadcrumb `notes-cli › #4`, `␣cx stop`; title `Port the config loader from JSON to TOML. Keep …` (coral, one line); facts `▶ running · claude-sonnet-4-5 · turn 6` and `details ›`.
    - Transcript: your message (container on `tint.human`, `input 16:21`); `tools: read, write, edit, bash`; one `agent 16:21` header for the turn; `✓ 2 tool calls · read, bash   ok`; text; the failed row `× bash npm install smol-toml@1.3.1   failed` on `tint.row_fail` (the only tinted row); text; `✓ 2 tool calls · write, edit   ok`; `▶ bash npm test -- test/config.test.ts   running 17 m` with its last 6 output lines open on `surface.sunken`. No finished row and no fold carries a figure.
    - The queue: `steer · queued` and `Also keep the NOTES_CONFIG environment variable working…` on the content bar, `queued inputs can't be taken back`; the composer, caption `steer · joins this run after the current tool round`, no hints. Cursor on the last entry, the running tool row: a whole-row band with the coral bar inside the column; follow on.
    - *Changed by TC-16, TC-06:* durations only on the running row; the cursor's bar drawn inside the column.

16. **`conversation-steer-insert`** · `INSERT`, the composer.
    - As 15 with the draft `Use node:test's snapshot support for the TOML fixtures.`
    - *Changed by TC-16:* the tool rows as 15.
    - Must show: the composer's coral edge and caret; hints `Esc jk leave Insert · ⌘⏎ steer · M-⏎ follow-up`; `␣cx stop` and the sidebar's chips muted.

17. **`conversation-failed`** · `NORMAL`; `#5` (render-core) current, its rose `×` in the sidebar.
    - Data: `streams.open render-core #5`.
    - *Changed by TC-16:* the failed test row reads `failed` with no figure.
    - Must show: `␣cc continue`; facts `× run failed · model_error · claude-sonnet-4-5`; the transcript ending with the failed test row, then the run-failure head on `tint.row_fail` (`Run failed · model_error`), `429 rate_limit_error…` in `mono.small`, `␣cc continue runs it again on the same context`; the composer caption `send · starts a new run · the last run failed`.

18. **`model-picker`** · `COMMAND`, the workflow list has the keys.
    - *Changed by TC-32:* now the start page's one flat list of workflows and models, not `␣cm`'s picker.
    - Over 5 after `M-w`.
    - Must show: head `Workflow` · `for the new conversation in render-core`; rows `w1 · workflow` (cursor, coral bar), then `bare agent · Claude Opus 4.5`, `bare agent · Claude Sonnet 4.5`, `bare agent · Claude Haiku 4.5`, `bare agent · GPT-5.1`, `bare agent · Gemini 3 Pro` (from `calls.models`); no model step; foot input empty with `6 rows`, hints `⏎ choose · Esc back`; the composer's edge back to the hairline.

19. **`conversation-joined`** · `INSERT`, the composer; `#7` joins render-core's group, current.
    - Data: `streams.open render-core #7` with `later[0]`.
    - *Changed by TC-16:* the fold carries no figure.
    - Must show: `␣cx stop` (muted, Insert); facts `▶ running · claude-opus-4-5 · turn 3`; your message; `tools: read, bash, web_search, publish_design`; `✓ 2 tool calls · web_search, read   ok`; the generation `agent · writing`, its last block in `fg.muted`; the composer empty with the caption `steer · …` and its Insert hints.

20. **`conversation-records`** · `NORMAL`; `#3` joins render-core's group, current; **the details panel open** (`␣td`).
    - Data: `streams.open render-core #3`. Cursor on the second `w1.design` record.
    - Must show: title `Add an export of the event log: tau export --events should write every…` (backticks dropped); records with the header `w1.design` (orange mono) `by a task` and no box; the design as Markdown (rose `h1` with its rule, orange `h2`, the `json` code block with its gold rule, the table); the long input with `<design>` as literal text.
    - *Changed by TC-06, TC-26:* the cursor on the record's header line is a whole-row band with a visible coral bar; the details panel stacks each label over its value and names the unset thinking level.
    - The details panel (240 px, hairline): Settings, each label (steel) over its value, the model ID never broken (`anthropic/claude-opus-4-5 · Claude Opus 4.5`, thinking `model default`, `tools project default + w1`, `cwd …/render`, `w1 may set the model again when it next sends work here`); Decisions `◆ Review the design #2`; Events `13:55 The design is ready for review.`, `13:30 The designer is revising the design.`, `09:52 The design is ready for review.`; the facts line's `details ‹`.

21. **`conversation-owned`** · `NORMAL`; `#8` joins render-core's group, current.
    - Data: `streams.open render-core #8`.
    - *Changed by TC-06:* a visible coral cursor on the last entry's line, its bar inside the column.
    - Must show: facts `idle · claude-sonnet-4-5 · owned by #3`.

## Decision

22. **`decision-long`** · `NORMAL`, the stage; `Review the design #2` current.
    - Data: `calls.decision render-core/w1:3:design#2`, drafts `local.noteDrafts`.
    - Must show: breadcrumb `render-core › w1:3:design#2`; `◆ ␣ad Answer` solid coral; title `Review the design` (coral, no `#2`); facts `open · requested 13:55 · 2 h 45 m ago` and `details ›`; the basis from its top: `NDJSON export of the event log` (rose `h1` with its rule, 24 px below the title block), `Why`, `Shape of a line` (orange), the json block (ending at the column's edge, never under the margin, its clipped edge faded), the bullets.
    - *Changed by TC-26:* the code block stops at the 600 px column's edge.
    - The margin: `Your notes · 3 · not sent`; `on the whole` `Close. Answer the two notes and I'll approve.`; `› Earlier: #1 revise · 2 notes`; the note `Reported where? Say what the command prints.` beside `A longer event is written whole; the limit is reported, not enforced.`, which carries `tint.note_range`. The document cursor on that bullet, so its passage carries `tint.note_active`.

23. **`decision-note-insert`** · `INSERT`, the note being typed.
    - As 22. Override: `local.noteDrafts` holds the passage note and the note on the whole only. `VISUAL` over `two exports of the same store are byte-equal.`, then `␣mn`, typing `Byte-equal only while history keeps its key order across core versions. Say whether it does.`
    - Must show: the typed note as a container with a coral edge beside its passage (`note on the selection · not sent`, caret, `Esc jk leave Insert` inside); its passage on `tint.note_active`; the `Reported where?` note pushed below it, with a 1 px `rule.note_leader` line from the column's edge at `A longer event is written whole…` to its head; the head still `Your notes · 3 · not sent`; the Answer as a muted outline; the badge `INSERT`.

24. **`readback-answer`** · `READBACK`, the readback has the keys.
    - Over 22 after `␣ad`, `2` and a `y` pressed inside the 400 ms, which was discarded.
    - Must show: badge `READBACK` and tag `◆ ANSWER` solid coral; head `Review the design #2 · render-core`, meta `open · requested 13:55`; left column `1 approve`, `2 revise` (selection, coral bar, 600), `3 abort`, `j k or 1–3`; a hairline; right column `notes 3 sent with the answer · 2 on passages, 1 on the whole  K reads them`, `basis entry 34 · 2,203 characters · shown to the end`, `then w1.design_answer runs in #3`, `from this device, laptop`, its first row on the left column's first baseline; hints `y confirm · n Esc q cancel`; echo `read it first`; the Answer muted; the page scrolled so the cursor's passage sits above the float, clipped at the float's top edge, the insets beside and under the float plain `surface.read`.
    - *Changed by TC-23.*

25. **`decision-forecast`** · `NORMAL`; `Resolve forecast #1` current.
    - Data: `calls.decision forecasts/w2:2:pi-durable-2#1`.
    - Must show: the basis verbatim with `Resolve by 1794664800000 (epoch milliseconds).`; Choices `1 yes`, `2 no`, `3 void`, `4 not yet`, cursor on `2 no`; `Your notes · none`; the coral Answer.

26. **`decision-superseded`** · as 22 after `streams.inbox.later[2]`.
    - Must show: the notice band `Superseded by #3 · this request can no longer be answered` (steel `time` mark); facts `superseded by #3`; no Answer segment; the sidebar's `Review the design #2` without its diamond; the drafts still in the margin; `◆ 3` unchanged (the subject is still open, as `#3`).

27. **`answer-conflict`** · `READBACK` (plain), the reply float has the keys.
    - *Changed by TC-21, TC-05, TC-23 and the float-keys ruling.*
    - Over 22 after `y`, with `refusals."conflict (answer w1:3:design#2 after the phone answered approve)"`.
    - Must show: tag `◇ NOT RECORDED` (outline, neutral); key-value grid `reply Not recorded. phone answered approve at 16:42.`, `their choice 1 approve`, `their notes 0 notes`, `yours your choice revise and your 3 notes were not sent; they stay on this device`; hints `Esc closes · your notes stay on this device`; badge `READBACK` in its plain form (coral edge, not solid).
    - Behind it the view has taken the reply's record: facts `answered approve · by phone 16:42`, no Answer segment, no diamond on the sidebar's `Review the design #2`; the `◆` tag follows the inbox (3 until it changes). No coral cursor anywhere (the reply has no selection); the page clipped at the float's top edge. Under the float `␣cn`, `␣.`, `␣ad` and `␣cx` do nothing.

28. **`decision-then-faulted`** · `NORMAL`; `Review the design #1` (w1:6:design#1) joins render-core's group, current, with its rose `×`.
    - Data: `calls.decision render-core/w1:6:design#1`.
    - *Changed by TC-22:* the margin head `Notes · none · answered by phone 14:10` with no mark; the cursor on the first line of the text column.
    - Must show: facts `answered approve · by phone 14:10 · then w1.design_answer · × faulted`; `␣ar send again` (quiet); the head `The work after this answer failed` on `tint.row_fail`, the error in `mono.small`; choices with `●` on approve and `chosen by phone · 14:10`.

29. **`readback-send-again`** · `READBACK`.
    - Over 28 after `␣ar`.
    - *Changed by TC-05, TC-23:* no coral cursor anywhere: the float has no selection and the stage's cursor is neutral under it; the insets show `surface.read`.
    - Must show: badge `READBACK` and tag `◆ SEND AGAIN` solid coral; head `Review the design #1 · render-core`, meta `answered approve · phone 14:10`; the key-value grid alone: `its work`, `sends the same answer: approve, 0 notes, basis entry 17`, `then a new w1.design_answer starts in #6`, `from this device, laptop`.

## Places and project

30. **`timeline`** · `NORMAL`; Timeline current in the sidebar.
    - Must show: breadcrumb `Timeline › all projects`; title `Timeline`; the since-line; all events of `timeline render-core` and `timeline forecasts` merged by `at`, newest first, kinds in `mono.small`, the 5 after the base in `ui.strong`; cursor on `The design is ready for review.` (13:55, `#3`); the peek: that event, then conversation #3's peek, with no line inside the peek.
    - *Changed by TC-26:* the hairline inside the peek is gone.

31. **`faults`** · `NORMAL`; Home current in the sidebar (Faults has no row).
    - Must show: breadcrumb `Faults › all projects`; title `Faults`; the sentence under it; rows `w1.design_answer` (render-core, `after w1:6:design#1`, `#6`, 14:10) and `w2.due` (forecasts, `#2`, Nov 12 08:03), rose `×`; cursor on the first; the peek: the fault and its decision, with no line inside the peek.
    - *Changed by TC-26:* the hairline inside the peek is gone.

32. **`conversations`** · `NORMAL`; Conversations current.
    - Data: `local.places.conversations` (no filter, cursor on render-core #7).
    - Must show: breadcrumb `Conversations › all projects`; groups `forecasts 2 in all`, `notes-cli 1 running · 1 failed · 4 in all`, `render-core 1 running · 1 failed · 7 in all`; rows newest ID first with marks, labels, extensions, models, `#id`; cursor on render-core `#7`; the peek live, as 11 for #7.

33. **`project`** · `NORMAL`; the `render-core` project item current.
    - Data: `calls.projects`, `calls."options render-core"`.
    - Must show: breadcrumb `Projects › render-core`; title `render-core`; `working directory /home/vrod/src/render · a path on the server`; Work `Conversations 7 · 1 running · 1 failed`, `Open decisions 1`, `Faults 1`; Extensions `websearch tool · all set`, `w1 workflow · all set`, `An extension can't be removed from a project.`; Options · w1 (`designer Claude Opus 4.5 · anthropic/claude-opus-4-5`, `implementer …`, `reviewer GPT-5.1 · openai/gpt-5.1`, `base default main`); Options · websearch (`provider exa`, `apiKey set · hidden`); cursor on `reviewer`.

34. **`options-error`** · `NORMAL`; as 4 after `:w` with `designer = anthropic/claude-opus-9`.
    - Data: `refusals."bad_options (setOptions designer to an unknown model)"`.
    - *Changed by TC-26:* `3 to set` in rose.
    - Must show: under `designer`, rose `expected format model`; the field's sage `edit` mark; Extensions `w1 workflow · 3 to set` in rose; the bottom bar `✎ 1 not saved :w` at its right; echo `The options of w1 do not fit.`

## Floats

35. **`which-key-root`** · `NORMAL` with `␣` pending.
    - Over 10, 300 ms after `␣`.
    - Must show: head `␣ leader` alone; groups `a ◇ answer`, `c conversation`, `o open`, `p project`, `m this view`, `r recover`, `t toggle`, `h help` in `fg` 600; a hairline; `n next decision`, `. find anything`, `; commands`, `y yank to clipboard` in `fg.alt`; hint `Esc closes`; the bottom bar's pending `␣`; the Needs you head still solid coral (which-key takes no keys).
    - *Changed by TC-23:* the insets beside and under the float show `surface.read`, no line of the page.

36. **`which-key-answer`** · `NORMAL` with `␣a` pending.
    - Over 22 after `␣a`.
    - *Changed by TC-23:* as 35.
    - Must show: head `answer` · `every command reads back what it sends before y`; `d ◇ answer this decision` live; `r ◇ send the answer again · its work did not fail` greyed.

## Connection and failure

37. **`server-stopped`** · `NORMAL`; **the sidebar hidden** (`local.sidebarVisible` false).
    - Over 15 after `streams.ends.stopping` on every stream.
    - *Changed by TC-24, TC-13:* the view's stream is down, so its state is marked; PRESENCE runs on the bottom bar.
    - Must show: the window controls in the stage header, content from x 88; the bottom bar's right end `◆ 3`, `× 4` and `◷ server stopped · retrying` (`fg`), each once; echo `The server is stopping. tau reconnects when it is back.`; `stop · off` (dashed, no chip); facts `▶ running · claude-sonnet-4-5 · turn 6 · as of 16:40`; the running row's count stopped at `17 m`; the transcript as last drawn.

38. **`offline`** · `NORMAL`; over 10 at 16:47, no stream for 6 min.
    - Must show: the notice band `Offline since 16:41 · showing the state as of then · 3 notes and 1 unsent input are safe · writes resume when connected` with `␣rc`; the sidebar's server line `○ offline 6 m`; tags `◆ 3 × 4 ? 1`; the peek's facts ending `as of 16:41`.

39. **`unknown-outcome`** · `OUTBOX`, the Outbox float has the keys.
    - *Changed by TC-24 and the float-keys ruling:* the stream is down, so the header reads `stop · off` and the facts end `as of 16:40`; the badge reads `OUTBOX`; only the outbox's keys act (`j k`, `K`, `␣rt`, `x`, `Esc`, `q`).
    - Over 15, reconnecting; `local.outbox` with both entries.
    - Must show: the server line `○ reconnecting in 4 s`; above the composer, under the queue, the mauve `?`, `follow-up`, `Use node:test's snapshot support for the TOML fixtures.` on the content bar, `no reply · resends when connected`; `? 1`; the Outbox float: head `Outbox` · `every write, kept until done`, meta `1 without a reply`, rows for both entries, cursor on the unknown one, hints `␣rt retry · x forget · Esc closes`.

40. **`revoked`** · `NORMAL`; over 22 after `streams.ends.revoked`.
    - Must show: the rose notice band `This device was revoked on the server. Nothing more is sent. To use it again, run tau device pair <new name> on the server (a revoked name can't be reused), then ␣rp.`; the server line `× revoked`; `Answer · off` (dashed, no fill, no chip); the drafts in the margin.

## Samples

41. **`home-light`** · state 10 in **Ember Light**.
    - *Changed by TC-27:* Light's `surface.side` `#ede3d0`, the Home row's `surface.current` `#d8ccb0` (1.25:1), the cursor band `surface.cursor.read` `#e2d6be`.

42. **`decision-light`** · state 23 in **Ember Light**: the note tints, the leader, the coral title against the rose `h1`.
    - *Changed by TC-27:* Light `tint.note_range` `#dcd9c2` (1.17:1 on `surface.read`) and `tint.note_active` `#d1d4bc`.

43. **`narrow-800`** · **800 × 982**, Ember; three frames on one sheet; the sidebar hidden by itself (under 1000 px).
    - *Changed by TC-12, TC-11, TC-16:* the folded margin keeps its head, the note on the whole and Earlier; the sidebar is simply not drawn (the stored choice is untouched); frame 2's tool rows as 15.
    - Frames: state 10 (no peek, one column, the tags `◆ 3 × 4` at the bottom bar's right end, each once); state 15 (the 640 px column); state 22 (the text column 600 px; under the facts line, in this order, `Your notes · 3 · not sent`, `on the whole Close. Answer the two notes and I'll approve.`, `› Earlier: #1 revise · 2 notes`; each passage's note under its paragraph).


## Added by the calm challenge

44. **`home-connecting`** · `NORMAL`, the stage; Home current.
    - *New, TC-01.*
    - Data: launch; no stream has delivered a frame and no poll has answered. Only `local.*`.
    - Must show: the sidebar's server line `○ connecting` (`fg`, ring mark); no count tags on the Home row; the open items with no marks; breadcrumb `Home`; title `Home` (coral `title.page`, no tag); one line `Nothing received from the server yet` in place of the groups; the peek column empty. No `Nothing needs you` anywhere. PRESENCE: the server's words once.

45. **`start-starting`** · `INSERT`, the start composer, `create` in flight.
    - *New, TC-02.*
    - Data: as 5 with the text of `local.startDraftExample`; `⌘⏎` pressed, `calls."create w1 render-core"` not yet answered.
    - Must show: the hint `starting…` in place of `⌘⏎ start`; the text read-only (no caret, the composer's edge back to the hairline); the start line's `M-p` and `M-w` muted; the sidebar's chips muted; nothing else changed.

46. **`sidebar-focus-decision`** · `NORMAL`, **the sidebar has the keys**, over a decision.
    - *New, TC-04.*
    - As 22, then `C-h`; the sidebar's cursor on `Review the design #2` (the current row: fill and 2 px coral bar).
    - Must show: `◆ ␣ad Answer` as its muted outline (`␣ad` doesn't resolve in the sidebar); the decision's document cursor a neutral band; the passage's `tint.note_active` unchanged; the sidebar's `␣cn` and `␣.` lit (they resolve there); one coral cursor in all. DIM runs here.

47. **`conversation-hostile`** · `NORMAL`, the stage.
    - *New, TC-29.*
    - Data: the fixture's `hostile` block: `hostile.project` added to the projects; `hostile.inbox` merged into the inbox; `hostile.conversations`; `hostile."open hostile #2"`; labels from `hostile.firstInput`. A `hostile` group in `OPEN` with `#2` (current), `◆ Needs you · ok #1` and `#3`.
    - Must show, per `hostile.cases`: the title `# Needs you` as one plain coral line in `title.page`, no heading style; your message headed `input`, never `you`, its `you: ◆ 3 …` as plain text; U+202E, U+202C and U+200B as visible escapes on `surface.bar`; the agent's `**ok** · ◆ Answer · ␣ad` as Markdown (bold steel, inline code), no chip, tag or coral; the fenced `✓ exit 0` a neutral code block; the tool row `× bash npm test   failed` on `tint.row_fail` though its output says `✓ exit 0`; the last message's `# Review the design` a rose `prose.h1` with its rule; in the sidebar the decision's title one plain line with the client's own coral diamond, and `#3`'s label cut with `…`, `#3` whole. Tags `◆ 4 × 4`.
---

## From the previous mockups

Each kept state takes the old file's name unless shown otherwise; the README lines for `mockups/` can be rewritten from this table.

| Old mockup | Calm state |
|---|---|
| `connect.png` | `connect` |
| `connect-http-warning.png` | `connect-http-warning` |
| `bind-float.png` | `bind-float` (the confirm is now its own float and isn't drawn) |
| `project-options-unset.png` | `project-options-unset` |
| `new-conversation-picker.png` | **dropped**: the picker is gone; `start`, `start-project-picker` and `start-options-unset` replace it |
| `home.png` | `home` |
| `home-peek-running.png` | `home-peek-running` |
| `home-calm.png` | `home-calm` |
| `conversation-streaming.png` | `conversation-streaming` |
| `conversation-steer-insert.png` | `conversation-steer-insert` |
| `conversation-failed.png` | `conversation-failed` |
| `model-picker.png` | `model-picker` |
| `conversation-joined.png` | `conversation-joined` |
| `conversation-records.png` | `conversation-records` (now with the details panel open) |
| `conversation-owned.png` | `conversation-owned` |
| `decision-long.png` | `decision-long` |
| `decision-note-insert.png` | `decision-note-insert` (now typed on another passage, to draw the leader) |
| `readback-answer.png` | `readback-answer` (two columns) |
| `decision-forecast.png` | `decision-forecast` |
| `decision-superseded.png` | `decision-superseded` |
| `answer-conflict.png` | `answer-conflict` |
| `decision-then-faulted.png` | `decision-then-faulted` |
| `readback-send-again.png` | `readback-send-again` |
| `timeline.png` | `timeline` |
| `faults.png` | `faults` |
| `conversations.png` | `conversations` |
| `project.png` | `project` |
| `options-error.png` | `options-error` |
| `which-key-root.png` | `which-key-root` |
| `which-key-answer.png` | `which-key-answer` |
| `server-stopped.png` | `server-stopped` (now with the sidebar hidden) |
| `offline.png` | `offline` |
| `unknown-outcome.png` | `unknown-outcome` |
| `revoked.png` | `revoked` |
| `home-light.png` | `home-light` |
| `decision-light.png` | `decision-light` (now over the note being typed) |
| `width-1300.png` | `home-1300` (one frame; the decision fits at 1300 px and needs no frame) |
| `narrow-800.png` | `narrow-800` |
| — | new: `start`, `start-project-picker`, `start-options-unset`, `start-no-reply`, `start-light`, `sidebar-focus` |
| — | new in the calm challenge: `home-connecting`, `start-starting`, `sidebar-focus-decision`, `conversation-hostile` |

37 old mockups carry over (one renamed), 1 is dropped, 6 states are new in the calm pass and 4 in its challenge: 47 in all. `model-picker` keeps its file name but now shows the start page's workflow list (TC-32). The four wireframes (`wire-home`, `wire-conversation-streaming`, `wire-decision-long`, `wire-readback-answer`) are owed again over `home`, `conversation-streaming`, `decision-long` and `readback-answer`, with the calm frame's regions labelled by their boxes (`spec.md` §4).
