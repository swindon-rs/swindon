use std::sync::{Arc, RwLock};

use tokio_core::reactor::Handle;
use futures::Stream;
use futures::sync::mpsc::{unbounded as channel};
use minihttp::client::HttpClient;

use config::{ListenSocket, Handler, ConfigCell};
use handler::Main;
use chat::{ChatBackend, Processor, handle_pool_message};
use minihttp;
use handlers;

pub struct State {
    chat: Arc<RwLock<Processor>>,
}


pub fn populate_loop(handle: &Handle, cfg: &ConfigCell, verbose: bool)
    -> State
{
    let chat_pro = Arc::new(RwLock::new(Processor::new()));
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
                let main_handler = main_handler.clone();
                minihttp::serve(handle, addr,
                    move || Ok(main_handler.clone()));
            }
        }
    }
    for (name, cfg) in &cfg.get().session_pools {
        let (tx, rx) = channel();
        chat_pro.write().unwrap().create_pool(name, cfg, tx);
        handle.spawn(rx.for_each(move |msg| {
            handle_pool_message(msg);
            Ok(())
        }))
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
                    let chat_handler = ChatBackend {
                        config: cfg.clone(),
                        chat_pool: chat_pro.read().unwrap().pool(
                            &chat.session_pool),
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
    let mut chat_pro = state.chat.write().unwrap();
    for (name, cfg) in &cfg.get().session_pools {
        if !chat_pro.has_pool(name) {
            let (tx, rx) = channel();
            chat_pro.create_pool(name, cfg, tx);
            handle.spawn(rx.for_each(move |msg| {
                handle_pool_message(msg);
                Ok(())
            }));
        }
    }
    // TODO(tailhook) update chat handlers
}
