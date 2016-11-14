use tokio_core::reactor::Handle;
use tokio_core::channel::channel;
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
    let mut chat_pro = chat::Processor::new();
    let main_handler = Main {
        config: cfg.clone(),
        handle: handle.clone(),
        http_client: HttpClient::new(handle.clone()),
    };
    // TODO(tailhook) do something when config updates
    for sock in &cfg.get().listen {
        match sock {
            &ListenSocket::Tcp(addr) => {
                if verbose {
                    println!("Listening at {}", addr);
                }
                let main_handler = main_handler.clone();
                minihttp::serve(handle, addr,
                    move || Ok(main_handler.clone()));
            }
        }
    }
    for (name, cfg) in &cfg.get().session_pools {
        let (tx, rx) = channel(handle).expect("create channel");
        chat_pro.create_pool(name, cfg, tx);
        // TODO(tailhook) read from rx
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
                        move || Ok(chat_handler.clone()));
                }
            }
        }
    }
    handlers::files::update_pools(&cfg.get().disk_pools);
    State {
        chat: chat_pro,
    }
}

pub fn update_loop(state: &mut State, cfg: &ConfigCell, handle: &Handle) {
    // TODO(tailhook) update listening sockets
    handlers::files::update_pools(&cfg.get().disk_pools);
    for (name, cfg) in &cfg.get().session_pools {
        if !state.chat.has_pool(name) {
            let (tx, rx) = channel(&handle).expect("create channel");
            state.chat.create_pool(name, cfg, tx);
            // TODO(tailhook) read from rx
        }
    }
    // TODO(tailhook) update chat handlers
}
