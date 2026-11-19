//! Compile custom WESL against the actual Bevy shader libraries without requiring a GPU.

use bevy::asset::{AssetId, Assets};
use bevy::shader::{Shader, ShaderCache, ShaderCacheSource, ShaderDefVal};
use std::{fs, path::Path};

fn files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for f in fs::read_dir(dir).unwrap() {
        let p = f.unwrap().path();
        if p.is_dir() {
            if p.file_name().unwrap() != "temp" {
                files(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "wesl") {
            out.push(p);
        }
    }
}

fn submits_to_oit_and_discards(module: &naga::Module, block: &naga::Block) -> bool {
    let submits_here = block.iter().zip(block.iter().skip(1)).any(|(first, next)| {
        matches!(first, naga::Statement::Call { function, .. }
            if module.functions[*function].name.as_deref().is_some_and(|name| name.ends_with("oit_draw")))
            && matches!(next, naga::Statement::Kill)
    });
    submits_here
        || block.iter().any(|statement| match statement {
            naga::Statement::Block(block) => submits_to_oit_and_discards(module, block),
            naga::Statement::If { accept, reject, .. } => {
                submits_to_oit_and_discards(module, accept) || submits_to_oit_and_discards(module, reject)
            }
            naga::Statement::Switch { cases, .. } => cases.iter().any(|case| submits_to_oit_and_discards(module, &case.body)),
            naga::Statement::Loop { body, continuing, .. } => {
                submits_to_oit_and_discards(module, body) || submits_to_oit_and_discards(module, continuing)
            }
            _ => false,
        })
}

#[test]
fn material_and_terrain_shaders_compile_with_bevy_020() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    // Restrict metadata to the host so this offline test doesn't need unrelated platform crates.
    let rustc = std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("-vV")
        .output()
        .expect("rustc version lookup failed");
    let version = String::from_utf8(rustc.stdout).unwrap();
    let host = version.lines().find_map(|line| line.strip_prefix("host: ")).unwrap();
    // Cargo metadata supplies the exact shader library sources from the locked Bevy version.
    let metadata = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--format-version=1", "--locked", "--offline", "--filter-platform", host])
        .current_dir(repo)
        .output()
        .expect("cargo metadata failed");
    assert!(metadata.status.success(), "{}", String::from_utf8_lossy(&metadata.stderr));
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let mut cache = ShaderCache::<String, ()>::new((), |_, source, _| match source {
        ShaderCacheSource::Wgsl(s) => {
            let m =
                naga::front::wgsl::parse_str(&s).map_err(|e| bevy::shader::ShaderCacheError::CreateShaderModule(e.emit_to_string(&s)))?;
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
                .validate(&m)
                .map_err(|e| bevy::shader::ShaderCacheError::CreateShaderModule(format!("{e:?}")))?;
            Ok(s)
        }
        _ => unreachable!(),
    });
    let mut assets = Assets::<Shader>::default();
    let bevy_version = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["name"] == "bevy")
        .unwrap()["version"]
        .as_str()
        .unwrap();
    for package in metadata["packages"].as_array().unwrap() {
        let name = package["name"].as_str().unwrap();
        if !name.starts_with("bevy_") || package["version"] != bevy_version {
            continue;
        }
        let base = Path::new(package["manifest_path"].as_str().unwrap()).parent().unwrap().join("src");
        let mut paths = Vec::new();
        files(&base, &mut paths);
        for path in paths {
            let relative = path.strip_prefix(&base).unwrap().to_string_lossy().replace('\\', "/");
            let module = format!("embedded://{name}/{relative}");
            let shader = Shader::from_wesl(fs::read_to_string(path).unwrap(), module);
            let handle = assets.add(shader.clone());
            cache.set_shader(handle.id(), shader);
        }
    }
    let mut targets: Vec<(String, AssetId<Shader>)> = Vec::new();
    for root in ["cosmos_client", "cosmos_server"] {
        let base = repo.join(root).join("assets");
        let mut paths = Vec::new();
        files(&base, &mut paths);
        for path in paths {
            let name = path.strip_prefix(&base).unwrap().to_string_lossy().replace('\\', "/");
            let mut source = fs::read_to_string(&path).unwrap();
            if name.ends_with("biosphere/main.wesl") {
                source = source.replace(
                    "// generate_imports",
                    r#"
import package::cosmos::shaders::biosphere::biospheres::grass::generate as generate_0;
import package::cosmos::shaders::biosphere::biospheres::ice::generate as generate_1;
import package::cosmos::shaders::biosphere::biospheres::molten::generate as generate_2;
"#,
                );
                source = source.replace(
                    "// generate_biosphere_switch",
                    r#"
switch param.biosphere_id.x {
    case 0u: { values[idx] = generate_0(param, coords); }
    case 1u: { values[idx] = generate_1(param, coords); }
    case 2u: { values[idx] = generate_2(param, coords); }
    default: { break; }
}
"#,
                );
            }
            let shader = Shader::from_wesl(source, name.clone());
            let h = assets.add(shader.clone());
            cache.set_shader(h.id(), shader.clone());
            if root == "cosmos_server" {
                // The client receives the same files in a temporary namespace.
                let client_path = if name.ends_with("/main.wesl") {
                    "temp/shaders/biosphere/main.wesl".to_owned()
                } else {
                    format!("temp/shaders/biosphere/{name}")
                };
                let client_source = match &shader.source {
                    bevy::shader::Source::Wesl(source) => source.as_ref(),
                    _ => unreachable!(),
                }
                .replace("import package::", "import package::temp::shaders::biosphere::");
                let client_shader = Shader::from_wesl(client_source, client_path.clone());
                let client_handle = assets.add(client_shader.clone());
                cache.set_shader(client_handle.id(), client_shader);
                if name.ends_with("/main.wesl") {
                    targets.push((client_path, client_handle.id()));
                }
            }
            if !name.contains("biosphere/") || name.ends_with("/main.wesl") {
                targets.push((name, h.id()));
            }
        }
    }
    let defs: Vec<ShaderDefVal> = [
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "VERTEX_OUTPUT_INSTANCE_INDEX",
        "NORMAL_PREPASS_OR_DEFERRED_PREPASS",
        "MULTIPLE_LIGHT_PROBES_IN_ARRAY",
        "IRRADIANCE_VOLUMES_ARE_USABLE",
    ]
    .into_iter()
    .map(Into::into)
    .chain([
        ShaderDefVal::UInt("MATERIAL_BIND_GROUP".into(), 3),
        ShaderDefVal::UInt("MAX_DIRECTIONAL_LIGHTS".into(), 10),
        ShaderDefVal::UInt("MAX_CASCADES_PER_LIGHT".into(), 4),
        ShaderDefVal::UInt("MAX_RECT_LIGHTS".into(), 10),
        ShaderDefVal::UInt("PER_OBJECT_BUFFER_BATCH_SIZE".into(), 64),
        ShaderDefVal::UInt("AVAILABLE_STORAGE_BUFFER_BINDINGS".into(), 8),
    ])
    .collect();
    let mut failures = Vec::new();
    for (name, id) in targets {
        let shader_defs: Vec<_> = defs
            .iter()
            .filter(|def| {
                !name.ends_with("/lod.wesl")
                    || !matches!(def, ShaderDefVal::Bool(key, _) if key == "VERTEX_NORMALS" || key == "VERTEX_UVS_A")
            })
            .cloned()
            .collect();
        let mut variants = vec![("forward", shader_defs.clone())];
        if ["/block.wesl", "/animated.wesl", "/lod.wesl", "/shield.wesl"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
        {
            let mut oit = shader_defs.clone();
            oit.push("MATERIAL_OIT_ENABLED".into());
            oit.push("OIT_ENABLED".into());
            if name.ends_with("/shield.wesl") {
                // Additive materials share the premultiplied pipeline.
                let mut additive = shader_defs.clone();
                additive.push("BLEND_PREMULTIPLIED_ALPHA".into());
                variants.push(("additive", additive));
                oit.push("BLEND_PREMULTIPLIED_ALPHA".into());
            }
            variants.push(("oit", oit));
        }
        if name.ends_with("/black_hole.wesl") {
            let mut premultiplied = shader_defs.clone();
            premultiplied.push("BLEND_PREMULTIPLIED_ALPHA".into());
            variants.push(("premultiplied", premultiplied.clone()));
            // The disk opts out of material OIT even when the camera enables it.
            premultiplied.push("OIT_ENABLED".into());
            variants.push(("oit-camera", premultiplied));
        }
        if name.contains("prepass.wesl") || name.ends_with("/shield.wesl") {
            let mut prepass = shader_defs.clone();
            prepass.extend([
                "PREPASS_FRAGMENT".into(),
                "NORMAL_PREPASS".into(),
                "PREPASS_PIPELINE".into(),
                "DEFERRED_PREPASS".into(),
            ]);
            variants.push(("prepass", prepass));
        }
        for (variant, shader_defs) in variants {
            match cache.get(0, id, &shader_defs) {
                Ok(compiled) => {
                    if name.ends_with("/shield.wesl") || name.ends_with("/black_hole.wesl") {
                        let module = naga::front::wgsl::parse_str(&compiled).unwrap();
                        let fragment = module
                            .entry_points
                            .iter()
                            .find(|entry| entry.stage == naga::ShaderStage::Fragment)
                            .unwrap();
                        assert_eq!(
                            submits_to_oit_and_discards(&module, &fragment.function.body),
                            variant == "oit",
                            "Unexpected OIT submission in {name} ({variant})"
                        );
                    }
                    println!("PASS {name} ({variant})");
                }
                Err(e) => failures.push(format!("{name} ({variant}): {e}")),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
