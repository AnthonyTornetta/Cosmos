use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};

#[derive(Default, Resource)]
pub struct NetworkMapping {
    server_to_client: HashMap<Entity, Entity>,
    client_to_server: HashMap<Entity, Entity>,
}

impl NetworkMapping {
    pub fn add_mapping(&mut self, client_entity: &Entity, server_entity: &Entity) {
        self.server_to_client
            .insert(server_entity.clone(), client_entity.clone());
        self.client_to_server
            .insert(client_entity.clone(), server_entity.clone());
    }

    pub fn client_from_server(&self, server_entity: &Entity) -> Option<&Entity> {
        self.server_to_client.get(server_entity)
    }

    pub fn server_from_client(&self, client_entity: &Entity) -> Option<&Entity> {
        self.client_to_server.get(client_entity)
    }

    pub fn remove_mapping_from_server_entity(&mut self, server_entity: &Entity) {
        let client_ent = self.server_to_client.remove(server_entity);
        if client_ent.is_some() {
            self.client_to_server.remove(&client_ent.unwrap());
        }
    }
}
