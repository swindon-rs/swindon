use trimmer::{Parser};


lazy_static! {
    /// This holds parser so we don't need to compile it's comlplex regexes
    /// every time
    pub static ref PARSER: Parser = Parser::new();
}
