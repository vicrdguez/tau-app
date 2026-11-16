//! Writes that never left the device.
use tau_client::outbox::NotSent;

pub fn project_not_sent(reason: &NotSent, start: bool) -> Option<&'static str> {
    Some(match reason {
        NotSent::Empty | NotSent::Starting => return None,
        NotSent::TooLarge => "Over 1 MiB · nothing was sent",
        NotSent::Offline if start => "offline · start when connected",
        NotSent::Offline => "offline · save when connected",
        NotSent::Refused => "The server refused this device; nothing was sent",
        NotSent::Revoked => "This device was revoked; nothing was sent",
        NotSent::NotPaired => "Connect to a server first",
        NotSent::OtherProtocol => "This server speaks another protocol; nothing was sent",
        _ => "Nothing was sent",
    })
}
