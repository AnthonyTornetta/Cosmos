#![feature(get_many_mut)]

pub mod block;
pub mod blockitems;
pub mod entities;
pub mod events;
pub mod inventory;
pub mod item;
pub mod loader;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod registry;
pub mod structure;
pub mod utils;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
