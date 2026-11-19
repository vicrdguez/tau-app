# Tau desktop usability review

**19 November 2026 · macOS 15.5, Apple silicon · revision `cd01930`**

Three changes deserve the first implementation pass. Restore typing focus when a pairing warning closes. Keep the selected row visible in short windows. Make the two most prominent actions on Start and in the decision reader respond to a click. Each interrupts a core task, and each has a reproducible keyboard or pointer comparison below.

The keyboard model worked particularly well when Tau made the next step visible. Filtering by typing, opening failed tool output in place, and reviewing exactly which notes accompany an answer are worth preserving. The proposed changes keep those paths available.

## What was exercised

Launched the packaged fixture with `nix run .#demo`. For direct computer use, opened the matching Nix-built macOS bundle and verified its executable matched the packaged binary byte for byte. Used pointer clicks, native key events, text entry, screenshots, and the macOS accessibility tree. Tested Connect separately with a fresh temporary profile. Ran `nix run .#e2e-ui` against the repository’s temporary loopback server and echo model. All three checkpoints passed, including verification of the assistant’s reply.

| Surface | Interaction coverage |
| --- | --- |
| Connect | First launch, empty submit, focus between fields, editing the URL, HTTP warning and cancellation, connection refusal. The live-server test paired, reached Home, bound a project, and started a conversation. |
| Start | Typed a draft, tried pointer and keyboard project selection, searched to zero results and back, chose a project and workflow, and started. Native replays exercised binding from an empty server, opening missing options, and retrying an uncertain start. |
| Home and sidebar | Moved among groups and rows, inspected previews, opened objects, navigated through the sidebar and open items, used hint mode, and toggled the sidebar. Replayed the compact preview and selection at 800 px wide. |
| Conversations, Timeline, Faults | Filtered running conversations and project activity, opened a result, viewed empty project activity, hid faults, revealed hidden rows, and restored a row. Compared Home activity with the full Timeline. |
| Projects and project detail | Opened a project, inspected Work and Extensions, edited and committed an option field, saved, navigated away and back, filled and cancelled Bind, and toggled an extension. Replayed reset, compact checklist selection, and bind confirmation. |
| Conversation reader | Opened a running transcript, expanded failed tool output, toggled Details, inspected model and thinking pickers, composed a steer, and queued a follow-up. Replayed tool navigation, folds, stop, continue, and multiline send. Inspected joined, owned, record, failed, and refused-send scenes. |
| Decision reader | Opened a forecast, added a whole-decision note, tried the header action, opened readback by keyboard, changed choice, expanded notes, cancelled, reopened, and answered in the fixture. Replayed source/visual selection, long-document annotation, conflict dismissal, superseded-answer handling, and send-again cancellation. |
| Commands, preferences, recovery | Used Find, the command palette, command line, leader help, Keys, Jumps, Messages, Settings, Server, and Outbox. Changed motion and exercised key rebinding. Replayed reconnect, re-pair from a revoked fixture, inspecting an unknown write, and cancelling external-link confirmation. Reviewed and cancelled the forget-server prompt. |

Reviewed renders of all **78 named native scenes**, including light theme, loading, offline, revoked, conflict, and unknown outcome. Inspected eight additional scenes at **800 × 600**. Sampled sidebar, Details, fold, and send transitions at intermediate and settled frames. The direct window review, fixture replays, and real-server test answer different questions. A scene establishes what Tau shows in that state. Only the live-server test establishes the pairing-to-reply round trip.

## High priority

### 1. Connect loses typing focus after the HTTP warning

**Reproduce.** In a fresh profile, enter a dummy pairing code and `http://example.invalid:6283`. With the Server field focused, press Return and then Escape. Press a printable key. The badge still says **INSERT**, the accessibility tree reports no focused field, and neither value changes. Clicking Server restores typing. Reproduced with both text entry and a single keypress.

Someone who cancels specifically to correct the address can type a replacement into nowhere. In a related recovery case, submitting to an unused loopback port produced a helpful refusal, but editing the address left the previous host’s error under the new value.

**Change and retest.** Restore native focus and the caret to the initiating field when the warning closes. Clear an address-specific error when that address changes, or label it as the previous attempt. After Escape, Command-A and typing should replace the URL immediately, retain the pairing code, and leave no stale error attached to the new address. [Connect](../crates/tau-app/src/screens/connect/mod.rs)

### 2. Short dialogs let the selection move out of sight

**Reproduce.** Render `bind-float` at 800 × 600. The checklist shows `w1` and `w2`. From the first row, press Control-J twice. The highlight disappears. Press Tab and Return. The confirmation asks to bind with **w1, websearch**. The selected third row starts at y=506 and its label at y=511, in the clipped area above the footer. The confirmation is the first visible evidence of which extension was toggled. This matters because binding an extension is permanent for that project.

Settings has the same failure. From the `settings` scene at that size, press Down eight times. The last fully visible row is “find anything,” the footer offers “new key,” and the selected “commands” row begins at y=518 behind the footer. A user can start rebinding a command they cannot see.

**Change and retest.** Scroll the body to keep the complete active row visible whenever selection moves, including reverse navigation and after resize. Keep the search and action footer fixed. At 800 × 600, navigate to every extension and past the first page of Settings using only the keyboard. The visible highlight, spoken label, and subsequent confirmation or key capture should always name the same item. [Float shell](../crates/tau-app/src/ui/shell.rs) · [Settings rows](../crates/tau-app/src/floats/settings.rs) · [Bind checklist](../crates/tau-app/src/floats/project_bind.rs)

### 3. The first required choice on Start looks actionable but ignores clicks

**Reproduce.** Open New with no current project, type a draft, and click “choose a project.” The composer keeps focus and no picker opens. Option-P opens Project immediately with the draft intact. The native tree exposes “choose a project” and “M-p” as separate text elements. On macOS the printed `M-p` also requires the reader to know that Meta means Option.

**Change and retest.** Make the project and workflow labels named controls with the same command handlers as their shortcuts. Use the native Option glyph or provide its name in help. Give the Start affordance the same treatment for its current action, including “bind” and “set its options.” Starting from the empty form, a pointer user should be able to pick a project and workflow, return to the same draft, and start it. [Start view](../crates/tau-app/src/screens/start/view.rs)

### 4. The coral Answer control cannot open the answer review

**Reproduce.** From Home, open a “Resolve forecast” decision. Click the solid coral **Answer** pill in the top-right corner. The reader stays open with no feedback. Press Space, A, D and the review appears. The header exposes the word and key hint as text. A nearby Details control does respond to a click.

Inside the review, “y confirm” is exposed as a button. Activating it leaves the review open and puts “press y to confirm” in the distant bottom bar. Pressing Y after reviewing the selected choice records it. The extra deliberate keystroke serves a purpose. The confusing part is encountering two button-shaped elements with two different meanings.

**Change and retest.** Let the header control open readback. Phrase the final instruction as “Press Y to send this answer,” adjacent to the selected choice or confirmation footer. Preserve the 400 ms arming period, the review of notes, and the existing protection against click, Return, and key repeat sending an answer. Verify a click can reach review and Escape can leave it with the draft intact. [Header](../crates/tau-app/src/frame/header.rs) · [Readback](../crates/tau-app/src/screens/decision/readback/view.rs) · [Decision interaction rules](spec.md)

### 5. Outbox asks users to open a decision without identifying it

**Reproduce.** Answer the first forecast, then open Outbox with Space, R, U. The completed row reads “answer · forecasts · ” followed by “done” and a time. The nearby conversation input identifies both its conversation and its text. In the `unknown-outcome` scene, the answer row reads “answer · render-core · ” followed by “no reply · open the decision to send it again.” Several decisions can belong to the same project. The row omits the one fact needed for recovery.

**Change and retest.** Label an answer write with its decision title and reference, use the stored reference as a fallback, and provide an open-decision action. Continue to require the decision’s own readback for a resend. With two answer entries in one project, a user should be able to open the correct decision from each row and see exactly which choice they are reviewing. Omit separators for absent fields. [Outbox row construction](../crates/tau-app/src/screens/places/floats.rs) · [Outbox view](../crates/tau-app/src/floats/outbox.rs)

## Medium priority

### 6. Hiding the last fault removes the route back

On an untouched demo, open Faults with Space, O, F. Press X on both tasks. The settled page reads **“No matching rows.”** The paragraph explains that hiding is local, but gives no hidden count or recovery action. Space, T, H reveals both rows, and X can restore one. A person returning later could read the empty page as a healthy project.

Distinguish “no faults,” “no matches for this filter,” and “2 faults hidden on this device.” Put **Show hidden** beside the last message, with its shortcut. Retest in all-project and single-project scope, including hiding the final visible fault and restoring it without consulting help. [Places](../crates/tau-app/src/screens/places.rs)

### 7. Shortcut names lose essential keys in the accessibility tree

On Start, the visual Command-Return hint is exposed as “⌘” and “start,” with Return missing. New’s shortcut is exposed as “cn,” and the key-help result for filtering is “pf.” Both sequences require a leading Space. Home’s nested count buttons are named simply “3” and “4.” These omissions were visible in the macOS accessibility tree. VoiceOver speech was not tested.

Give each shortcut a complete accessible name generated from the active keymap, including modifiers, leader, and special keys. Name count controls by purpose, such as “3 open decisions” and “4 failures.” Retest the native tree and spoken navigation after rebinding a key. Keep the compact visual glyphs. [Key rendering](../crates/tau-app/src/ui/keys.rs) · [Sidebar](../crates/tau-app/src/frame/sidebar.rs)

### 8. Pickers hide how to search and what to do at zero results

In Project or Find, type `zzzz`. All rows disappear. The footer reports “0 projects” or “0 rows” and still offers “choose.” Before typing, the compact project picker shows a caret beside “3 projects.” There is little indication that the caret is a search field. A person looking for a missing project cannot tell at a glance whether the server has none or their query excluded all three.

Label the focused field “Filter projects” or “Search projects, conversations, decisions, and events.” At zero matches, show the query and a clear-filter action where the list was. Suppress the choose hint until a result is available. Retest with zero, one, and multiple matches, and confirm clearing the query restores the original list and keyboard focus. [Project picker](../crates/tau-app/src/floats/project_picker.rs) · [List picker](../crates/tau-app/src/floats/list_picker.rs)

### 9. Tab changes meaning halfway through binding

Open Bind, enter a name, then Tab to working directory and Tab again. In the extensions list the same key now changes a checkbox. Pressing it to reach a presumed next field changes what will be bound. The footer switches from “Tab next” to “Tab toggle.” In the fixture checklist, Down twice left the cursor on the first row, while Control-J twice selected the third. The picker conventions used elsewhere make Down a reasonable expectation here.

Let Tab and Shift-Tab traverse form regions, arrows move through extensions, and Space toggle the current one. Keep existing expert bindings where they can coexist. Show movement, toggle, and review keys together. Retest with zero selections, a single extension, and multiple extensions. The confirmation should list the deliberate selection exactly. [Bind input](../crates/tau-app/src/screens/project/actions.rs)

### 10. The project option save boundary is easy to miss

In render-core, select the `base` option, press I, replace its value, and press Return. The field stops editing and displays the new value. The remaining evidence that it is unsaved is “1 not saved” and `:w` in the window’s bottom bar. On the wide display this is far from the edited row. A successful field commit looks like a successful project save.

Mark the changed field locally and display a named “Save 1 change” action beside the options heading. Keep the command-line save. Recheck field commit, save, refusal with the draft retained, and reopening on a real disposable server. During this demo, `:w` cleared the dirty indicator and the field returned to “default main.” The fixture synthesizes a write response from static options, so that observation does **not** establish live data loss. [Project actions](../crates/tau-app/src/screens/project/actions.rs) · [Fixture write behavior](../crates/tau-client/src/fixture.rs)

### 11. Active filters are easier to apply than to remove

In Timeline, use Space, P, F to select `notes-cli`. The breadcrumb names the project and the empty page says “No events yet. Bare conversations write none.” Events from other projects are still available. In Conversations, select “running.” Two projects each show one matching row, but their headers also report failures and totals of four and seven conversations. Those totals are useful context, though they make poor result counts.

Give the active scope a visible clear action. On an empty scoped list, offer “Show all projects.” On filtered conversation groups, distinguish the visible count from the project total, for example “1 running shown · 4 total.” Verify clearing Timeline’s project leaves the independent Conversations filter intact. [Place filters](../crates/tau-app/src/screens/places/floats.rs)

### 12. Compact answer review truncates the note scope at the point of commitment

At 800 × 600, `readback-answer` shows “3 sent with the answer · 2 on pas…” beside “K reads them.” Space remains above the review. In the interactive forecast review, Shift-K did reveal the note text and its scope, and Escape preserved the draft. The summary still asks users to open a second view to check information that is already part of the confirmation.

Let this row wrap or stack counts beneath the choices when horizontal room is scarce. Keep the selected choice, note counts and scopes, downstream task, and final confirmation visible together. Retest with both whole-decision and passage notes at 800 px, then with no notes and one note. [Readback layout](../crates/tau-app/src/screens/decision/readback/view.rs)

## Visual design and motion

| Finding | Evidence | Proposed change and acceptance check |
| --- | --- | --- |
| **13 · Opening Details displaces the passage being read.** Medium | In the 1512 × 982 `details-slide` replay, the transcript and composer both remained 880 px wide. Their left edge moved from x=440 before opening to x=330 at 0.30 s and x=320 when settled. The reader travels 120 px to reveal metadata even though its line breaks need no change. | Use the available side margin first, or hold the reading anchor while Details enters. Check the active line and caret before, during, and after opening and closing. At narrower widths verify the selected passage stays in view through any necessary reflow. |
| **14 · Home activity loses the detail that distinguishes repeated events.** Medium | In the wide-window demo the first two activity rows both read “Nov 14 · Forecast is due. · forecasts #2.” Timeline distinguished them at 19:02 and 18:00. The selected preview or a trip to Timeline is needed to tell which occurrence a row represents. | Show date and time when day, title, and object repeat. Keep the project and conversation reference on compact rows. Retest with same-day duplicates and events from different days. |
| **15 · Old work is hard to date from the duration.** Polish | The live fixture reader showed “111 h 34 m ago,” and a running tool showed more than 109 hours. The fixture clock accounts for the unusually old examples. The formatter expresses every duration above an hour in hours and minutes. | Use a day-scale age for multi-day spans, with an exact local timestamp on inspection. Check 23 h 59 m, 24 h, and several days in both the decision reader and tool view. [Time formatting](../crates/tau-app/src/time.rs) |
| **16 · Settings gives three preferences and key bindings the same visual weight.** Polish | Opening Settings reported “162 settings.” Theme, reading width and motion occupy the first three rows. Shortcut entries then fill the same list, including consecutive “commands” entries for its two bindings. Typing “motion” immediately reduced the list to one and Right successfully switched it off. | Group preferences and shortcuts, preserve type-to-filter across groups, and count each group accurately. Verify someone can find motion, inspect either command binding, and reset a rebound key without losing the query. |

Keep the current fold and send behavior as comparisons when tuning motion. In `fold-glide`, tool output expanded beneath the selected row and the composer stayed at y=871 across all four samples. In `send-enter`, the multiline composer shrank from 130 px to 94 px to 91 px with its lower edge anchored. In the sidebar control, the motion-off scene had reached its final geometry at 0.30 s while the normal scene was still animating. The measurements describe the sampled frame geometry.

## Reproducing the review

From the repository root, these commands cover the interactive fixture, live-server control, compact problem cases, and the measured Details transition.

```sh
nix run .#demo
nix run .#e2e-ui
nix run .#snap -- bind-float 800x600
nix run .#snap -- settings 800x600
nix run .#snap -- readback-answer 800x600
nix run .#snap -- unknown-outcome 800x600
nix run .#script -- details-slide 0.30 conversation-streaming
nix run .#script -- details-slide 0.70 conversation-streaming
```

The snapshot and replay commands write to the ignored `target/snaps/` directory. For the two Details samples, inspect or copy the first frame before running the second command, which reuses its filename. [Nix commands](../nix/commands.nix) · [Native replay scripts](../tools/scripts) · [Live UI assertions](../tools/e2e/ui.py)

**Coverage limits.** The review covers every screen family and the named scene catalogue, with representative input paths through each family. It does not establish every keybinding permutation, VoiceOver behavior, Linux rendering, or long-running network behavior. Offline, revocation, conflicting answers, and uncertain writes used fixtures. Forget-server was tested through cancellation only. Live persistence of edited extension options and a real multi-device answer conflict remain follow-up verification cases.
