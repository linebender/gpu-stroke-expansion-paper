// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

use {
    anyhow::{anyhow, bail, Context, Result},
    vello::{
        kurbo::{Affine, Vec2},
        util::RenderContext,
    },
};

const WIDTH: u32 = 1000;
const HEIGHT: u32 = 1000;

struct Bench {
    context: RenderContext,
    dev: usize,
    renderer: vello::Renderer,
    render_target: wgpu::Texture,
}

struct Scenes {
    tests: scenes::SceneSet,
}

#[derive(Debug)]
struct SceneSamples {
    samples: Vec<Vec<wgpu_profiler::GpuTimerQueryResult>>,
}

impl Bench {
    async fn new() -> Result<Self> {
        let mut context = RenderContext::new()
            .or_else(|_| bail!("failed to initialize render context"))?;
        let dev = context
            .device(None)
            .await
            .ok_or_else(|| anyhow!("failed to initialize device"))?;
        let device = &context.devices[dev].device;
        let mut renderer = vello::Renderer::new(
            device,
            vello::RendererOptions {
                surface_format: None,
                use_cpu: false,
                num_init_threads: std::num::NonZeroUsize::new(1),
                antialiasing_support: vello::AaSupport::area_only(),
            },
        )
        .or_else(|_| bail!("failed to initialize renderer"))?;
        renderer
            .profiler
            .change_settings(wgpu_profiler::GpuProfilerSettings {
                enable_timer_queries: true,
                enable_debug_groups: true,
                ..Default::default()
            })
            .context("failed to enable timer queries")?;
        let render_target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target texture"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        Ok(Bench { context, dev, renderer, render_target })
    }

    fn sample(&mut self, scene_to_sample: &mut scenes::ExampleScene, count: usize) -> Result<SceneSamples> {
        // TODO: sample CPU encoding and end-to-end render times too.
        let mut text = scenes::SimpleText::new();
        let mut images = scenes::ImageCache::new();
        let mut scene_params = scenes::SceneParams {
            time: 0.,
            text: &mut text,
            images: &mut images,
            resolution: None,
            base_color: None,
            interactive: false,
            complexity: 15,
        };
        let mut fragment = vello::Scene::new();
        scene_to_sample
            .function
            .render(&mut fragment, &mut scene_params);

        let transform = match scene_params.resolution {
            Some(res) => {
                let factor = Vec2::new(WIDTH as f64, HEIGHT as f64);
                let scale_factor = (factor.x / res.x).min(factor.y / res.y);
                Affine::scale(scale_factor)
            },
            None => Affine::IDENTITY,
        };

        let mut scene = vello::Scene::new();
        scene.append(&fragment, Some(transform));

        let render_params = vello::RenderParams {
            base_color: scene_params.base_color.unwrap_or(vello::peniko::Color::BLACK),
            width: WIDTH,
            height: HEIGHT,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.sample_scene(&scene, &render_params, count)
    }

    fn sample_scene(&mut self, scene: &vello::Scene, params: &vello::RenderParams, count: usize) -> Result<SceneSamples> {
        let view = self.render_target.create_view(&wgpu::TextureViewDescriptor::default());
        let device = &self.context.devices[self.dev].device;
        let queue = &self.context.devices[self.dev].queue;
        let mut samples = vec![];
        self.renderer
            .render_to_texture(device, queue, scene, &view, params)
            .or_else(|e| bail!("failed to render scene {:?}", e))?;
        device.poll(wgpu::Maintain::Wait);
        let timer_query_result = self
            .renderer
            .profiler
            .process_finished_frame(queue.get_timestamp_period());//profile_result.take();
        let sample = timer_query_result.ok_or_else(|| anyhow!("no timer query was recorded"))?;
        samples.push(sample);
        Ok(SceneSamples { samples })
    }
}

impl Scenes {
    fn new() -> Self {
        Scenes {
            tests: scenes::test_scenes(),
        }
    }
}

#[pollster::main]
async fn main() -> Result<()> {
    let mut bench = Bench::new().await?;
    let mut scenes = Scenes::new();
    let samples = bench.sample(&mut scenes.tests.scenes[0], 1)?;
    println!("{:?}", samples);
    Ok(())
}
