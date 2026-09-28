//! The Minecraft map's terrain: MinecraftOSS section meshes drawn before the
//! exact material passes, with the same camera and scene depth range, so the
//! players, weapons and effects that follow sort against it.
use std::sync::Arc;

use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::binding_types::{
    sampler, texture_2d, uniform_buffer_sized,
};
use bevy::render::render_resource::{
    AddressMode, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindingResource, BlendState,
    Buffer, BufferBinding, BufferDescriptor, BufferInitDescriptor, BufferUsages, ColorTargetState,
    ColorWrites, CompareFunction, DepthStencilState, Extent3d, FilterMode, IndexFormat,
    MipmapFilterMode, MultisampleState, Origin3d, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, RawFragmentState, RawRenderPipelineDescriptor,
    RawVertexBufferLayout, RawVertexState, RenderPipeline, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, VertexAttribute, VertexFormat, VertexStepMode,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::view::{ExtractedView, Msaa, ViewTarget};
use bevy::render::{Render, RenderApp, RenderSystems};

use super::depth_range::{GFX_DEPTH_RANGE_SCENE, reverse_z_viewport_depth};
use super::exact_pipeline::ExactPipelineRegistry;
use super::scene_depth::{SCENE_DEPTH_FORMAT, SceneDepthTexture};

/// Bytes of one MinecraftOSS `SectionVertex`: position, atlas uv, colour,
/// sky and block light (times 16), padding.
pub const MINECRAFT_VERTEX_BYTES: u64 = 28;
/// Map units per block, as `sim::voxel::BLOCK`.
const BLOCK: f32 = 40.0;

pub struct MinecraftSectionUpload {
    pub pos: [i32; 3],
    pub vertices: Vec<u8>,
    pub indices: Vec<u32>,
    pub transparent_start: Option<u32>,
}

pub struct MinecraftAtlasImage {
    pub width: u32,
    pub height: u32,
    /// RGBA8 levels, largest first.
    pub levels: Vec<Vec<u8>>,
}

/// One frame of the Minecraft world as the render world sees it.
#[derive(Resource, Default)]
pub struct MinecraftWorldFrame {
    pub active: bool,
    pub origin: [f64; 3],
    pub view_distance: f32,
    pub atlas: Option<Arc<MinecraftAtlasImage>>,
    pub uploads: Vec<MinecraftSectionUpload>,
    pub removed: Vec<[i32; 3]>,
    pub visible: Vec<[i32; 3]>,
    pub generation: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct TerrainView {
    clip_from_rel: [f32; 16],
    /// View origin in map units.
    view: [f32; 4],
    /// The block at map origin; `w` is the fog end in blocks.
    origin: [f32; 4],
    sky: [f32; 4],
}

const VIEW_SIZE: u64 = std::mem::size_of::<TerrainView>() as u64;
/// The plains sky colour, in sRGB values as Minecraft keeps it.
const SKY: [f32; 4] = [0.47, 0.65, 1.0, 1.0];

struct SectionGpu {
    vertices: Buffer,
    indices: Buffer,
    count: u32,
    transparent_start: u32,
}

#[derive(Resource, Default)]
struct TerrainGpu {
    generation: u64,
    view: Option<Buffer>,
    atlas: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    sampler: Option<Sampler>,
    bind: Option<BindGroup>,
    sections: HashMap<[i32; 3], SectionGpu>,
    visible: Vec<[i32; 3]>,
    pipelines: HashMap<(TextureFormat, u32), [RenderPipeline; 3]>,
}

pub(super) fn register(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<MinecraftWorldFrame>()
        .init_resource::<TerrainGpu>()
        .add_systems(Render, prepare_terrain.in_set(RenderSystems::PrepareResources))
        .add_systems(
            Core3d,
            draw_terrain
                .in_set(Core3dSystems::MainPass)
                .before(super::draw::ExactColourDrawSet),
        );
}

fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "iw4l_minecraft_terrain",
        &[
            uniform_buffer_sized(false, std::num::NonZeroU64::new(VIEW_SIZE))
                .visibility(ShaderStages::VERTEX_FRAGMENT)
                .build(0, ShaderStages::VERTEX_FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(1, ShaderStages::FRAGMENT),
            sampler(SamplerBindingType::Filtering)
                .visibility(ShaderStages::FRAGMENT)
                .build(2, ShaderStages::FRAGMENT),
        ],
    )
}

static HIDES_MAP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the loaded IW4 map is only standing in for a Minecraft world, so
/// its own world surfaces are not drawn.
pub(super) fn hides_map() -> bool {
    HIDES_MAP.load(std::sync::atomic::Ordering::Relaxed)
}

fn prepare_terrain(
    mut frame: ResMut<MinecraftWorldFrame>,
    published: Option<Res<super::PublishedRenderFrame>>,
    registry: Res<ExactPipelineRegistry>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut gpu: ResMut<TerrainGpu>,
) {
    HIDES_MAP.store(frame.active, std::sync::atomic::Ordering::Relaxed);
    if !frame.active {
        if !gpu.sections.is_empty() || gpu.atlas.is_some() {
            *gpu = TerrainGpu {
                pipelines: std::mem::take(&mut gpu.pipelines),
                ..TerrainGpu::default()
            };
        }
        return;
    }
    if gpu.generation != frame.generation {
        gpu.sections.clear();
        gpu.generation = frame.generation;
    }
    if let Some(atlas) = frame.atlas.clone()
        && gpu.atlas.as_ref().is_none_or(|(held, _)| !Arc::ptr_eq(held, &atlas))
    {
        let view = upload_atlas(&device, &queue, &atlas);
        gpu.atlas = Some((atlas, view));
        gpu.bind = None;
    }
    if gpu.view.is_none() {
        gpu.view = Some(device.create_buffer(&BufferDescriptor {
            label: Some("iw4l_minecraft_terrain_view"),
            size: VIEW_SIZE,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        gpu.sampler = Some(device.create_sampler(&SamplerDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Linear,
            ..default()
        }));
    }
    if gpu.bind.is_none()
        && let (Some(view), Some((_, atlas)), Some(sampler)) =
            (gpu.view.as_ref(), gpu.atlas.as_ref(), gpu.sampler.as_ref())
    {
        let layout = registry.bind_group_layout(&device, &layout());
        let bind = device.create_bind_group(
            "iw4l_minecraft_terrain",
            &layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: view,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(atlas),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        );
        gpu.bind = Some(bind);
    }
    for pos in std::mem::take(&mut frame.removed) {
        gpu.sections.remove(&pos);
    }
    for upload in std::mem::take(&mut frame.uploads) {
        if upload.indices.is_empty() || upload.vertices.is_empty() {
            gpu.sections.remove(&upload.pos);
            continue;
        }
        let vertices = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("iw4l_minecraft_section_vertices"),
            contents: &upload.vertices,
            usage: BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("iw4l_minecraft_section_indices"),
            contents: bytemuck::cast_slice(&upload.indices),
            usage: BufferUsages::INDEX,
        });
        let count = upload.indices.len() as u32;
        gpu.sections.insert(
            upload.pos,
            SectionGpu {
                vertices,
                indices,
                count,
                transparent_start: upload.transparent_start.unwrap_or(count).min(count),
            },
        );
    }
    gpu.visible = frame.visible.clone();

    let mut view = TerrainView {
        sky: SKY,
        ..TerrainView::default()
    };
    if let Some(exec) = published.as_ref().map(|p| &p.exec_frame)
        && let Some(clip_from_world) = exec.clip_from_world
    {
        let o = exec.view_origin;
        view.clip_from_rel = (clip_from_world * Mat4::from_translation(o)).to_cols_array();
        view.view = [o.x, o.y, o.z, 1.0];
    }
    view.origin = [
        frame.origin[0] as f32,
        frame.origin[1] as f32,
        frame.origin[2] as f32,
        frame.view_distance * 16.0,
    ];
    if let Some(buffer) = gpu.view.as_ref() {
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&view));
    }
}

fn upload_atlas(device: &RenderDevice, queue: &RenderQueue, atlas: &MinecraftAtlasImage) -> TextureView {
    let levels = atlas.levels.len().max(1) as u32;
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("iw4l_minecraft_atlas"),
        size: Extent3d {
            width: atlas.width,
            height: atlas.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, texels) in atlas.levels.iter().enumerate() {
        let (w, h) = ((atlas.width >> level).max(1), (atlas.height >> level).max(1));
        if texels.len() < (w * h * 4) as usize {
            break;
        }
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            texels,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&TextureViewDescriptor::default())
}

fn draw_terrain(
    view: ViewQuery<(&ViewTarget, &SceneDepthTexture, &ExtractedView, Option<&Msaa>)>,
    registry: Res<ExactPipelineRegistry>,
    device: Res<RenderDevice>,
    mut gpu: ResMut<TerrainGpu>,
    mut context: RenderContext,
) {
    let Some(bind) = gpu.bind.clone() else {
        return;
    };
    let (target, depth, extracted_view, msaa) = view.into_inner();
    let format = target.main_texture_format();
    let samples = msaa.map_or(1, Msaa::samples);
    let [sky, opaque, translucent] = gpu
        .pipelines
        .entry((format, samples))
        .or_insert_with(|| pipelines(&device, &registry, format, samples))
        .clone();
    let attachments = [Some(target.get_color_attachment())];
    let mut pass =
        context.begin_tracked_render_pass(bevy::render::render_resource::RenderPassDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    let vp = extracted_view.viewport;
    let (depth_min, depth_max) = reverse_z_viewport_depth(GFX_DEPTH_RANGE_SCENE);
    pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, depth_min, depth_max);
    pass.set_bind_group(0, &bind, &[]);
    pass.set_render_pipeline(&opaque);
    for pos in &gpu.visible {
        let Some(section) = gpu.sections.get(pos) else {
            continue;
        };
        if section.transparent_start == 0 {
            continue;
        }
        pass.set_vertex_buffer(0, section.vertices.slice(..));
        pass.set_index_buffer(section.indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..section.transparent_start, 0, 0..1);
    }
    // Sky wherever nothing has been drawn yet: a full-screen triangle at the
    // cleared depth.
    pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, 0.0, 0.0);
    pass.set_render_pipeline(&sky);
    pass.draw(0..3, 0..1);
    pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, depth_min, depth_max);
    pass.set_render_pipeline(&translucent);
    for pos in gpu.visible.iter().rev() {
        let Some(section) = gpu.sections.get(pos) else {
            continue;
        };
        if section.transparent_start >= section.count {
            continue;
        }
        pass.set_vertex_buffer(0, section.vertices.slice(..));
        pass.set_index_buffer(section.indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(section.transparent_start..section.count, 0, 0..1);
    }
}

const TERRAIN_WGSL: &str = r#"
struct TerrainView {
    clip_from_rel: mat4x4<f32>,
    view: vec4<f32>,
    origin: vec4<f32>,
    sky: vec4<f32>,
}
@group(0) @binding(0) var<uniform> view: TerrainView;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

const BLOCK: f32 = 40.0;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) distance: f32,
}

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) light: vec2<f32>,
) -> Out {
    let rel = position - view.origin.xyz;
    let map = vec3<f32>(rel.x, -rel.z, rel.y) * BLOCK;
    var out: Out;
    out.clip = view.clip_from_rel * vec4<f32>(map - view.view.xyz, 1.0);
    out.uv = uv;
    out.colour = colour;
    // Unorm of level * 16 back to a level in 0..15.
    out.light = light * 255.0 / 16.0;
    out.distance = length(map - view.view.xyz) / BLOCK;
    return out;
}

fn light_brightness(level: f32) -> f32 {
    return level / (4.0 - 3.0 * level);
}

// MinecraftOSS's port of 26.3 lightmap.fsh with a daytime Overworld: no
// ambient light, full sky light, warm block light, and the default
// brightness option's notGamma blend.
fn lightmap(sky_level: f32, block_level: f32) -> vec3<f32> {
    let sky = sky_level / 15.0;
    let block = block_level / 15.0;
    var color = vec3<f32>(light_brightness(sky));
    let parabolic = (2.0 * block - 1.0) * (2.0 * block - 1.0);
    let block_color = mix(vec3<f32>(1.0, 0.85, 0.7), vec3<f32>(1.0), 0.9 * parabolic);
    color += block_color * light_brightness(block);
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    let greatest = max(color.r, max(color.g, color.b));
    let inverted = 1.0 - greatest;
    let gamma = color * ((1.0 - inverted * inverted * inverted * inverted) / max(greatest, 0.00001));
    return mix(color, gamma, 0.5);
}

// Minecraft's values are display values, as the target's are.
fn shade(in: Out, texel: vec4<f32>) -> vec4<f32> {
    let lit = texel.rgb * in.colour.rgb * lightmap(in.light.x, in.light.y);
    let fog_end = max(view.origin.w, 16.0);
    let fog = smoothstep(fog_end * 0.75, fog_end, in.distance);
    return vec4<f32>(mix(lit, view.sky.rgb, fog), texel.a * in.colour.a);
}

@fragment
fn opaque(in: Out) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas, atlas_sampler, in.uv);
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(shade(in, texel).rgb, 1.0);
}

@fragment
fn translucent(in: Out) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas, atlas_sampler, in.uv);
    return shade(in, texel);
}

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) height: f32,
}

@vertex
fn sky_vertex(@builtin(vertex_index) index: u32) -> SkyOut {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: SkyOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.height = uv.y;
    return out;
}

@fragment
fn sky_fragment(in: SkyOut) -> @location(0) vec4<f32> {
    return vec4<f32>(mix(view.sky.rgb * 1.05, view.sky.rgb * 0.85, in.height), 1.0);
}
"#;

fn pipelines(
    device: &RenderDevice,
    registry: &ExactPipelineRegistry,
    format: TextureFormat,
    samples: u32,
) -> [RenderPipeline; 3] {
    let shader = unsafe {
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            source: ShaderSource::Wgsl(TERRAIN_WGSL.into()),
        })
    };
    let layout = registry.bind_group_layout(device, &layout());
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("iw4l_minecraft_terrain"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let attributes = [
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 12,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Unorm8x4,
            offset: 20,
            shader_location: 2,
        },
        VertexAttribute {
            format: VertexFormat::Unorm8x2,
            offset: 24,
            shader_location: 3,
        },
    ];
    let options = PipelineCompilationOptions {
        constants: &[],
        zero_initialize_workgroup_memory: false,
    };
    let buffers = [RawVertexBufferLayout {
        array_stride: MINECRAFT_VERTEX_BYTES,
        step_mode: VertexStepMode::Vertex,
        attributes: &attributes,
    }];
    let make = |vertex: &str, fragment: &str, buffers: &[RawVertexBufferLayout], depth_write: bool, compare: CompareFunction, blend: Option<BlendState>| {
        device.create_render_pipeline(&RawRenderPipelineDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            layout: Some(&pipeline_layout),
            vertex: RawVertexState {
                module: &shader,
                entry_point: Some(vertex),
                buffers,
                compilation_options: options.clone(),
            },
            fragment: Some(RawFragmentState {
                module: &shader,
                entry_point: Some(fragment),
                targets: &[Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: options.clone(),
            }),
            primitive: PrimitiveState {
                cull_mode: None,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: SCENE_DEPTH_FORMAT,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(compare),
                stencil: default(),
                bias: default(),
            }),
            multisample: MultisampleState {
                count: samples,
                ..default()
            },
            multiview_mask: None,
            cache: None,
        })
    };
    [
        make("sky_vertex", "sky_fragment", &[], false, CompareFunction::Equal, None),
        make("vertex", "opaque", &buffers, true, CompareFunction::GreaterEqual, None),
        make(
            "vertex",
            "translucent",
            &buffers,
            false,
            CompareFunction::GreaterEqual,
            Some(BlendState::ALPHA_BLENDING),
        ),
    ]
}
