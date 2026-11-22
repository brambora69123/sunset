// Improved transparency's cloud stages (`clouds.fsh` under OIT_*), appended to
// entity.wgsl after oit.wgsl. The clouds' vertex shader and draw are the
// classic ones; only the fragment colour goes through the three stages.

// The cloud's colour; it fades out towards the cloud fog end everywhere but in
// the depth bounds stage, which sees the unfaded alpha.
fn oit_cloud_color_of(texel: vec4<f32>, in: Out, front: bool, fade: bool) -> vec4<f32> {
    let color = texel * select(in.back_color, in.color, front);
    let faded = cloud_fade(in.fog_distance.x);
    return vec4(color.rgb, color.a * select(1.0, faded, fade));
}
fn oit_cloud_color(in: Out, front: bool, fade: bool) -> vec4<f32> {
    return oit_cloud_color_of(textureSample(atlas, atlas_sampler, in.uv), in, front, fade);
}
// The era's layer samples its own `clouds.png` (the cloud group's texture).
fn oit_cloud_color_era(in: Out, front: bool, fade: bool) -> vec4<f32> {
    return oit_cloud_color_of(textureSample(cloud_texture, cloud_sampler, in.uv), in, front, fade);
}
fn oit_cloud_bounds(in: Out, color: vec4<f32>) -> vec4<f32> {
    if color.a < 0.01 { discard; }
    let linear = oit_linear_depth(in.clip.z);
    let opaque = select(0.0, in.clip.z, color.a > OIT_FULLY_OPAQUE_ALPHA);
    return vec4(-linear, linear, in.clip.z, opaque);
}
fn oit_cloud_transmittance(in: Out, color: vec4<f32>) -> OitCoefficients {
    return oit_add_transmittance(1.0 - color.a, in.clip.z, in.clip.xy);
}
fn oit_cloud_accumulate(in: Out, texel_color: vec4<f32>) -> vec4<f32> {
    var color = texel_color;
    if HDR_WRITE_LIGHT && globals.hdr_light.w != 0.0 {
        color = vec4(hdr_written(color.rgb, 1.0, globals.hdr_settings, globals.hdr_gains), color.a);
    }
    let seen = oit_sample_transmittance(vec2<i32>(in.clip.xy),
        oit_normalize_depth(in.clip.z, in.clip.xy), 1.0 - color.a);
    return vec4(color.rgb * color.a, color.a) * seen;
}
@fragment fn fs_oit_cloud_bounds(in: Out, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    return oit_cloud_bounds(in, oit_cloud_color(in, front, false));
}
@fragment fn fs_oit_cloud_bounds_era(in: Out, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    return oit_cloud_bounds(in, oit_cloud_color_era(in, front, false));
}
@fragment fn fs_oit_cloud_transmittance(in: Out, @builtin(front_facing) front: bool) -> OitCoefficients {
    return oit_cloud_transmittance(in, oit_cloud_color(in, front, true));
}
@fragment fn fs_oit_cloud_transmittance_era(in: Out, @builtin(front_facing) front: bool) -> OitCoefficients {
    return oit_cloud_transmittance(in, oit_cloud_color_era(in, front, true));
}
@fragment fn fs_oit_cloud_accumulate(in: Out, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    return oit_cloud_accumulate(in, oit_cloud_color(in, front, true));
}
@fragment fn fs_oit_cloud_accumulate_era(in: Out, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    return oit_cloud_accumulate(in, oit_cloud_color_era(in, front, true));
}
