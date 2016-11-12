use tokio_core::reactor::Handle;
use minihttp::client::HttpClient;

use config::{ListenSocket, Handler, ConfigCell};
use handler::Main;
use chat::handler::ChatAPI;
use chat;
use minihttp;
use handlers;

pub struct State {
    chat: chat::Processor,
}


pub fn populate_loop(handle: &Handle, cfg: &ConfigCell, verbose: bool)
    -> State
{
    let chat_pro = chat::Processor::new();
    let main_handler = Main {
        config: cfg.clone(),
        handle: handle.clone(),
        http_client: HttpClient::new(handle.clone()),
        chat_processor: chat_pro.clone(),
    };
    // TODO(tailhook) do something when config updates
    for sock in &cfg.get().listen {
        match sock {
            &ListenSocket::Tcp(addr) => {
                if verbose {
                    println!("Listening at {}", addr);
                }
                minihttp::serve(handle, addr, main_handler.clone());
            }
        }
    }
    let root = cfg.get();
    for (name, h) in root.handlers.iter() {
        if let &Handler::SwindonChat(ref chat) = h {
            let sess = root.session_pools
                .get(&chat.session_pool).unwrap();
            match sess.listen {
                ListenSocket::Tcp(addr) => {
                    if verbose {
                        println!("Listening {} at {}", name, addr);
                    }
                    let chat_handler = ChatAPI {
                        config: cfg.clone(),
                        chat_pool: chat_pro.pool(&chat.session_pool),
                    };
                    minihttp::serve(handle, addr,
                        chat_handler.clone());
                }
            }
        }
    }
    handlers::files::update_pools(&cfg.get().disk_pools);
    chat_pro.update_pools(&cfg.get().session_pools);
    State {
        chat: chat_pro,
    }
}

pub fn update_loop(state: &State, cfg: &ConfigCell) {
    handlers::files::update_pools(&cfg.get().disk_pools);
    state.chat.update_pools(&cfg.get().session_pools);
}
