// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod anims;
mod mmark;
mod text;

use anims::Anims;
use anyhow::Result;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::dpi::LogicalSize;
use winit::event::*;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::Window;

use vello::wgpu;
// Simple struct to hold the state of the renderer
pub struct ActiveRenderState<'s> {
    // The fields MUST be in this order, so that the surface is dropped before the window
    surface: RenderSurface<'s>,
    window: Arc<Window>,
}

enum RenderState<'s> {
    Active(ActiveRenderState<'s>),
    // Cache a window so that it can be reused when the app is resumed after being suspended
    Suspended(Option<Arc<Window>>),
}

fn main() -> Result<()> {
    let start_time = Instant::now();
    let mut anims = Anims::new();
    // Setup a bunch of state:

    // The vello RenderContext which is a global context that lasts for the lifetime of the application
    let mut render_cx = RenderContext::new();

    // An array of renderers, one per wgpu device
    let mut renderers: Vec<Option<Renderer>> = vec![];

    // State for our example where we store the winit Window and the wgpu Surface
    let mut render_state = RenderState::Suspended(None);

    // A vello Scene which is a data structure which allows one to build up a description a scene to be drawn
    // (with paths, fills, images, text, etc) which is then passed to a renderer for rendering
    let mut scene = Scene::new();

    // Create and run a winit event loop
    let event_loop = EventLoop::new()?;
    #[allow(deprecated)]
    event_loop
        .run(move |event, event_loop| match event {
            // Setup renderer. In winit apps it is recommended to do setup in Event::Resumed
            // for best cross-platform compatibility
            Event::Resumed => {
                let RenderState::Suspended(cached_window) = &mut render_state else {
                    return;
                };

                // Get the winit window cached in a previous Suspended event or else create a new window
                let window = cached_window
                    .take()
                    .unwrap_or_else(|| create_winit_window(event_loop));

                // Create a vello Surface
                let size = window.inner_size();
                let surface_future = render_cx.create_surface(
                    window.clone(),
                    size.width,
                    size.height,
                    wgpu::PresentMode::AutoVsync,
                );
                let surface = pollster::block_on(surface_future).expect("Error creating surface");

                // Create a vello Renderer for the surface (using its device id)
                renderers.resize_with(render_cx.devices.len(), || None);
                renderers[surface.dev_id]
                    .get_or_insert_with(|| create_vello_renderer(&render_cx, &surface));

                // Save the Window and Surface to a state variable
                render_state = RenderState::Active(ActiveRenderState { window, surface });

                event_loop.set_control_flow(ControlFlow::Poll);
            }

            // Save window state on suspend
            Event::Suspended => {
                if let RenderState::Active(state) = &render_state {
                    render_state = RenderState::Suspended(Some(state.window.clone()));
                }
                event_loop.set_control_flow(ControlFlow::Wait);
            }

            Event::WindowEvent {
                ref event,
                window_id,
            } => {
                // Ignore the event (return from the function) if
                //   - we have no render_state
                //   - OR the window id of the event doesn't match the window id of our render_state
                //
                // Else extract a mutable reference to the render state from its containing option for use below
                let render_state = match &mut render_state {
                    RenderState::Active(state) if state.window.id() == window_id => state,
                    _ => return,
                };

                match event {
                    // Exit the event loop when a close is requested (e.g. window's close button is pressed)
                    WindowEvent::CloseRequested => event_loop.exit(),

                    // Resize the surface when the window is resized
                    WindowEvent::Resized(size) => {
                        render_cx.resize_surface(
                            &mut render_state.surface,
                            size.width,
                            size.height,
                        );
                        render_state.window.request_redraw();
                    }

                    // This is where all the rendering happens
                    WindowEvent::RedrawRequested => {
                        // Empty the scene of objects to draw. You could create a new Scene each time, but in this case
                        // the same Scene is reused so that the underlying memory allocation can also be reused.
                        scene.reset();

                        let t = start_time.elapsed().as_secs_f64();
                        // TODO: stabilize for steady frame rates
                        // Re-add the objects to draw to the scene.
                        anims.render(&mut scene, t);

                        // Get the RenderSurface (surface + config)
                        let surface = &render_state.surface;

                        // Get the window size
                        let width = surface.config.width;
                        let height = surface.config.height;

                        // Get a handle to the device
                        let device_handle = &render_cx.devices[surface.dev_id];

                        // Get the surface's texture
                        let surface_texture = surface
                            .surface
                            .get_current_texture()
                            .expect("failed to get surface texture");

                        // Render to the surface's texture
                        renderers[surface.dev_id]
                            .as_mut()
                            .unwrap()
                            .render_to_surface(
                                &device_handle.device,
                                &device_handle.queue,
                                &scene,
                                &surface_texture,
                                &vello::RenderParams {
                                    base_color: Color::WHITE, // Background color
                                    width,
                                    height,
                                    antialiasing_method: AaConfig::Msaa16,
                                },
                            )
                            .expect("failed to render to surface");

                        // Queue the texture to be presented on the surface
                        surface_texture.present();

                        device_handle.device.poll(wgpu::Maintain::Poll);
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                if let RenderState::Active(state) = &render_state {
                    state.window.request_redraw();
                }
            }
            _ => {}
        })
        .expect("Couldn't run event loop");
    Ok(())
}

/// Helper function that creates a Winit window and returns it (wrapped in an Arc for sharing between threads)
fn create_winit_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let attr = Window::default_attributes()
        .with_inner_size(LogicalSize::new(1044, 800))
        .with_resizable(true)
        .with_title("Vello Shapes");
    Arc::new(event_loop.create_window(attr).unwrap())
}

/// Helper function that creates a vello `Renderer` for a given `RenderContext` and `RenderSurface`
fn create_vello_renderer(render_cx: &RenderContext, surface: &RenderSurface) -> Renderer {
    Renderer::new(
        &render_cx.devices[surface.dev_id].device,
        RendererOptions {
            surface_format: Some(surface.format),
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: NonZeroUsize::new(1),
        },
    )
    .expect("Couldn't create renderer")
}
