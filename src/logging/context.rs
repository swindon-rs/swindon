pub use trimmer::Context;

pub trait AsContext {
    fn as_context(&self) -> Context;
}
