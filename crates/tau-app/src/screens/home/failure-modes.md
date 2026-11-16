# Home and places failure cases

Written before implementation. Tests exercise row projections and commands against the fixture link.

- An unreceived inbox looks calm, or unknown polls look empty.
- Needs you sorts by project rather than request time.
- Failed excludes a fault or counts a hidden failure. Its ordering puts an unlinked fault before a failed run.
- The eight-row Failed cap loses overflow counts or opens the wrong filtered place.
- A cursor stored as an index moves to a different object when a row arrives above it. Removal must choose the next surviving row and explain an answered decision.
- A hidden failed run remains hidden after a poll observes it idle or running.
- Hidden state, place filters, and cursors leak between places or disappear on restart.
- A short absence shows the since-line. Failed runs incorrectly enter its counts. The base advances while focused.
- Narrow rows truncate an ID or keep optional slots while the label has less than 200 px.
- A filter affects Home, or searches fetch labels for every undrawn conversation.
- Timeline paging requests only one project's log, or sorts older pages by entry ID instead of time.
- A peek opens an item, takes the stage stream, fetches before 300 ms, or leaves its stream open after departure.
- A sidebar peek follows the stage rather than the sidebar cursor.
- Find ignores its kind prefix, or changes a moved selection as new data arrives.
- Outbox retry sends an answer, create, stop, restored settings, or a done entry. Forget sends a write.
- A float lets global keys reach the page underneath it.
- Count tag clicks open Home indiscriminately rather than the oldest decision, Failed, or Outbox.

Verification units are list behavior and Home, places and peek, then floats and frame clicks. Snapshot checks cover dark, light, calm, connecting, narrow, reduced list width, running peek, offline, and sidebar focus. The final workspace test and lint gate runs on the committed candidate.

The data shape is a row identity enum with object IDs. Rendered rows are derived from Server. Home owns its cursor and away base. Places owns separate filters and cursors. Peek owns only its pending target, deadline, and shown target. This is smaller than a second normalized UI model and avoids synchronizing server copies. Floats draw plain row data; their screen controllers own client actions.
