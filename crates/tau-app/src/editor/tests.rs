//! How the editor could fail its writer, one test (or a few) each:
//! - a motion leaves its line, passes the last character in Normal, loses the column over a short
//!   line, or splits a multibyte character;
//! - `w b e` misread words, punctuation, spaces or line ends;
//! - `0 ^ $ gg G` land in the wrong place, or a count is ignored;
//! - `x dd D C` delete the wrong span, leave the cursor off the text, or forget the register;
//! - `dw` eats the line break, `cw` eats the space after the word, `iw aw` take the wrong span;
//! - `yy p P` put lines or words in the wrong place;
//! - `u` undoes half an operator, or `C-r` fails to redo it;
//! - `i a I A o O` start Insert in the wrong place, or `o O` work on a single line;
//! - Visual selects one character short, or `V` misses a line;
//! - `Esc` leaves the caret past the line's end;
//! - `jk` leaves its `j` behind, fires when the `k` comes late or after another key, or fires
//!   while an input method composes;
//! - `C-w C-u` delete too much or too little;
//! - `⏎` breaks a single line, or loses a list item's marker;
//! - an operator stops waiting, or `Esc` fails to cancel it.

use rune::{Ime, KeyEvent};

use super::*;
use crate::keys::{Code, parse_seq};

/// Text with `|` at the cursor.
fn at(ed: &Editor) -> String {
    let mut s = ed.text().to_string();
    s.insert(ed.cursor(), '|');
    s
}

/// An editor over `marked`, whose `|` places the cursor.
fn normal(marked: &str) -> Editor {
    let pos = marked.find('|').expect("a | marks the cursor");
    let text = marked.replacen('|', "", 1);
    let mut ed = Editor::new(&text, Lines::Multi);
    ed.edit.set_caret(pos);
    ed
}

/// Feeds keys as the host does: Normal and Visual keys to `normal`, Insert keys to `insert`,
/// and what Insert passes on to the field, as rune's field would type it.
fn keys(ed: &mut Editor, seq: &str) {
    keys_at(ed, seq, 0.0, 1.0);
}

fn keys_at(ed: &mut Editor, seq: &str, start: f64, step: f64) {
    let mut now = start;
    for key in parse_seq(seq).expect("keys") {
        if ed.mode() == EditMode::Insert {
            if ed.insert(key, now) == Reply::Pass {
                type_into_field(ed, key);
            }
        } else {
            ed.normal(None, key);
        }
        now += step;
    }
}

fn type_into_field(ed: &mut Editor, key: Key) {
    let rk = match key.code {
        Code::Char(c) => rune::Key::Text(c.to_string()),
        Code::Backspace => rune::Key::Backspace,
        Code::Enter => rune::Key::Enter,
        _ => return,
    };
    ed.edit.key(
        &KeyEvent {
            key: rk,
            mods: rune::Mods::default(),
        },
        None,
    );
}

// ───────────── motions ─────────────

#[test]
fn h_and_l_stay_on_the_line_and_off_its_end() {
    let mut ed = normal("ab|c\ndef");
    keys(&mut ed, "l");
    assert_eq!(at(&ed), "ab|c\ndef", "l stops on the last character");
    keys(&mut ed, "hhh");
    assert_eq!(at(&ed), "|abc\ndef", "h stops at the line's start");
}

#[test]
fn j_and_k_keep_the_column_over_a_short_line() {
    let mut ed = normal("abcd|ef\nxy\nabcdefg");
    keys(&mut ed, "j");
    assert_eq!(at(&ed), "abcdef\nx|y\nabcdefg");
    keys(&mut ed, "j");
    assert_eq!(at(&ed), "abcdef\nxy\nabcd|efg");
    keys(&mut ed, "kk");
    assert_eq!(at(&ed), "abcd|ef\nxy\nabcdefg");
}

#[test]
fn w_b_e_move_by_words_and_punctuation() {
    let mut ed = normal("|foo.bar  baz\nqux");
    keys(&mut ed, "w");
    assert_eq!(at(&ed), "foo|.bar  baz\nqux");
    keys(&mut ed, "w");
    assert_eq!(at(&ed), "foo.|bar  baz\nqux");
    keys(&mut ed, "w");
    assert_eq!(at(&ed), "foo.bar  |baz\nqux");
    keys(&mut ed, "w");
    assert_eq!(at(&ed), "foo.bar  baz\n|qux", "w crosses the line break");
    keys(&mut ed, "b");
    assert_eq!(at(&ed), "foo.bar  |baz\nqux");
    keys(&mut ed, "bb");
    assert_eq!(at(&ed), "foo|.bar  baz\nqux");
    keys(&mut ed, "e");
    assert_eq!(at(&ed), "foo.ba|r  baz\nqux");
    keys(&mut ed, "e");
    assert_eq!(at(&ed), "foo.bar  ba|z\nqux");
}

#[test]
fn zero_caret_and_dollar_find_the_lines_edges() {
    let mut ed = normal("x\n   ab|cd\ny");
    keys(&mut ed, "0");
    assert_eq!(at(&ed), "x\n|   abcd\ny");
    keys(&mut ed, "^");
    assert_eq!(at(&ed), "x\n   |abcd\ny");
    keys(&mut ed, "$");
    assert_eq!(at(&ed), "x\n   abc|d\ny");
}

#[test]
fn gg_and_g_go_to_the_first_and_last_lines() {
    let mut ed = normal("one\n  two\nthr|ee");
    keys(&mut ed, "gg");
    assert_eq!(at(&ed), "|one\n  two\nthree");
    keys(&mut ed, "G");
    assert_eq!(at(&ed), "one\n  two\n|three");
    keys(&mut ed, "2G");
    assert_eq!(at(&ed), "one\n  |two\nthree", "a count names the line");
}

#[test]
fn a_count_repeats_a_motion() {
    let mut ed = normal("|a b c d e");
    keys(&mut ed, "3w");
    assert_eq!(at(&ed), "a b c |d e");
    let mut ed = normal("|a b c d e");
    assert_eq!(ed.normal(Some(2), Key::char('w')), Reply::Handled);
    assert_eq!(at(&ed), "a b |c d e", "the engine's count");
}

#[test]
fn motions_step_over_multibyte_characters_whole() {
    let mut ed = normal("|été");
    keys(&mut ed, "l");
    assert_eq!(at(&ed), "é|té");
    keys(&mut ed, "x");
    assert_eq!(at(&ed), "é|é");
}

// ───────────── operators ─────────────

#[test]
fn x_deletes_under_the_cursor_and_stays_on_the_text() {
    let mut ed = normal("ab|c");
    keys(&mut ed, "x");
    assert_eq!(at(&ed), "a|b");
    let mut ed = normal("|abcd");
    keys(&mut ed, "2x");
    assert_eq!(at(&ed), "|cd");
    keys(&mut ed, "P");
    assert_eq!(at(&ed), "a|bcd", "x fills the register");
}

#[test]
fn dd_deletes_whole_lines_and_lands_on_the_next() {
    let mut ed = normal("one\nt|wo\n  three");
    keys(&mut ed, "dd");
    assert_eq!(at(&ed), "one\n  |three");
    keys(&mut ed, "dd");
    assert_eq!(
        at(&ed),
        "|one",
        "deleting the last line lands on the one before"
    );
    let mut ed = normal("|a\nb\nc\nd");
    keys(&mut ed, "2dd");
    assert_eq!(at(&ed), "|c\nd");
}

#[test]
fn d_and_c_capitals_take_the_rest_of_the_line() {
    let mut ed = normal("abc|def\nx");
    keys(&mut ed, "D");
    assert_eq!(at(&ed), "ab|c\nx");
    let mut ed = normal("abc|def\nx");
    keys(&mut ed, "C");
    assert_eq!(
        (at(&ed), ed.mode()),
        ("abc|\nx".to_string(), EditMode::Insert)
    );
}

#[test]
fn dw_keeps_the_line_break() {
    let mut ed = normal("one |two\nthree");
    keys(&mut ed, "dw");
    assert_eq!(
        at(&ed),
        "one| \nthree",
        "dw on the last word stops at the line's end"
    );
}

#[test]
fn dw_takes_the_space_after_the_word() {
    let mut ed = normal("|one two");
    keys(&mut ed, "dw");
    assert_eq!(at(&ed), "|two");
}

#[test]
fn cw_changes_to_the_words_end_only() {
    let mut ed = normal("|one two");
    keys(&mut ed, "cw");
    assert_eq!(
        (at(&ed), ed.mode()),
        ("| two".to_string(), EditMode::Insert)
    );
}

#[test]
fn iw_and_aw_take_the_word_and_its_space() {
    let mut ed = normal("one t|wo three");
    keys(&mut ed, "diw");
    assert_eq!(at(&ed), "one | three");
    let mut ed = normal("one t|wo three");
    keys(&mut ed, "daw");
    assert_eq!(at(&ed), "one |three");
    let mut ed = normal("one t|wo three");
    keys(&mut ed, "ciwX<Esc>");
    assert_eq!(at(&ed), "one |X three");
}

#[test]
fn yy_and_p_put_lines_below_and_above() {
    let mut ed = normal("o|ne\ntwo");
    keys(&mut ed, "yyjp");
    assert_eq!(at(&ed), "one\ntwo\n|one");
    keys(&mut ed, "ggP");
    assert_eq!(at(&ed), "|one\none\ntwo\none");
}

#[test]
fn a_yanked_word_puts_after_the_cursor() {
    let mut ed = normal("|ab cd");
    keys(&mut ed, "yw$p");
    assert_eq!(at(&ed), "ab cdab| ");
}

#[test]
fn u_undoes_a_whole_operator_and_ctrl_r_redoes_it() {
    let mut ed = normal("one t|wo three");
    keys(&mut ed, "daw");
    assert_eq!(ed.text(), "one three");
    keys(&mut ed, "u");
    assert_eq!(ed.text(), "one two three");
    keys(&mut ed, "<C-r>");
    assert_eq!(ed.text(), "one three");
}

#[test]
fn an_insert_session_undoes_as_one() {
    let mut ed = normal("|x");
    keys(&mut ed, "ihello <Esc>");
    assert_eq!(ed.text(), "hello x");
    keys(&mut ed, "u");
    assert_eq!(ed.text(), "x");
}

#[test]
fn an_operator_waits_and_esc_cancels_it() {
    let mut ed = normal("|one two");
    keys(&mut ed, "d");
    assert!(ed.is_pending());
    keys(&mut ed, "<Esc>");
    assert!(!ed.is_pending());
    keys(&mut ed, "w");
    assert_eq!(at(&ed), "one |two", "w moves again once d is cancelled");
    keys(&mut ed, "d2w");
    assert_eq!(ed.text(), "one ", "a count inside the operator");
}

// ───────────── entering and leaving Insert ─────────────

#[test]
fn i_a_capital_i_and_a_start_insert_where_vim_does() {
    for (k, want) in [
        ("i", "  ab|cd"),
        ("a", "  abc|d"),
        ("I", "  |abcd"),
        ("A", "  abcd|"),
    ] {
        let mut ed = normal("  ab|cd");
        keys(&mut ed, k);
        assert_eq!(
            (at(&ed), ed.mode()),
            (want.to_string(), EditMode::Insert),
            "{k}"
        );
    }
}

#[test]
fn o_and_capital_o_open_lines() {
    let mut ed = normal("a|b\ncd");
    keys(&mut ed, "o");
    assert_eq!(
        (at(&ed), ed.mode()),
        ("ab\n|\ncd".to_string(), EditMode::Insert)
    );
    let mut ed = normal("ab\nc|d");
    keys(&mut ed, "O");
    assert_eq!(at(&ed), "ab\n|\ncd");
}

#[test]
fn esc_leaves_insert_one_character_back() {
    let mut ed = Editor::inserting("abc", Lines::Multi);
    assert_eq!((at(&ed), ed.mode()), ("abc|".to_string(), EditMode::Insert));
    keys(&mut ed, "<Esc>");
    assert_eq!((at(&ed), ed.mode()), ("ab|c".to_string(), EditMode::Normal));
    let mut ed = normal("|abc");
    keys(&mut ed, "i<Esc>");
    assert_eq!(at(&ed), "|abc", "not past the line's start");
}

#[test]
fn jk_in_time_leaves_insert_without_its_j() {
    let mut ed = Editor::inserting("ab", Lines::Multi);
    keys_at(&mut ed, "jk", 1.0, 0.1);
    assert_eq!((ed.text(), ed.mode()), ("ab", EditMode::Normal));
    keys(&mut ed, "u");
    assert_eq!(
        ed.text(),
        "ab",
        "the j and its removal leave no undo step behind"
    );
}

#[test]
fn a_held_j_appears_once_its_time_is_up() {
    let mut ed = Editor::inserting("", Lines::Multi);
    keys_at(&mut ed, "j", 1.0, 0.0);
    assert!(!ed.tick(1.0 + JK_SECONDS - 0.01));
    assert!(ed.tick(1.0 + JK_SECONDS + 0.01));
    assert_eq!((ed.text(), ed.mode()), ("j", EditMode::Insert));
}

#[test]
fn j_then_esc_types_the_j_and_leaves() {
    let mut ed = Editor::inserting("", Lines::Multi);
    keys_at(&mut ed, "j<Esc>", 1.0, 0.05);
    assert_eq!((ed.text(), ed.mode()), ("j", EditMode::Normal));
}

#[test]
fn jk_too_slow_types_both() {
    let mut ed = Editor::inserting("", Lines::Multi);
    keys_at(&mut ed, "jk", 1.0, JK_SECONDS + 0.05);
    assert_eq!((ed.text(), ed.mode()), ("jk", EditMode::Insert));
}

#[test]
fn j_then_another_key_then_k_types_all() {
    let mut ed = Editor::inserting("", Lines::Multi);
    keys_at(&mut ed, "jxk", 1.0, 0.05);
    assert_eq!((ed.text(), ed.mode()), ("jxk", EditMode::Insert));
}

#[test]
fn jk_is_off_while_an_input_method_composes() {
    let mut ed = Editor::inserting("", Lines::Multi);
    keys_at(&mut ed, "j", 1.0, 0.05);
    ed.edit.ime(&Ime::Preedit("か".into(), None, Vec::new()));
    assert_eq!(
        ed.insert(Key::char('k'), 1.05),
        Reply::Pass,
        "the input method takes the k"
    );
    assert_eq!(ed.mode(), EditMode::Insert);
}

#[test]
fn ctrl_w_and_ctrl_u_delete_back_by_word_and_line() {
    let mut ed = Editor::inserting("x\nfoo bar baz", Lines::Multi);
    keys(&mut ed, "<C-w>");
    assert_eq!(ed.text(), "x\nfoo bar ");
    keys(&mut ed, "<C-w>");
    assert_eq!(ed.text(), "x\nfoo ");
    keys(&mut ed, "<C-u>");
    assert_eq!((at(&ed), ed.mode()), ("x\n|".to_string(), EditMode::Insert));
}

// ───────────── Visual ─────────────

#[test]
fn v_selects_through_the_cursor_and_d_deletes_it() {
    let mut ed = normal("a|bcde");
    keys(&mut ed, "vl");
    assert_eq!(ed.selection(), Some(1..3));
    keys(&mut ed, "d");
    assert_eq!((at(&ed), ed.mode()), ("a|de".to_string(), EditMode::Normal));
}

#[test]
fn capital_v_takes_whole_lines() {
    let mut ed = normal("one\nt|wo\nthree\nfour");
    keys(&mut ed, "Vj");
    assert_eq!(ed.mode(), EditMode::VisualLine);
    keys(&mut ed, "d");
    assert_eq!(at(&ed), "one\n|four");
}

#[test]
fn visual_y_yanks_and_esc_leaves() {
    let mut ed = normal("|abc");
    keys(&mut ed, "vly");
    assert_eq!((at(&ed), ed.mode()), ("|abc".to_string(), EditMode::Normal));
    keys(&mut ed, "$p");
    assert_eq!(ed.text(), "abcab");
    keys(&mut ed, "v<Esc>");
    assert_eq!((ed.mode(), ed.selection()), (EditMode::Normal, None));
}

// ───────────── lines ─────────────

#[test]
fn a_new_line_continues_a_list_item() {
    let mut ed = Editor::inserting("- one", Lines::Multi);
    ed.newline();
    assert_eq!(at(&ed), "- one\n- |");
    let mut ed = Editor::inserting("3. three", Lines::Multi);
    ed.newline();
    assert_eq!(at(&ed), "3. three\n4. |");
    let mut ed = Editor::inserting("plain", Lines::Multi);
    ed.newline();
    assert_eq!(at(&ed), "plain\n|");
}

#[test]
fn a_new_line_on_an_empty_list_item_ends_the_list() {
    let mut ed = Editor::inserting("- one\n- ", Lines::Multi);
    ed.newline();
    assert_eq!(at(&ed), "- one\n|");
}

#[test]
fn a_single_line_never_breaks() {
    let mut ed = Editor::inserting("abc", Lines::Single);
    ed.newline();
    assert_eq!(ed.text(), "abc");
    let mut ed = Editor::new("abc", Lines::Single);
    keys(&mut ed, "o");
    assert_eq!(
        (ed.text(), ed.mode()),
        ("abc", EditMode::Normal),
        "o opens no line"
    );
    keys(&mut ed, "j");
    assert_eq!(ed.cursor(), 0);
    keys(&mut ed, "dd");
    assert_eq!(ed.text(), "", "dd clears the one line");
}

#[test]
fn normal_mode_draws_a_block_on_the_character() {
    let ed = normal("a|bc");
    assert_eq!(ed.block(), Some(1..2));
    let ed = Editor::inserting("abc", Lines::Multi);
    assert_eq!(ed.block(), None);
}
