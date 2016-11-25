use std::sync::Arc;
use std::collections::HashMap;

use intern::Upstream;
use config::Config;
use config::chat::{Chat};
use config::Destination;
use config::http_destinations::Destination as Target;


#[derive(Clone)]
pub struct MessageRouter(pub Arc<Chat>, pub Arc<Config>);

impl MessageRouter {

    /// Builds target url for specified method.
    pub fn resolve(&self, method: &str) -> String {
        // TODO: optimize this method

        let dest = self.0.find_destination(method);
        // XXX: do not unwrap()
        url_for(method.replace(".", "/").as_str(), &dest,
            &self.1.http_destinations).unwrap()
    }

    // Predefined urls

    /// Tangle Authorization URL
    pub fn get_auth_url(&self) -> String {
        self.resolve("tangle.authorize_connection")
    }
}

pub fn url_for(path: &str, dest: &Destination,
    table: &HashMap<Upstream, Target>)
    -> Option<String>
{
    // XXX: We currently use first address of http_destination;
    //  also we dont resolve DNS, it lended to Curl.
    let result = table.get(&dest.upstream).and_then(|d| d.addresses.first());
    if let Some(addr) = result {
        let url = if dest.path.ends_with("/") {
            format!("http://{}{}{}", addr, dest.path, path)
        } else {
            format!("http://{}{}/{}", addr, dest.path, path)
        };
        Some(url)
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use config::Handler::SwindonChat;
    use config::test::make_config;

    use super::MessageRouter;

    #[test]
    fn match_route() {

        let cfg = make_config();
        let chat_cfg = match cfg.handlers.get(
            "example-chat".into()).unwrap() {
            &SwindonChat(ref cfg) => cfg.clone(),
            _ => panic!("Invalid config"),
        };

        let router = MessageRouter(chat_cfg, cfg);
        let result = router.resolve("some.method");
        assert_eq!(result,
            "http://example.com:5000/chat/some/method".to_string());

        let result = router.resolve("sub.chat");
        assert_eq!(result,
            "http://example.com:5000/sub/sub/chat".to_string());

        let result = router.resolve("sub.chat2");
        assert_eq!(result,
            "http://example.com:5000/chat/sub/chat2".to_string());

        let result = router.resolve("sub.chat.room1");
        assert_eq!(result,
            "http://example.com:5000/sub_chat/sub/chat/room1".to_string());

        let result = router.resolve("other.method");
        assert_eq!(result,
            "http://example.com:5000/other/method".to_string());
    }
}
