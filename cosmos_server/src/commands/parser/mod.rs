use crate::commands::prelude::ArgumentError;

pub trait CommandParser<'a, T> {
    fn parse(&self) -> Result<(T, &'a [String]), ArgumentError>;
}

pub mod location_parser;
