pub struct Item {
    unlocalized_name: String,
    numeric_id: u16,
}

impl Item {
    pub fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    pub fn set_numeric_id(&mut self, id: u16) {
        self.numeric_id = id;
    }

    pub fn numeric_id(&self) -> u16 {
        self.numeric_id
    }
}
