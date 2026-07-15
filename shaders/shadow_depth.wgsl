struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct CameraUniform {
    view_projection: mat4x4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(1)
var<uniform> object: ObjectUniform;

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_projection * object.model * vec4<f32>(input.position, 1.0);
}
