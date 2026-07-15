# Shaders

Runtime shader sources and versioned manifests live here. `clear.wgsl` is the
minimal bootstrap shader used by the renderer startup smoke path; the RHI
constructs its pipeline before active runtime traversal. Compilation and
validation are offline/build-time responsibilities, and active gameplay must
not create pipelines in release builds. `triangle.wgsl` is the first indexed
mesh smoke shader and consumes a position-only vertex layout.
`textured_triangle.wgsl` exercises the initial sampled base-color binding
contract. `textured_uniform_triangle.wgsl` adds the camera/object uniform
group used by the next renderer smoke boundary. `textured_material_triangle.wgsl`
adds base-color, normal, and metallic-roughness channel sampling, the direct
PBR material parameter group, a camera-position uniform, and a 32-byte
sun-light group with a Cook–Torrance direct-light response. The same material
path now consumes the RHI-owned cascaded depth array and applies camera-depth
cascade selection with 3x3 comparison PCF. It also samples an RHI-owned,
pre-convolved irradiance cube for diffuse image-based lighting. Prefiltered
specular IBL, a BRDF integration LUT, and advanced material variants remain
separate engine work.
`shadow_depth.wgsl` is the depth-only vertex path used by the RHI-owned
cascaded shadow pass; visible shadow-quality captures remain a later
validation step because the current host smoke surface is occluded.
