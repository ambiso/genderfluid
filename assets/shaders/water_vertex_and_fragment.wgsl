#import bevy_shader_utils::simplex_noise_3d simplex_noise_3d
/* #import bevy_pbr::mesh_vertex_output MeshVertexOutput */
#import bevy_pbr::mesh_bindings   mesh
#import bevy_pbr::mesh_functions  mesh_position_local_to_clip

@group(1) @binding(13)
var height_map_texture: texture_2d<f32>;
@group(1) @binding(14)
var height_map_sampler: sampler;

@group(1) @binding(15)
var velocity_texture: texture_2d<f32>;
@group(1) @binding(16)
var velocity_sampler: sampler;

@group(1) @binding(17)
var terrain_texture: texture_2d<f32>;
@group(1) @binding(18)
var terrain_sampler: sampler;

@group(1) @binding(19)
var<uniform> is_water: u32;

// struct Vertex {
//     @location(0) position: vec3<f32>,
// };

// @vertex
// fn vertex(vertex: Vertex) -> MeshVertexOutput {
//     var out: MeshVertexOutput;
// 	out.uv = vertex.position.xz / 5.0 + 0.5;
// 	var height_offset: vec4<f32> = textureLoad(base_color_texture, vec2<i32>(i32(out.uv.x * f32(material.size)), i32(out.uv.y * f32(material.size))), 0);
//     out.clip_position = mesh_position_local_to_clip(
//         mesh.model,
//         vec4<f32>(vertex.position + vec3(0.0, height_offset.x, 0.0), 1.0)
//     );
//     return out;
// }

// @fragment
// fn fragment(
// 	input: VertexOutput,
// ) -> @location(0) vec4<f32> {
// 	var height_offset: vec4<f32> = textureSample(base_color_texture, base_color_sampler, input.uv);
//     return vec4(0.1529, 0.5764, 0.8470588, 1.0) * (height_offset.x/2.0 + 1.0);
// }

#define_import_path bevy_pbr::fragment

#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::pbr_bindings as pbr_bindings
#import bevy_pbr::pbr_types as pbr_types
#import bevy_pbr::prepass_utils

#import bevy_pbr::mesh_vertex_output       MeshVertexOutput
#import bevy_pbr::mesh_bindings            mesh
#import bevy_pbr::mesh_view_bindings       view, fog, screen_space_ambient_occlusion_texture
#import bevy_pbr::mesh_view_types          FOG_MODE_OFF
#import bevy_core_pipeline::tonemapping    screen_space_dither, powsafe, tone_mapping
#import bevy_pbr::parallax_mapping         parallaxed_uv

#import bevy_pbr::prepass_utils

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::gtao_utils gtao_multibounce
#endif

@fragment
fn fragment(
    in: MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = pbr_bindings::material.base_color;

    let is_orthographic = view.projection[3].w == 1.0;
    let V = pbr_functions::calculate_view(in.world_position, is_orthographic);
#ifdef VERTEX_UVS
    var uv = in.uv;
#ifdef VERTEX_TANGENTS
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT) != 0u) {
        let N = in.world_normal;
        let T = in.world_tangent.xyz;
        let B = in.world_tangent.w * cross(N, T);
        // Transform V from fragment to camera in world space to tangent space.
        let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));
        uv = parallaxed_uv(
            pbr_bindings::material.parallax_depth_scale,
            pbr_bindings::material.max_parallax_layer_count,
            pbr_bindings::material.max_relief_mapping_search_steps,
            uv,
            // Flip the direction of Vt to go toward the surface to make the
            // parallax mapping algorithm easier to understand and reason
            // about.
            -Vt,
        );
    }
#endif
#endif

#ifdef VERTEX_COLORS
    output_color = output_color * in.color;
#endif
#ifdef VERTEX_UVS
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSampleBias(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, uv, view.mip_bias);
    }
#endif

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
        // the material members
        var pbr_input: pbr_functions::PbrInput;

        pbr_input.material.base_color = output_color;
        pbr_input.material.reflectance = pbr_bindings::material.reflectance;
        pbr_input.material.flags = pbr_bindings::material.flags;
        pbr_input.material.alpha_cutoff = pbr_bindings::material.alpha_cutoff;

        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = pbr_bindings::material.emissive;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSampleBias(pbr_bindings::emissive_texture, pbr_bindings::emissive_sampler, uv, view.mip_bias).rgb, 1.0);
        }
#endif
        pbr_input.material.emissive = emissive;

        var metallic: f32 = pbr_bindings::material.metallic;
        var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSampleBias(pbr_bindings::metallic_roughness_texture, pbr_bindings::metallic_roughness_sampler, uv, view.mip_bias);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
#endif
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        // TODO: Split into diffuse/specular occlusion?
        var occlusion: vec3<f32> = vec3(1.0);
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = vec3(textureSampleBias(pbr_bindings::occlusion_texture, pbr_bindings::occlusion_sampler, uv, view.mip_bias).r);
        }
#endif
#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = gtao_multibounce(ssao, pbr_input.material.base_color.rgb);
        occlusion = min(occlusion, ssao_multibounce);
#endif
        pbr_input.occlusion = occlusion;

        pbr_input.frag_coord = in.position;
        pbr_input.world_position = in.world_position;

        pbr_input.world_normal = pbr_functions::prepare_world_normal(
            in.world_normal,
            (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            is_front,
        );

        pbr_input.is_orthographic = is_orthographic;

#ifdef LOAD_PREPASS_NORMALS
        pbr_input.N = bevy_pbr::prepass_utils::prepass_normal(in.position, 0u);
#else
        pbr_input.N = pbr_functions::apply_normal_mapping(
            pbr_bindings::material.flags,
            pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif
#endif
#ifdef VERTEX_UVS
            uv,
#endif
            view.mip_bias,
        );
#endif

        pbr_input.V = V;
        pbr_input.occlusion = occlusion;

        pbr_input.flags = mesh.flags;

        output_color = pbr_functions::pbr(pbr_input);
    } else {
        output_color = pbr_functions::alpha_discard(pbr_bindings::material, output_color);
    }

    // fog
    if (fog.mode != FOG_MODE_OFF && (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) {
        output_color = pbr_functions::apply_fog(fog, output_color, in.world_position.xyz, view.world_position.xyz);
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color, view.color_grading);
#ifdef DEBAND_DITHER
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
#endif
#endif
#ifdef PREMULTIPLY_ALPHA
    output_color = pbr_functions::premultiply_alpha(pbr_bindings::material.flags, output_color);
#endif
    // return vec4(in.world_normal, 1.0);
    var is_visible_water: f32 = 1.0;
    if (is_water == u32(1)) {
        let dim = textureDimensions(height_map_texture);
	    let height_offset: f32 = textureSample(height_map_texture, height_map_sampler, in.uv).x;
        if (height_offset < 0.01) {
            is_visible_water = 0.0;
        }
    }
    var noise_factor = 0.005;
    var noise_scale = 50.0;
    if (is_water == u32(1)) {
        noise_factor = 0.02;
        noise_scale = 500.0;
    }
    let n = simplex_noise_3d(vec3(in.uv * noise_scale, 0.0));
    let noise = vec4(vec3(n), 0.0);
    return (output_color + noise*noise_factor) * vec4(1.0, 1.0, 1.0, is_visible_water);
}

#import bevy_pbr::mesh_functions as mesh_functions
#import bevy_pbr::skinning
#import bevy_pbr::morph
#import bevy_pbr::mesh_bindings       mesh
#import bevy_pbr::mesh_vertex_output  MeshVertexOutput

struct Vertex {
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif
};

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let weight_count = bevy_pbr::morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr::morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr::morph::normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr::morph::tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> MeshVertexOutput {
    var out: MeshVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var model = bevy_pbr::skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    var model = mesh.model;
#endif

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

    let dim = textureDimensions(height_map_texture);
	let height_offset: f32 = textureLoad(height_map_texture, vec2<i32>(i32(out.uv.x * f32(dim.x)), i32(out.uv.y * f32(dim.y))), 0).x;
	
    
    var terrain_height_offset: f32 = 0.0;
	if (is_water != u32(0)) {
        let terimdim = textureDimensions(terrain_texture);
        terrain_height_offset = textureLoad(terrain_texture, vec2<i32>(i32(out.uv.x * f32(terimdim.x)), i32(out.uv.y * f32(terimdim.y))), 0).x;
    }

    let height_xp1: f32 = textureLoad(height_map_texture, vec2<i32>(i32(out.uv.x * f32(dim.x))+1, i32(out.uv.y * f32(dim.y))), 0).x;
	let height_zp1: f32 = textureLoad(height_map_texture, vec2<i32>(i32(out.uv.x * f32(dim.x)), i32(out.uv.y * f32(dim.y))+1), 0).x;
	let height_xm1: f32 = textureLoad(height_map_texture, vec2<i32>(i32(out.uv.x * f32(dim.x)) - 1, i32(out.uv.y * f32(dim.y))), 0).x;
	let height_zm1: f32 = textureLoad(height_map_texture, vec2<i32>(i32(out.uv.x * f32(dim.x)), i32(out.uv.y * f32(dim.y)) - 1), 0).x;
    let dxz = 1.0;
    let dhdx = (height_xp1 - height_offset)/dxz;
    let dhdx2 = -(height_xm1 - height_offset)/dxz;
    let dhdz = (height_zp1 - height_offset)/dxz;
    let dhdz2 = -(height_zm1 - height_offset)/dxz;
    let impact_factor = 10.0;
    let normal1 = normalize(vec3(-dhdx * impact_factor, 1.0, -dhdz * impact_factor));
    let normal2 = normalize(vec3(-dhdx * impact_factor, 1.0, -dhdz2 * impact_factor));
    let normal3 = normalize(vec3(-dhdx2 * impact_factor, 1.0, -dhdz * impact_factor));
    let normal4 = normalize(vec3(-dhdx2 * impact_factor, 1.0, -dhdz2 * impact_factor));

    let normal = normalize(normal1 + normal2 + normal3 + normal4);


#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = bevy_pbr::skinning::skin_normals(model, normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(normal);
    // out.world_normal = normal;
#endif
#endif

#ifdef VERTEX_POSITIONS
    let position = vertex.position + vec3(0.0, height_offset + terrain_height_offset, 0.0);
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(position, 1.0));
    out.position = mesh_functions::mesh_position_world_to_clip(out.world_position);
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(model, vertex.tangent);
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
    

    return out;
}

// @fragment
// fn fragment(
//     mesh: MeshVertexOutput,
// ) -> @location(0) vec4<f32> {
// #ifdef VERTEX_COLORS
//     return mesh.color;
// #else
//     return vec4<f32>(1.0, 0.0, 1.0, 1.0);
// #endif
// }
