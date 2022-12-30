pub trait Identifiable {
    fn id(&self) -> u16;

    fn unlocalized_name(&self) -> &str;

    /// Only use this if you know what you're doing.  Should really only be used in the Registry struct
    fn set_numeric_id(&mut self, id: u16);
}
