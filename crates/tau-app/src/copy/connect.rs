//! Connect failures. A transport error alone cannot prove that pair did not land.

use super::refusals::{HOST_ADVICE, HOST_MESSAGE};
use tau_client::{
    engine::PairFailure,
    wire::{Refusal, RefusalCode, Unknown},
};

/// What Connect says when `pair` to `url` failed. A connection that could not be made sent
/// nothing: no tau server answered. A lost reply names the chosen device, not the current paired
/// device. With no chosen name, Connect says "the name chosen on the server" and uses
/// `<old name>` in the revoke command.
pub fn pair_failure(failure: &PairFailure, url: &str, chosen_name: Option<&str>) -> String {
    match failure {
        PairFailure::Refused(r) => pair_refusal(r),
        PairFailure::NoReply(Unknown::Unreachable(error)) => no_server(url, error),
        PairFailure::NoReply(Unknown::OtherProtocol(n)) => {
            format!("This server speaks protocol {n}; this client speaks 1.")
        }
        PairFailure::NoReply(_) => {
            let name = chosen_name.unwrap_or("chosen on the server");
            let old = chosen_name.unwrap_or("<old name>");
            format!(
                "No reply from the server. If the code was accepted, the name {name} is now used and this device never got its token. Run tau device pair <new name> on the server, pair with the new code, then revoke the old name: tau device revoke {old}."
            )
        }
    }
}

/// For a connection failure known to precede sending pair. Do not use this for
/// an ambiguous transport failure after sending. The error is quoted unchanged, with a full stop
/// after it unless it ends a sentence already.
pub fn no_server(url: &str, error: &str) -> String {
    let separator = if error.ends_with(['.', '!', '?']) {
        " "
    } else {
        ". "
    };
    format!(
        "No tau server answered at {url}: {error}{separator}Check the address, and that tau is running there."
    )
}

pub(super) fn pair_refusal(refused: &Refusal) -> String {
    if refused.code == RefusalCode::Unauthorized {
        if refused.message == HOST_MESSAGE {
            format!("{} {HOST_ADVICE}", refused.message)
        } else {
            format!(
                "{} A code works once, for 10 minutes, and five wrong tries cancel every pending code. Run tau device pair <name> again.",
                refused.message
            )
        }
    } else {
        format!("Not done: {}", refused.message)
    }
}
