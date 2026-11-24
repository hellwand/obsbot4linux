// SPDX-License-Identifier: GPL-3.0-or-later

//! The client library in a few lines (design spec 6.1, `docs/protocol-2.md` section 12): list
//! the devices once with the blocking client, then print every state event of a subscribed
//! connection. It sends no write. Run it with `cargo run -p obsbot-proto --example watch`.

use obsbot_proto::client::{Client, ClientError, Incoming};
use obsbot_proto::{Event, Hello, short_id};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A blocking client: one request at a time, no events (subscribe: false).
    let mut client = Client::connect(Hello::new("example 1.0"))?;
    for device in client.devices().to_vec() {
        // `{:?}` quotes the short id and escapes any control character it holds: an id comes
        // from the camera, and what a terminal prints is not for the camera to choose.
        println!(
            "{:?}  {}  {:?}",
            short_id(&device.id),
            device.model,
            device.support
        );
        if device.available && !device.description.settings.is_empty() {
            match client.get(&device.id, Some(&["zoom.factor"])) {
                Ok((rev, state)) => println!("  rev {rev}: zoom {:?}", state.at("zoom.factor")),
                // A model without the leaf refuses with a code; that device ends nothing.
                Err(ClientError::Refused(body)) => {
                    println!("  zoom.factor: {}", body.code.as_str())
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    // A subscribed client: the hello reply, a sender and a receiver (15.1 row 96).
    let (hello, _sender, receiver) = Client::subscribe(Hello::new("example 1.0"))?;
    println!("daemon {} speaks protocol {}", hello.daemon, hello.protocol);
    for incoming in receiver {
        if let Incoming::Event(Event::State { device, rev, state }) = incoming? {
            println!(
                "{:?} rev {rev}: pending {:?}",
                short_id(&device),
                state.pending
            );
        }
    }
    Ok(())
}
