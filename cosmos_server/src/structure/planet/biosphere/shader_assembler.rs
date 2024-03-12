use std::fs;

use bevy::{
    app::App,
    ecs::{
        schedule::OnEnter,
        system::{Commands, Res, Resource},
    },
};
use cosmos_core::{
    registry::{identifiable::Identifiable, Registry},
    structure::planet::biosphere::RegisteredBiosphere,
};

use crate::state::GameState;

#[derive(Resource, Default)]
struct CachedShaders(Vec<(String, String)>);

fn assemble_shaders(mut commands: Commands, registered_biospheres: Res<Registry<RegisteredBiosphere>>) {
    let main_path = "cosmos/shaders/main.wgsl";
    let main_text = fs::read_to_string(&format!("assets/{main_path}")).expect("Missing main.wgsl file for biosphere generation!");

    let mut biosphere_switch_text = String::from("switch param.biosphere_id.x {\n");
    let mut import_text = String::new();

    let mut cached_shaders = CachedShaders::default();

    for biosphere in registered_biospheres.iter() {
        let num = biosphere.id();
        let unlocalized_name = biosphere.unlocalized_name();
        let mut split = unlocalized_name.split(":");
        let mod_id = split.next().expect("Empty biosphere name?");
        let biosphere_name = split
            .next()
            .unwrap_or_else(|| panic!("Unlocalized names must be formatted as modid:name - {unlocalized_name} is not valid."));

        let shader_path = format!("{mod_id}/shaders/biosphere/{biosphere_name}.wgsl");
        let shader_text =
            fs::read_to_string(format!("assets/{shader_path}")).unwrap_or_else(|_| panic!("Unable to read shader @ assets/{shader_path}."));

        import_text.push_str(&format!("#import \"{shader_path}\"::{{generate as generate_{num}}};\n",));

        cached_shaders.0.push((shader_path, shader_text));

        biosphere_switch_text.push_str(&format!(
            "        case {num}u: {{
            values[idx] = generate_{num}(param, coords);
            break;
        }}\n"
        ));
    }

    biosphere_switch_text.push_str(
        "        default: {
            // If this happens, the biosphere may not have been registered properly
            break;
        }
    }",
    );

    let main_text = main_text
        .replace("// generate_biosphere_switch", &biosphere_switch_text)
        .replace("// generate_imports", &import_text);

    cached_shaders.0.push((main_path.to_owned(), main_text.clone()));

    commands.insert_resource(cached_shaders);

    let _ = fs::create_dir_all("assets/temp/shaders/biosphere");
    fs::write("assets/temp/shaders/biosphere/main.wgsl", main_text).expect("Failed to write biosphere generation file!");
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), assemble_shaders);
}
