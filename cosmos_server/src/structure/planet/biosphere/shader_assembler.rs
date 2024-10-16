//! Assembles the shaders in the assets directory for terrain generation & gets it ready to be used by the server and sent to the clients.

use std::{ffi::OsStr, fs};

use bevy::{
    app::App,
    ecs::system::{Commands, Res, Resource},
    state::state::OnEnter,
    utils::hashbrown::HashSet,
};
use cosmos_core::{
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::planet::biosphere::Biosphere,
};

#[derive(Debug, Resource, Default)]
/// Contains every shader loaded with its path and contents to send to clients
///
/// Vec<(path, contents)>
pub struct CachedShaders(pub Vec<(String, String)>);

fn assemble_shaders(mut commands: Commands, registered_biospheres: Res<Registry<Biosphere>>) {
    let main_path = "cosmos/shaders/biosphere/main.wgsl";
    let main_text = fs::read_to_string(format!("assets/{main_path}")).expect("Missing main.wgsl file for biosphere generation!");

    let mut biosphere_switch_text = String::from("switch param.biosphere_id.x {\n");
    let mut import_text = String::new();

    let mut cached_shaders = CachedShaders::default();

    let mut biospheres_to_find = HashSet::default();

    for biosphere in registered_biospheres.iter() {
        let num = biosphere.id();
        let unlocalized_name = biosphere.unlocalized_name();
        let mut split = unlocalized_name.split(':');
        let mod_id = split.next().expect("Empty biosphere name?");
        let biosphere_name = split
            .next()
            .unwrap_or_else(|| panic!("Unlocalized names must be formatted as modid:name - {unlocalized_name} is not valid."));

        let shader_path = format!("{mod_id}/shaders/biosphere/biospheres/{biosphere_name}.wgsl");
        import_text.push_str(&format!("#import \"{shader_path}\"::{{generate as generate_{num}}};\n",));
        biospheres_to_find.insert(shader_path);

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

    recursively_add_files(
        OsStr::new("assets/cosmos/shaders/biosphere/"),
        &mut cached_shaders,
        &mut biospheres_to_find,
        main_path,
    )
    .unwrap_or_else(|e| panic!("Failed to load files {e:?}!"));

    if !biospheres_to_find.is_empty() {
        panic!("Failed to find biosphere generation scripts for all biospheres! Missing: {biospheres_to_find:?}");
    }

    let main_text = main_text
        .replace("// generate_biosphere_switch", &biosphere_switch_text)
        .replace("// generate_imports", &import_text);

    cached_shaders.0.push(("main.wgsl".to_owned(), main_text.clone()));

    commands.insert_resource(cached_shaders);

    let _ = fs::create_dir_all("assets/temp/shaders/biosphere");
    fs::write("assets/temp/shaders/biosphere/main.wgsl", main_text).expect("Failed to write biosphere generation file!");
}

fn recursively_add_files(
    path: &OsStr,
    cached_shaders: &mut CachedShaders,
    biospheres_to_find: &mut HashSet<String>,
    main_path: &str,
) -> std::io::Result<()> {
    let contents = fs::read_dir(path)?;

    for file in contents {
        let file = file?;
        let path = file.path();
        let path_str = path.as_os_str().to_str().expect("Failed to convert OsStr to str.");
        // Remove the assets/ folder from the path because it's meaningless to the client
        let clean_path = &path_str["assets/".len()..].replacen('\\', "/", usize::MAX);

        if clean_path == main_path {
            continue;
        }

        if path.is_file() {
            if path.extension().map(|x| x == "wgsl").unwrap_or(false) {
                // Removes the "assets/" from the beginning of the path and converts any '\' to /
                biospheres_to_find.remove(clean_path);

                let shader_text = fs::read_to_string(&path)?.replacen('\r', "", usize::MAX);

                cached_shaders.0.push((clean_path.to_owned(), shader_text));
            }
        } else {
            recursively_add_files(path.as_os_str(), cached_shaders, biospheres_to_find, main_path)?;
        }
    }

    Ok(())
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), assemble_shaders);
}
