pub trait Identifiable {
    fn id(&self) -> u16;

    fn unlocalized_name(&self) -> &str;

    fn set_numeric_id(&mut self, id: u16);
}
