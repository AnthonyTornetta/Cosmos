pub mod structure;
pub mod block;
pub mod utils;
pub mod entities;
pub mod physics;
pub mod plugin;
pub mod netty;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
