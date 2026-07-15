struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0)
var base_color_texture: texture_2d<f32>;

@group(0) @binding(1)
var base_color_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 1.0);
    return output;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return textureSample(base_color_texture, base_color_sampler, vec2<f32>(0.5, 0.5));
}
