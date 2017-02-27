use quire::validate::{Structure, Scalar};


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct BaseRedirect {
    pub redirect_to_domain: String,
}


pub fn base_redirect<'x>() -> Structure<'x> {
    Structure::new()
    .member("redirect_to_domain", Scalar::new())
}
