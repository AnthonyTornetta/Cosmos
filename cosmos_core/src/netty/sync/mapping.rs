//! Used to map server entities to client entities and client entities to server entities.
//!
//! The resources in this file are **exclusively** used on the client-side. They are only
//! in the core project for specific client-only use cases.

use crate::{
    netty::netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
    prelude::StructureBlock,
};
use bevy::{
    ecs::component::Component,
    prelude::{Entity, Resource},
    reflect::Reflect,
    utils::HashMap,
};

/// This is the server entity that refers to this entity.
///
/// This is mostly for debugging purposes.
#[derive(Component, Reflect, PartialEq, Eq)]
pub struct ServerEntity(pub Entity);

#[derive(Default, Resource, Debug)]
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

#[derive(Debug)]
/// This error is returned if there is an issue extracting the proper entity from the network mapping provided.
pub enum MappingError<T> {
    /// This error is returned if the proper entity is missing from the network mapping provided.
    MissingRecord(T),
}

/// Used to convert structs with server entities into structs with client entities
pub trait Mappable {
    /// Converts all instances of server entities into their respective client entities based off the mapping.
    ///
    /// Returns Err if it is unable to find the proper mapping
    fn map_to_client(self, network_mapping: &NetworkMapping) -> Result<Self, MappingError<Self>>
    where
        Self: Sized;

    /// Converts all instances of server entities into their respective server entities based off the mapping.
    ///
    /// Returns Err if it is unable to find the proper mapping
    fn map_to_server(self, network_mapping: &NetworkMapping) -> Result<Self, MappingError<Self>>
    where
        Self: Sized;
}

impl Mappable for NettyRigidBody {
    fn map_to_client(self, network_mapping: &NetworkMapping) -> Result<Self, MappingError<Self>> {
        match self.location {
            NettyRigidBodyLocation::Relative(rel_pos, parent_ent) => {
                let Some(client_ent) = network_mapping.client_from_server(&parent_ent) else {
                    return Err(MappingError::MissingRecord(self));
                };

                Ok(NettyRigidBody {
                    body_vel: self.body_vel,
                    location: NettyRigidBodyLocation::Relative(rel_pos, client_ent),
                    rotation: self.rotation,
                })
            }
            NettyRigidBodyLocation::Absolute(_) => Ok(self),
        }
    }

    fn map_to_server(self, network_mapping: &NetworkMapping) -> Result<Self, MappingError<Self>>
    where
        Self: Sized,
    {
        match self.location {
            NettyRigidBodyLocation::Relative(rel_pos, parent_ent) => {
                let Some(client_ent) = network_mapping.server_from_client(&parent_ent) else {
                    return Err(MappingError::MissingRecord(self));
                };

                Ok(NettyRigidBody {
                    body_vel: self.body_vel,
                    location: NettyRigidBodyLocation::Relative(rel_pos, client_ent),
                    rotation: self.rotation,
                })
            }
            NettyRigidBodyLocation::Absolute(_) => Ok(self),
        }
    }
}

impl Mappable for StructureBlock {
    fn map_to_client(self, network_mapping: &NetworkMapping) -> Result<Self, MappingError<Self>> {
        if let Some(e) = network_mapping.client_from_server(&self.structure()) {
            Ok(Self::new(self.coords(), e))
        } else {
            Err(MappingError::MissingRecord(self))
        }
    }

    fn map_to_server(self, network_mapping: &NetworkMapping) -> Result<Self, MappingError<Self>>
    where
        Self: Sized,
    {
        if let Some(e) = network_mapping.server_from_client(&self.structure()) {
            Ok(Self::new(self.coords(), e))
        } else {
            Err(MappingError::MissingRecord(self))
        }
    }
}
