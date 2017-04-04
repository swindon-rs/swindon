use std::io;


quick_error! {
    #[derive(Debug)]
    pub enum FileError {
        Sendfile(err: io::Error) {
            description("sendfile error")
            cause(err)
        }
    }
}
