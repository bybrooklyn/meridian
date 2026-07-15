struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct CameraUniform {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
};

struct MaterialUniform {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _padding: vec2<f32>,
};

struct SunUniform {
    direction_to_light: vec4<f32>,
    color_and_intensity: vec4<f32>,
};

struct ShadowUniform {
    light_view_projection: array<mat4x4<f32>, 8>,
    cascade_splits: array<vec4<f32>, 2>,
    map_resolution_bias: vec4<f32>,
};

struct EnvironmentUniform {
    diffuse_intensity: vec4<f32>,
};

@group(0) @binding(0)
var base_color_texture: texture_2d<f32>;

@group(0) @binding(1)
var base_color_sampler: sampler;

@group(0) @binding(2)
var normal_texture: texture_2d<f32>;

@group(0) @binding(3)
var normal_sampler: sampler;

@group(0) @binding(4)
var metallic_roughness_texture: texture_2d<f32>;

@group(0) @binding(5)
var metallic_roughness_sampler: sampler;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(1)
var<uniform> object: ObjectUniform;

@group(2) @binding(0)
var<uniform> material: MaterialUniform;

@group(3) @binding(0)
var<uniform> sun: SunUniform;

@group(3) @binding(1)
var shadow_texture: texture_depth_2d_array;

@group(3) @binding(2)
var shadow_sampler: sampler_comparison;

@group(3) @binding(3)
var<uniform> shadow: ShadowUniform;

@group(3) @binding(4)
var environment_irradiance_texture: texture_cube<f32>;

@group(3) @binding(5)
var environment_sampler: sampler;

@group(3) @binding(6)
var<uniform> environment: EnvironmentUniform;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_position = object.model * vec4<f32>(input.position, 1.0);
    output.position = camera.view_projection * world_position;
    output.world_position = world_position.xyz;
    output.normal = normalize((object.model * vec4<f32>(input.normal, 0.0)).xyz);
    output.tangent = vec4<f32>(
        normalize((object.model * vec4<f32>(input.tangent.xyz, 0.0)).xyz),
        input.tangent.w,
    );
    output.uv = input.uv;
    return output;
}

const PI: f32 = 3.14159265359;

fn distribution_ggx(normal: vec3<f32>, half_vector: vec3<f32>, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha_squared = alpha * alpha;
    let normal_half = max(dot(normal, half_vector), 0.0);
    let normal_half_squared = normal_half * normal_half;
    let denominator = normal_half_squared * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / max(PI * denominator * denominator, 0.0001);
}

fn geometry_schlick_ggx(normal_direction: f32, roughness: f32) -> f32 {
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    return normal_direction / (normal_direction * (1.0 - k) + k);
}

fn geometry_smith(
    normal: vec3<f32>,
    view_direction: vec3<f32>,
    light_direction: vec3<f32>,
    roughness: f32,
) -> f32 {
    let normal_view = max(dot(normal, view_direction), 0.0);
    let normal_light = max(dot(normal, light_direction), 0.0);
    return geometry_schlick_ggx(normal_view, roughness) *
        geometry_schlick_ggx(normal_light, roughness);
}

fn fresnel_schlick(cosine: f32, base_reflectance: vec3<f32>) -> vec3<f32> {
    return base_reflectance + (vec3<f32>(1.0) - base_reflectance) *
        pow(1.0 - clamp(cosine, 0.0, 1.0), 5.0);
}

fn shadow_cascade_for_depth(camera_depth: f32) -> u32 {
    var cascade = 0u;
    if camera_depth > shadow.cascade_splits[0].x {
        cascade = 1u;
    }
    if camera_depth > shadow.cascade_splits[0].y {
        cascade = 2u;
    }
    if camera_depth > shadow.cascade_splits[0].z {
        cascade = 3u;
    }
    if camera_depth > shadow.cascade_splits[0].w {
        cascade = 4u;
    }
    if camera_depth > shadow.cascade_splits[1].x {
        cascade = 5u;
    }
    if camera_depth > shadow.cascade_splits[1].y {
        cascade = 6u;
    }
    if camera_depth > shadow.cascade_splits[1].z {
        cascade = 7u;
    }
    return min(cascade, max(u32(shadow.map_resolution_bias.w) - 1u, 0u));
}

fn shadow_visibility(
    world_position: vec3<f32>,
    normal: vec3<f32>,
    light_direction: vec3<f32>,
) -> f32 {
    let camera_depth = max(
        dot(world_position - camera.camera_position.xyz, camera.camera_forward.xyz),
        0.0,
    );
    let cascade = shadow_cascade_for_depth(camera_depth);
    let light_clip = shadow.light_view_projection[cascade] * vec4<f32>(world_position, 1.0);
    if light_clip.w <= 0.0 {
        return 1.0;
    }
    let projected = light_clip.xyz / light_clip.w;
    if projected.x < -1.0 || projected.x > 1.0 || projected.y < -1.0 || projected.y > 1.0 ||
        projected.z < 0.0 || projected.z > 1.0
    {
        return 1.0;
    }
    let uv = projected.xy * 0.5 + vec2<f32>(0.5);
    let slope_bias = shadow.map_resolution_bias.z *
        (1.0 - max(dot(normal, light_direction), 0.0));
    let compare_depth = projected.z - shadow.map_resolution_bias.y - slope_bias;
    let texel = vec2<f32>(1.0 / shadow.map_resolution_bias.x);
    var visibility = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            visibility += textureSampleCompare(
                shadow_texture,
                shadow_sampler,
                uv + offset,
                i32(cascade),
                compare_depth,
            );
        }
    }
    return visibility / 9.0;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(base_color_texture, base_color_sampler, input.uv);
    let normal_sample = textureSample(normal_texture, normal_sampler, input.uv).xyz * 2.0 - vec3<f32>(1.0);
    let geometric_normal = normalize(input.normal);
    let tangent = normalize(input.tangent.xyz - geometric_normal * dot(geometric_normal, input.tangent.xyz));
    let bitangent = normalize(cross(geometric_normal, tangent)) * input.tangent.w;
    let shading_normal = normalize(
        tangent * normal_sample.x + bitangent * normal_sample.y + geometric_normal * normal_sample.z
    );
    let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, input.uv);
    let metallic = clamp(material.metallic * metallic_roughness.b, 0.0, 1.0);
    let roughness = clamp(material.roughness * metallic_roughness.g, 0.04, 1.0);
    let base_color = material.base_color.rgb * sampled.rgb;
    let view_direction = normalize(camera.camera_position.xyz - input.world_position);
    let light_direction = normalize(sun.direction_to_light.xyz);
    let half_vector = normalize(view_direction + light_direction);
    let normal_view = max(dot(shading_normal, view_direction), 0.0);
    let normal_light = max(dot(shading_normal, light_direction), 0.0);
    let dielectric_reflectance = vec3<f32>(0.04);
    let base_reflectance = mix(dielectric_reflectance, base_color, metallic);
    let fresnel = fresnel_schlick(max(dot(half_vector, view_direction), 0.0), base_reflectance);
    let distribution = distribution_ggx(shading_normal, half_vector, roughness);
    let geometry = geometry_smith(shading_normal, view_direction, light_direction, roughness);
    let specular = distribution * geometry * fresnel /
        max(4.0 * normal_view * normal_light, 0.0001);
    let diffuse_weight = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    let diffuse = diffuse_weight * base_color / PI;
    let radiance = sun.color_and_intensity.rgb * sun.color_and_intensity.a;
    let irradiance = textureSample(
        environment_irradiance_texture,
        environment_sampler,
        shading_normal,
    ).rgb * environment.diffuse_intensity.x;
    let ambient = irradiance * base_color * diffuse_weight / PI;
    let shadow_visibility_factor = shadow_visibility(input.world_position, shading_normal, light_direction);
    let lit = ambient + (diffuse + specular) * radiance * normal_light * shadow_visibility_factor;
    return vec4<f32>(lit, sampled.a * material.base_color.a);
}
