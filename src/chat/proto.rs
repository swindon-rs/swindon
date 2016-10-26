//! Chat protocol.
use rustc_serialize::json::Json;

use tokio_core::reactor::{Handle, Timeout};

use websocket::{Dispatcher, Frame, ImmediateReplier, RemoteReplier, Error};


pub struct Chat(pub Handle);

impl Dispatcher for Chat {

    fn dispatch(&mut self, frame: Frame,
        replier: &mut ImmediateReplier, remote: &RemoteReplier)
        -> Result<(), Error>
    {
        // spawn call to backend with passthrough to ws;

        let message = match frame {
            Frame::Text(data) => {
                let obj = Json::from_str(data);
                match obj {
                    Ok(Json::Array(mut message)) => {
                        if message.len() != 4 {
                            return Ok(())
                        }
                        (
                            message.pop().unwrap(),
                            message.pop().unwrap(),
                            message.pop().unwrap(),
                            message.pop().unwrap(),
                        )
                    }
                    _ => return Ok(()) // TODO: close connection;
                }
            }
            _ => return Ok(()),
        };

        match message {
            // order reversed
            (Json::Object(_kwargs), Json::Array(_args),
             Json::Object(_meta), Json::String(method),
            ) => {
                if method.starts_with("tangle.") {
                    return Ok(())
                }
                if method.find("/").is_some() {
                    return Ok(())
                }
                replier.text(format!("Parsed: {:?}", method).as_str());
            }
            _ => return Ok(())
        }
        Ok(())
    }
}
