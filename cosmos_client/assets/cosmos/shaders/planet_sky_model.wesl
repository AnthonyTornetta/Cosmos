// A directional, art-directed scattering approximation in linear color space.
// There is no sun disk here: the star already has its own visible mesh.
struct SkyParameters {
    atmosphere_color: vec4<f32>,
    up_density: vec4<f32>,
    sun_direction: vec4<f32>,
    sun_color: vec4<f32>,
}

fn sky_color(direction: vec3<f32>, sky: SkyParameters) -> vec4<f32> {
    let elevation = dot(direction, sky.up_density.xyz);
    let sun_elevation = dot(sky.sun_direction.xyz, sky.up_density.xyz);
    let sun_angle = clamp(dot(direction, sky.sun_direction.xyz), -1.0, 1.0);
    let sunlight = sky.sun_direction.w;
    let daylight = smoothstep(-0.12, 0.18, sun_elevation) * sunlight;
    let twilight = (1.0 - smoothstep(0.02, 0.28, abs(sun_elevation + 0.04))) * sunlight;
    let sun_visible = smoothstep(-0.09, 0.015, sun_elevation) * sunlight;

    // Longer optical paths near the horizon make it paler and more opaque.
    let horizon = exp(-abs(elevation) * 4.5);
    let gradient = exp(-max(elevation, 0.0) * 3.2);
    let palette = max(sky.atmosphere_color.rgb, vec3(0.0));
    let zenith_color = palette * vec3(0.32, 0.50, 0.78);
    let horizon_color = mix(palette * 0.85, vec3(0.80, 0.88, 1.0), 0.46);
    let rayleigh = 0.75 * (1.0 + sun_angle * sun_angle);
    let day_color = mix(zenith_color, horizon_color, gradient) * (0.86 + 0.14 * rayleigh);
    let night_color = palette * mix(vec3(0.006, 0.008, 0.020), vec3(0.025, 0.020, 0.050), horizon);
    var radiance = mix(night_color, day_color, daylight);

    // A warm, directional belt remains after the sun passes below the horizon.
    let toward_sun = pow(max(sun_angle, 0.0), 4.0);
    let sunset_color = mix(vec3(1.0, 0.16, 0.035), sky.sun_color.rgb, smoothstep(-0.06, 0.24, sun_elevation));
    radiance += sunset_color * horizon * twilight * (0.10 + 0.70 * toward_sun);

    // Henyey-Greenstein-style forward scattering supplies a soft stellar halo.
    let asymmetry = 0.78;
    let mie = (1.0 - asymmetry * asymmetry)
        / pow(max(1.0 + asymmetry * asymmetry - 2.0 * asymmetry * sun_angle, 0.001), 1.5);
    let glow = mie * 0.025 + pow(max(sun_angle, 0.0), 8.0) * horizon * 0.10;
    radiance += sunset_color * glow * sun_visible;

    // Beer-Lambert extinction opens the existing galaxy skybox at night and at
    // altitude. Haze persists longer at the horizon than directly overhead.
    let optical_depth = sky.up_density.w * (4.5 + 6.0 * horizon)
        * (0.015 + daylight * 0.985 + twilight * 0.25);
    let opacity = clamp(1.0 - exp(-optical_depth), 0.0, 1.0);
    return vec4(max(radiance, vec3(0.0)), opacity);
}
