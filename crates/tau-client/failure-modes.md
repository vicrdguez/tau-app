# How the link can fail

Written before the code, one list per module. Each failure names the test that would catch it. Tests live in
`tests/`, feed the public seams (the engine's inputs and the model the UI reads), and take their data from
`design/fixture.json` and the real server streams recorded in `crates/tau-client/tests/data/` where a shape matters.

## ops (`tests/ops.rs`)

| Failure | Test |
|---|---|
| `r` merges into the old value instead of replacing it | `replace_drops_the_old_value` |
| `s` on an array index at the end is refused, or beyond the end is allowed and leaves holes | `set_appends_at_the_end_and_refuses_beyond` |
| `s` through a missing key creates the path instead of refusing | `a_path_through_nothing_is_refused` |
| `d` on an array leaves a `null` or doesn't shift the rest | `delete_removes_array_items_and_keys` |
| `a` on a non-string silently creates or stringifies | `append_and_trim_need_a_string` |
| `t` counts bytes or chars instead of UTF-16 code units (`✔`, emoji) | `trim_counts_utf16_units` |
| `p` removes and inserts at the wrong place, or refuses splicing at the end or on the root | `splice_removes_then_inserts`, `splice_and_reorder_reach_the_root` |
| `m` is applied as `new[order[i]] = old[i]`, or accepts a non-permutation | `reorder_takes_new_from_old_by_order`, `reorder_refuses_what_is_not_a_permutation` |
| `__proto__`, `constructor`, `prototype` accepted somewhere in a path | `prototype_segments_are_refused_anywhere` |
| `s d a t` with an empty path replace the root | `only_r_p_m_reach_the_root` |
| an unknown verb or a short wire form is silently skipped | `unknown_or_short_ops_are_refused` |
| ops of one frame are not applied in order | `ops_apply_in_order` |
| negative or fractional indexes accepted | `indexes_are_whole_and_non_negative` |
| the notes-cli #4 frames end in a state other than a fresh frame would show | conformance `open_frames_converge_to_the_fresh_state` |

## sse (`tests/sse.rs`)

| Failure | Test |
|---|---|
| a frame split across reads at any byte is lost, doubled or garbled (UTF-8 too) | `frames_survive_any_split` |
| a heartbeat is not reported, so a quiet stream looks dead | `heartbeat_is_a_frame` |
| `data:` without the space, CRLF or CR line ends break parsing | `line_ends_and_spacing_vary` |
| several `data:` lines of one event are not joined with newlines | `data_lines_join` |
| an `end` with an unknown reason is dropped instead of ending | `end_reasons` |
| malformed JSON panics or stops the parser | `a_bad_frame_does_not_stop_the_stream` |
| a frame with no blank line yet is emitted early | `an_unfinished_frame_waits` |
| the recorded server streams don't read as the frames they hold | `recorded_streams_parse` |

## wire (`tests/wire.rs`)

| Failure | Test |
|---|---|
| the status decides: a 409 or 408 proxy page read as a refusal, a 200 refusal read as done | `the_body_decides_not_the_status` |
| JSON that is none of the envelopes read as done or refused | `other_json_is_unknown` |
| `unknown` (storage_failed) read as refused | `the_unknown_envelope_is_unknown` |
| a reply of another protocol read as this one | `another_protocol_is_unknown_and_named` |
| a fixture refusal code falls to `Other`, or loses `errors`, `decision`, `open`, `project`, `calls` | `every_fixture_refusal_reads_with_its_fields` |
| an unknown code is read as done | `an_unknown_code_is_a_refusal` |
| a result with fields this client doesn't know fails to read | `unknown_fields_are_ignored` |
| the token is printed by `Debug` | `the_token_never_prints` |
| `input` sends `whenIdle`, or `whenBusy` when sending to an idle conversation | engine `input_bodies_follow_the_mode` |

## shapes (`tests/shapes.rs`)

| Failure | Test |
|---|---|
| `pi.user` with array content has no text, or a label from an image block | `user_content_string_or_blocks` |
| a label keeps more than its first line, isn't cut at 80 characters with `…`, or reformats `# Needs you` | `labels_follow_the_rule` |
| assistant blocks lose thinking or tool calls, or the model | `assistant_blocks` |
| a tool result loses `isError`, its call ID or its text | `tool_results` |
| `pi.system`, `pi.reset`, `pi.compaction` fail to read or read as records | `built_in_kinds` |
| a record's text is taken from a non-string `data.text` | `records_take_string_text_only` |
| an entry with a missing optional part fails to parse | `entries_are_read_leniently` |
| a stopped run (`aborted`) counted as failed; failed vs running vs idle wrong | `live_state` |
| queue items lose their mode or array content | `queue_items` |
| `pi.agent.extensions` absent / list / `{add, remove}` confused | `agent_selection` |
| missing options read from `required` errors instead of the schema, or a defaulted key counted missing | `options_problems_name_missing_keys_from_the_schema` |
| a value that doesn't fit counted as missing, or not named by its path | `options_problems_name_misfits_by_path` |
| decision ID, subject, extension, request number, event extension parsed wrong | `derived_names` |

## outbox and resends (`tests/outbox.rs`, through the engine)

| Failure | Test |
|---|---|
| a resend changes the body or the request ID | `a_resend_sends_the_same_bytes` |
| two sends share a request ID, or IDs aren't ULID-shaped | `request_ids_are_fresh_per_send` |
| the policy of a call differs from the table of `client.md` §2.1 | `the_resend_table` |
| an unknown write resent before 1 s, at the wrong steps, or more than 6 times | `the_resend_schedule` |
| an unknown write with the streams down is sent before a connection | `unknown_while_offline_waits_for_a_connection` |
| an entry of 11 minutes sent automatically, or one of 9 minutes not | `ten_minutes_ends_automatic_resends` |
| a restored entry sent automatically (FC12) | conformance `restored_entries_wait_for_retry` |
| `setModel`/`setOptions` restored can be retried | `restored_settings_cannot_be_retried` |
| an `answer`, `create` or `stop` resent by the outbox (FA6) | `answers_creates_and_stops_are_never_resent` |
| `␣rt` refused for a retryable entry, or sends twice | `retry_sends_once_now` |
| a done entry stays forever, or a refused one leaves by itself | `done_leaves_after_a_minute_refused_stays` |
| `x` sends something | `forget_sends_nothing` |
| a late reply of an earlier try settles the entry twice or wrongly | `a_late_reply_changes_nothing` |
| the outbox isn't saved after a change, or a sending entry restores as sending | `persisted_and_restored` |
| an offline input is refused instead of waiting | `offline_input_waits` |
| an offline write other than input is queued | `offline_writes_other_than_input_are_refused` |
| writes keep going after `unauthorized` | conformance `unauthorized_on_a_call_is_not_revoked` |

## streams and the link (`tests/streams.rs`, through the engine)

| Failure | Test |
|---|---|
| streams open before the handshake, or after another protocol | `the_handshake_comes_first`, `another_protocol_stops_everything` |
| a 45 s silence is not noticed, or noticed on a clock that stops in sleep | `silence_reopens_the_stream`; the clock itself by review: `SystemClock` reads `CLOCK_MONOTONIC` on macOS and `CLOCK_BOOTTIME` on Linux, which count sleep, and no test can put the machine to sleep |
| a heartbeat doesn't count as hearing from the stream | `heartbeats_keep_a_stream_alive` |
| backoff not 1, 2, 4, 8, 16, 30, 30 s, or not reset by a first frame | `backoff_steps_and_resets` |
| `end: stopping` not backed off from 1 s, or the line not `server stopped` | conformance `stopping_backs_off_from_one_second` |
| `end: revoked` reconnects, or anything else claims revoked | conformance `revoked_ends_everything`, `unauthorized_on_a_call_is_not_revoked` |
| `busy` on the inbox open not treated as stopping | `busy_inbox_is_a_stopping_server` |
| three `failed` ends in a row give no notice; other ends don't reset the count | `three_failures_in_a_row_are_noticed` |
| offline claimed before 60 s, or the band claims a state it never received | conformance `offline_after_sixty_seconds` |
| a frame from a closed or older attempt is applied | `a_stale_stream_changes_nothing` |
| a bad op or frame leaves a half-applied view in place | `a_bad_frame_reopens_the_stream` |
| a stream not wanted any more is not closed; a shown conversation not opened | `show_holds_one_stream_per_conversation` |
| a reopen sends no `after`, or the wrong one | `reopen_asks_after_the_newest_held` |
| `␣rc` waits for backoff | `reconnect_reopens_now` |

## the model (`tests/conformance.rs`)

| Failure | Test |
|---|---|
| before any frame the model reads empty instead of not received (FM13) | `nothing_received_is_not_calm` |
| inbox counts or order wrong | `inbox_first_frame` |
| a decision leaving the inbox gives no event, or says answered elsewhere for our own answer | `a_decision_answered_elsewhere_leaves`, answer tests |
| a superseded decision is not reported | `superseded_while_open` |
| a gap is not fetched, fetched twice, or fetched on a first open (FC5) | `gap_rule_fetches_once`, `gap_rule_skips_a_first_open` |
| entries duplicated after a gap fill | `gap_rule_fetches_once` |
| the window dropping entries drops them from the view | `entries_outlive_the_window` |
| `conversations` read one page only, or shown oldest first (FC14) | `conversations_read_every_page_newest_first` |
| `bad_cursor` stops paging instead of restarting | `bad_cursor_restarts_paging` |
| polls run while unfocused, or don't run at once on focus | `polls_follow_focus` |
| no poll 1 s after this device's input | `a_poll_follows_this_devices_writes` |
| scroll-back sends other bounds with the cursor, or pages below entry 1 | `scrolling_back_keeps_the_cursor_bounds` |
| a label is cached when none was found, asked more than once a minute, or read past 5 pages | `labels_are_found_once_and_cached` |
| a label taken from a window that doesn't start the conversation | `labels_come_from_the_start_only` |
| a gone object reads as not received | `gone_from_the_server` |

## start, answers (`tests/conformance.rs`)

| Failure | Test |
|---|---|
| `input` sent before `create`'s result, or to the wrong conversation | `start_creates_then_inputs` |
| a second `⌘⏎` makes a second `create` (FC3) | `one_press_one_create` |
| a lost `create` reply is resent or followed by an `input` | `a_lost_create_is_never_resent` |
| an earlier unknown `create` stays counted after a start that worked | `a_good_start_forgets_the_lost_create` |
| a refused `create` sends an `input`; its errors are lost | `a_refused_create_sends_no_input` |
| empty or over-1-MiB start text sends a `create` | `start_checks_the_text_first` |
| an unknown answer is not settled as landed when the server proves it (FA5) | `a_lost_answer_settles_as_landed` |
| a fresh inbox listing the decision does not mark the answer not recorded | `a_lost_answer_not_landed` |
| a second answer can be sent while one is unknown | `one_answer_at_a_time` |

## runtime (`tests/link.rs`, `tests/store.rs`)

| Failure | Test |
|---|---|
| a server message changes the model before `pump` (the UI's frame boundary) | `server_messages_wait_for_pump` |
| the waker is not called when something arrives | `the_waker_is_called` |
| the timer does not wake the UI at the engine's deadline | `the_timer_wakes_at_the_deadline` |
| the token lands in a file, a log or a URL (FC10) | `the_token_never_touches_disk` |
| `client.toml` or per-server state written somewhere other than the configured root | `state_lives_under_the_root` |
| `:forget-server` leaves the token or the server's state | `forget_server_deletes_everything` |
| a call lacks `Authorization`, `Content-Type` or a `Tau-Protocol` check | `http_calls_carry_their_headers` |
| a stream thread can't be closed promptly | `streams_close_within_a_second` |
| a URL with no server reads as a lost reply, so Connect can't say "No tau server answered" | `no_server_is_unreachable_so_nothing_was_sent` |
| the fixture transport answers with shapes other than the fixture's | `the_fixture_link_shows_the_design_data` |
| a decision watched before the handshake answers is never fetched, so a view opened at launch stays empty | `a_decision_watched_before_the_handshake_is_fetched_once_ready` |

## against a real server (`tests/e2e.rs`, `nix run .#e2e`)

One scenario, in order, on a tau server with the fake model `faux/echo` and the w2 extension. Each failure names
the step that catches it.

| Failure | Step |
|---|---|
| `pair` sends a token, or its reply's token isn't kept, so the handshake goes out without one | `pairs_and_connects` |
| the handshake or a stream reads a real reply as another protocol, or the link never reaches live | `pairs_and_connects` |
| a real catalogue reply fails to read (`models` without `faux/echo`, `extensions` without `w2`) | `pairs_and_connects` |
| `bind` succeeds but `projects` isn't read again, so the new project and its timeline are never seen | `binds_a_project` |
| `Start` sends `input` before `create` answers, or to another conversation | `starts_a_bare_conversation` |
| the `open` stream's real frames (`a` ops on the generation, entries appended) don't build the transcript, so the echo reply never shows | `starts_a_bare_conversation` |
| `stop` on a running conversation is not reflected as stopped, or `continue` after it is refused or not run | `stops_and_continues` |
| the inbox stream's real `s` ops on a new project and subject don't reach the model | `answers_a_w2_decision` |
| an `answer` built from a real `decision` (basis, choice) is refused, or its leaving the inbox is reported as answered elsewhere | `answers_a_w2_decision` |
| `tau device revoke` ends a stream and the link reconnects, or reads anything other than revoked | `revoke_ends_the_link` |
| a server stopping cleanly is read as a cut or a failure, not `end: stopping` | `a_stopping_server_is_retried` |
