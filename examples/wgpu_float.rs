#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! A detached 3D viewport: the application renders a wgpu scene (a spinning triangle) in a
//! tab, and the tab can be dragged out into a native window of its own, or back.
//!
//! egui_stack never makes a window or a rendering surface. The application hosts the native
//! windows through its own [`ViewportHost`], whose viewport ids are stable, so it can pair
//! anything with a window: here each window's tint, kept in egui's memory under its viewport
//! id. The scene itself is drawn by the tab, with an `egui_wgpu` paint callback on the wgpu
//! device eframe shares between all its viewports; it draws the same in the main window and
//! in a float.
//!
//! Drag the "3D view" tab out of the window (or use "Eject" in its menu); close its window
//! and the tab comes back.

use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};
use eframe::{NativeOptions, Renderer};
use egui::{
    Color32, Context, Id, Sense, Ui, ViewportBuilder, ViewportClass, ViewportId, WidgetText,
};
use egui_stack::layout::{Content, Dir, Float, FloatHost, Layout, LayoutArea, Size, Zone};
use egui_stack::layout::{ZoneName, ZoneViewer};
use egui_stack::{DockArea, DockRules, DockSpace, Docked, TabViewer, ViewportHost};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Detached 3D viewport",
        NativeOptions {
            renderer: Renderer::Wgpu,
            ..NativeOptions::default()
        },
        Box::new(|cc| {
            let render_state = cc
                .wgpu_render_state
                .as_ref()
                .ok_or("this example needs eframe's wgpu renderer")?;
            Scene::install(render_state);
            Ok(Box::new(App::new()))
        }),
    )
}

const SCENE: &str = "3D view";

struct App {
    docked: Docked<String>,
    host: Windows,
}

impl App {
    fn new() -> Self {
        let mut space = DockSpace::new().with_float_host(FloatHost::Native);
        space.add_dock(
            "centre",
            DockRules::default(),
            [SCENE, "Notes"].map(str::to_owned).to_vec(),
        );
        space.add_dock(
            "right",
            DockRules::default(),
            ["Inspector"].map(str::to_owned).to_vec(),
        );
        let layout = Layout::new(Zone::split(
            "root",
            Dir::Vertical,
            vec![
                Zone::split(
                    "main",
                    Dir::Horizontal,
                    vec![
                        Zone::leaf("centre", Content::Dock("centre".into()))
                            .with(|c| c.grow = true),
                        Zone::leaf("right", Content::Dock("right".into()))
                            .with(|c| c.default = Some(Size::Points(220.0))),
                    ],
                )
                .with(|c| c.grow = true),
                Zone::leaf("status", Content::App).with(|c| {
                    c.min = Some(Size::Points(28.0));
                    c.max = Some(Size::Points(28.0));
                }),
            ],
        ));
        Self {
            docked: Docked::new(layout, space),
            host: Windows::default(),
        }
    }
}

/// The application's native windows: egui_stack asks it for each float's viewport.
#[derive(Default)]
struct Windows {
    /// The windows shown in the last frame.
    shown: Vec<ViewportId>,
    /// Windows made so far, for their tints.
    made: usize,
}

const TINTS: [Color32; 4] = [
    Color32::from_rgb(90, 170, 250),
    Color32::from_rgb(120, 210, 120),
    Color32::from_rgb(230, 110, 200),
    Color32::from_rgb(240, 200, 80),
];

/// Where a window's tint is kept, in egui's memory, under its viewport id.
fn tint_id(viewport: ViewportId) -> Id {
    Id::unique(("wgpu_float.tint", viewport))
}

impl ViewportHost for Windows {
    /// The application's own ids, stable for as long as the float is open: anything paired
    /// with one (a tint here, a second surface or display settings elsewhere) keeps working
    /// while the window moves, changes screen or takes other tabs.
    fn viewport_id(&self, key: &str) -> ViewportId {
        ViewportId::from_hash_of(("wgpu_float.window", key))
    }

    fn builder(&mut self, float: &Float) -> ViewportBuilder {
        ViewportBuilder::default()
            .with_title(format!("Detached: {}", float.key))
            .with_min_inner_size([240.0, 180.0])
    }

    fn show(
        &mut self,
        ctx: &Context,
        id: ViewportId,
        builder: ViewportBuilder,
        body: &mut dyn FnMut(&mut Ui),
    ) {
        // The first time a window is seen, the application pairs a tint with it.
        let known = ctx.data(|d| d.get_temp::<Color32>(tint_id(id)).is_some());
        if !known {
            let tint = TINTS[self.made % TINTS.len()];
            self.made += 1;
            ctx.data_mut(|d| d.insert_temp(tint_id(id), tint));
        }
        self.shown.push(id);
        ctx.show_viewport_immediate(id, builder, |ui, class| {
            if class == ViewportClass::EmbeddedWindow {
                ui.small("(embedded: this backend has one window)");
            }
            body(ui);
        });
    }
}

struct Tabs;

impl TabViewer for Tabs {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> Id {
        Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        if tab == SCENE {
            scene(ui);
        } else {
            ui.label(format!(
                "{tab}: drag the \"{SCENE}\" tab out of the window."
            ));
        }
    }
}

/// The 3D view: the application's wgpu drawing, in whichever viewport the tab is.
fn scene(ui: &mut Ui) {
    let viewport = ui.ctx().viewport_id();
    let tint = ui
        .data(|d| d.get_temp::<Color32>(tint_id(viewport)))
        .unwrap_or(Color32::from_rgb(240, 130, 50));
    let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, Color32::from_gray(20));
    let angle = ui.input(|i| i.time) as f32;
    ui.painter().add(egui_wgpu::Callback::new_paint_callback(
        rect,
        Triangle {
            angle,
            aspect: rect.width() / rect.height().max(1.0),
            tint: tint.to_normalized_gamma_f32(),
        },
    ));
    let place = if viewport == ViewportId::ROOT {
        "in the main window".to_owned()
    } else {
        format!("in the application's window {viewport:?}")
    };
    ui.painter().text(
        rect.left_top() + egui::vec2(8.0, 8.0),
        egui::Align2::LEFT_TOP,
        place,
        egui::FontId::proportional(13.0),
        Color32::WHITE,
    );
    // An immediate viewport is drawn in the main window's pass: repainting it repaints all.
    ui.ctx().request_repaint_of(ViewportId::ROOT);
}

/// The scene's GPU resources, made once and kept in egui_wgpu's callback resources.
struct Scene {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
}

/// The uniforms: angle, aspect, two pads, then the tint (32 bytes).
const UNIFORMS: u64 = 32;

const SHADER: &str = r"
struct Uniforms { angle: f32, aspect: f32, pad: vec2<f32>, tint: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Uniforms;

struct Out { @builtin(position) position: vec4<f32>, @location(0) shade: f32 };

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Out {
    let a = u.angle + f32(i) * 2.0943951;
    var out: Out;
    out.position = vec4<f32>(0.7 * cos(a) / max(u.aspect, 1.0), 0.7 * sin(a) * min(u.aspect, 1.0), 0.0, 1.0);
    out.shade = 0.55 + 0.45 * f32(i) / 2.0;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(u.tint.rgb * in.shade, 1.0);
}
";

impl Scene {
    fn install(render_state: &egui_wgpu::RenderState) {
        let device = &render_state.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wgpu_float scene"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wgpu_float scene"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(UNIFORMS),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wgpu_float scene"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wgpu_float scene"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(render_state.target_format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wgpu_float scene"),
            size: UNIFORMS,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu_float scene"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        render_state
            .renderer
            .write()
            .callback_resources
            .insert(Scene {
                pipeline,
                bind_group,
                uniforms,
            });
    }
}

/// One frame of the scene, as a paint callback. eframe renders each viewport on its own, so
/// the one uniform buffer serves the tab wherever it is (one scene tab per window).
struct Triangle {
    angle: f32,
    aspect: f32,
    tint: [f32; 4],
}

impl CallbackTrait for Triangle {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(scene) = resources.get::<Scene>() {
            let values = [
                self.angle,
                self.aspect,
                0.0,
                0.0,
                self.tint[0],
                self.tint[1],
                self.tint[2],
                self.tint[3],
            ];
            let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_ne_bytes()).collect();
            queue.write_buffer(&scene.uniforms, 0, &bytes);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        if let Some(scene) = resources.get::<Scene>() {
            pass.set_pipeline(&scene.pipeline);
            pass.set_bind_group(0, &scene.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

struct Viewer<'a> {
    space: &'a mut DockSpace<String>,
    windows: usize,
}

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        match zone.0.as_str() {
            "status" => {
                ui.small(format!(
                    "Native windows open: {}. Drag the \"{SCENE}\" tab out to detach it.",
                    self.windows
                ));
            }
            dock => DockArea::dock(self.space, dock).show_inside(ui, &mut Tabs),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let windows = self.host.shown.len();
        self.host.shown.clear();
        let laid = LayoutArea::new(&mut self.docked.layout).show_inside(
            ui,
            &mut Viewer {
                space: &mut self.docked.space,
                windows,
            },
        );
        self.docked.space.bind(&laid);
        // The floats, after the docks, in the application's own windows.
        self.docked
            .floats()
            .host(&mut self.host)
            .show(ui.ctx(), &mut Tabs);
        self.docked.space.take_float_events();
    }
}
