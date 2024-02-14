use std::f32::INFINITY;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    window::Window
};
use wgpu::util::DeviceExt;


use log::debug;
use log::error;
use log::info;
use log::warn;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32;3],
    color: [f32;3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}


struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    clear_color: wgpu::Color,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: Window,
    render_pipeline: wgpu::RenderPipeline,
    color_render_pipeline: wgpu::RenderPipeline,
    use_color: bool,
    use_funny: bool,

    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,

    index_buffer: wgpu::Buffer,
    num_indices: u32,
    
    funny_vertex_buffer: wgpu::Buffer,
    funny_num_vertices: u32,

    funny_index_buffer: wgpu::Buffer,
    funny_num_indices: u32,
}



impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web, we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1, 
                mask: !0, 
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("color_shader.wgsl"));

        let color_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1, 
                mask: !0, 
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });        

        surface.configure(&device, &config);
        let clear_color = wgpu::Color::BLACK;

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_vertices = VERTICES.len() as u32;

        let num_indices = INDICES.len() as u32;

        let funny_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(FUNNY_VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let funny_index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(FUNNY_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let funny_num_vertices = VERTICES.len() as u32;

        let funny_num_indices = FUNNY_INDICES.len() as u32;

        Self {
            window,
            surface,
            device,
            queue,
            config,
            clear_color,
            size,
            render_pipeline,
            color_render_pipeline,
            use_color:true,
            use_funny:false,

            vertex_buffer,
            num_vertices,

            index_buffer,
            num_indices,

            funny_vertex_buffer,
            funny_index_buffer,
            funny_num_indices,
            funny_num_vertices,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.clear_color = wgpu::Color {
                    r: position.x as f64 / self.size.width as f64,
                    g: position.y as f64 / self.size.height as f64,
                    b: 1.0,
                    a: 1.0,
                };
                true
            }
            WindowEvent::KeyboardInput { input, ..} => {
                match input.virtual_keycode {
                    Some(VirtualKeyCode::C) => {
                        self.use_color = !self.use_color;
                        true
                    },
                    Some(VirtualKeyCode::F) => {
                        self.use_funny = !self.use_funny;
                        true
                    }
                    _ => false
                }
            }
            _ => false,
        }
    }

    fn update(&mut self) {
        // remove `todo!()`
    }
    

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Block tells the compiler that any variables inside should be dropped when it ends
        // This means that we implicitly drop the borrow of encoder that _render_pass does
        // without calling drop(_render_pass) 
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(self.clear_color),
                            store: wgpu::StoreOp::Store,
                        }
                    })
                ],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        
            if self.use_funny {
                render_pass.set_pipeline(&self.color_render_pipeline);
                render_pass.set_vertex_buffer(0, self.funny_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.funny_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                
                render_pass.draw_indexed(0..self.funny_num_indices, 0, 0..1);
            } else if self.use_color {
                render_pass.set_pipeline(&self.color_render_pipeline);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    
            } else {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                
                render_pass.draw(0..self.num_vertices, 0..1);
    
            }    
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}


const FUNNY_VERTICES: &[Vertex] = &[

    //L
    Vertex { position: [0.05, -0.10, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.15, -0.10, 0.0], color: [1.0, 1.0, 0.5] },
    Vertex { position: [0.15, -0.75, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.25, -0.75, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.25, -0.85, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.05, -0.85, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.05, -0.75, 0.0], color: [1.0, 0.0, 0.0] },

    //O
    Vertex { position: [0.40, -0.30, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.40, -0.20, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.60, -0.20, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.60, -0.30, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.70, -0.30, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.70, -0.70, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.60, -0.70, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.60, -0.80, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.40, -0.80, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.40, -0.70, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.30, -0.70, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.30, -0.30, 0.0], color: [1.0, 0.0, 0.0] },

    //L2
    Vertex { position: [0.05 + 0.7, -0.10, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.15 + 0.7, -0.10, 0.0], color: [1.0, 1.0, 0.5] },
    Vertex { position: [0.15 + 0.7, -0.75, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.25 + 0.7, -0.75, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.25 + 0.7, -0.85, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.05 + 0.7, -0.85, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.05 + 0.7, -0.75, 0.0], color: [1.0, 0.0, 0.0] },

];

const FUNNY_INDICES: &[u16] = &[
    //L
    6, 1, 0,
    6, 2, 1,
    3, 6, 5,
    5, 4, 3,

    9,8,7,
    7,10,9,
    12,11,10,
    10,13,12,
    13,15,14,
    13,16,15,
    16,18,17,
    16,7,18,

    //L2
    19,21,20,
    19,25,21,
    25,23,22,
    25,24,23,
    ];

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [1.0, 1.0, 0.5] },
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.0, 1.0, 1.0] },
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.0, 0.0, 1.0] },
];


const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

     // State::new uses async code, so we're going to wait for it to finish
     let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => {
                            warn!("Escape key pressed");
                            *control_flow = ControlFlow::Exit
                        },
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {warn!("Some other event")}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,

                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::RedrawEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                state.window().request_redraw();
            }
            _ => {}
        }
    });
}