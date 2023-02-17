use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::physics::location::Location;
use cosmos_core::structure::{
    events::StructureCreated, ship::ship_builder::TShipBuilder, Structure,
};

use crate::structure::ship::{loading::ShipNeedsCreated, server_ship_builder::ServerShipBuilder};
use crate::GameState;

pub struct CreateShipEvent {
    pub ship_location: Location,
    pub rotation: Quat,
}

fn event_reader(
    mut created_event_writer: EventWriter<StructureCreated>,
    mut event_reader: EventReader<CreateShipEvent>,
    mut commands: Commands,
) {
    for ev in event_reader.iter() {
        let mut entity = commands.spawn_empty();

        let mut structure = Structure::new(10, 10, 10);

        let builder = ServerShipBuilder::default();

        builder.insert_ship(
            &mut entity,
            ev.ship_location,
            Velocity::zero(),
            &mut structure,
        );

        entity.insert(structure).insert(ShipNeedsCreated);

        created_event_writer.send(StructureCreated {
            entity: entity.id(),
        });
    }
}

pub fn register(app: &mut App) {
    app.add_event::<CreateShipEvent>()
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(event_reader));
}
