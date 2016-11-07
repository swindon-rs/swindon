use quire::validate::{Structure, Numeric};

#[derive(RustcDecodable, Debug, PartialEq, Eq, Hash)]
pub struct Disk {
    pub num_threads: usize,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("num_threads", Numeric::new().min(1))
}
