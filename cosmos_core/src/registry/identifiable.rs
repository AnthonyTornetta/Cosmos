//! If something is identifiable, it has a numeric id used internally
//! & a string id used globally (unlocalized name)
//!
//! The numeric id will be set automatically once this is registered, so just set it to
//! some dummy value like 0. The unlocalized name has to be set by you.

/// Represents something that has an internally used numeric id & a globally used unlocalized name.
pub trait Identifiable: Send + Sync {
    /// Returns the internally used id
    ///
    /// Make sure the value this returns is the same as the value set by set_numeric_id.
    fn id(&self) -> u16;

    /// Returns the reference to the globally used unlocalized name.
    /// This should generally be formatted as "mod_id:name_of_thing".
    ///
    /// For example: `cosmos:laser_cannon`
    fn unlocalized_name(&self) -> &str;

    /// Only use this if you know what you're doing.  Should really only be used by the various registries
    ///
    /// Don't set the numeric id manually, simply set it to some dummy value at the start and once this
    /// is registered, [`set_numeric_id`] will be called for you.
    fn set_numeric_id(&mut self, id: u16);
}
