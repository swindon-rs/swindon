use std::sync::Arc;

use config::Config;
use config::chat::Chat;


#[derive(Clone)]
pub struct MessageRouter(pub Arc<Chat>, pub Arc<Config>);

impl MessageRouter {

    /// Builds target url for specified method.
    pub fn resolve(&self, method: &str) -> String {
        // TODO: optimize this method

        let default = self.0.message_handlers.get("*".into()).unwrap();

        let dest = self.0.message_handlers.iter()
        .rev()
        .find(|&(k, _)| {
            let p = Pattern::from_string(k);
            !p.is_default() && p.matches(method)
        })
        .map(|(_, v)| v)
        .unwrap_or(default);

        let target = self.1.http_destinations.get(&dest.upstream).unwrap();
        let addr = target.addresses.first().unwrap();
        if dest.path.ends_with("/") {
            format!("http://{}{}{}",
                addr, dest.path, method.replace(".", "/"))
        } else {
            format!("http://{}{}/{}",
                addr, dest.path, method.replace(".", "/"))
        }
    }

    // Predefined urls

    /// Tangle Authorization URL
    pub fn get_auth_url(&self) -> String {
        self.resolve("tangle.authorize_connection")
    }
}


#[derive(Debug)]
pub enum Pattern<'a> {
    Default,
    Glob(&'a str),
    Exact(&'a str),
}


impl<'a> Pattern<'a> {
    pub fn from_string(s: &'a String) -> Pattern<'a> {
        if s.as_str() == "*" {
            Pattern::Default
        } else if s.ends_with(".*") {
            let (p, _) = s.split_at(s.len()-1);
            Pattern::Glob(p)
        } else {
            Pattern::Exact(s.as_str())
        }
    }

    pub fn matches(&self, other: &str) -> bool {
        match self {
            &Pattern::Default => true,
            &Pattern::Glob(s) => other.starts_with(s) && other.len() > s.len(),
            &Pattern::Exact(s) => other == s,
        }
    }

    pub fn is_default(&self) -> bool {
        match self {
            &Pattern::Default => true,
            _ => false,
        }
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
