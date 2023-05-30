//! Used to map server entities to client entities and client entities to server entities.

use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};

#[derive(Default, Resource)]
/// Used to map server entities to client entities and client entities to server entities.
pub struct NetworkMapping {
    server_to_client: HashMap<Entity, Entity>,
    client_to_server: HashMap<Entity, Entity>,
}

impl NetworkMapping {
    /// Adds a mapping between two entities
    pub fn add_mapping(&mut self, client_entity: Entity, server_entity: Entity) {
        self.server_to_client.insert(server_entity, client_entity);
        self.client_to_server.insert(client_entity, server_entity);
    }

    /// Checks if the server has a given entity
    pub fn contains_server_entity(&self, entity: Entity) -> bool {
        self.server_to_client.contains_key(&entity)
    }

    /// Gets the client entity based on the entity the server sent.
    ///
    /// Returns None if no such entity has been created on the client-side.
    pub fn client_from_server(&self, server_entity: &Entity) -> Option<Entity> {
        self.server_to_client.get(server_entity).copied()
    }

    /// Gets the server entity based on the entity the client has.
    ///
    /// Returns None if the client doesn't know about a server entity for that.
    pub fn server_from_client(&self, client_entity: &Entity) -> Option<Entity> {
        self.client_to_server.get(client_entity).copied()
    }

    /// Removes a mapping given the server's entity.
    pub fn remove_mapping_from_server_entity(&mut self, server_entity: &Entity) {
        if let Some(client_ent) = self.server_to_client.remove(server_entity) {
            self.client_to_server.remove(&client_ent);
        }
    }

    /// Removes a mapping given the client's entity.
    pub fn remove_mapping_from_client_entity(&mut self, client_entity: &Entity) {
        if let Some(server_ent) = self.client_to_server.remove(client_entity) {
            self.server_to_client.remove(&server_ent);
        }
    }
}
