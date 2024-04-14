// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

use {
    anyhow::{anyhow, bail, Context, Result},
    std::{fmt, time::Duration},
    vello::{
        kurbo::{Affine, Vec2},
        util::RenderContext,
    },
};

const SAMPLE_COUNT: usize = 300;
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
struct SceneQueryResults {
    samples: Vec<Vec<wgpu_profiler::GpuTimerQueryResult>>,
}

#[derive(Debug)]
struct Stats {
    flatten_deltas: Vec<f64>,
    min: f64,
    max: f64,
    median: f64,
    mean: f64,
}

impl Bench {
    async fn new() -> Result<Self> {
        let mut context =
            RenderContext::new().or_else(|_| bail!("failed to initialize render context"))?;
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
        Ok(Bench {
            context,
            dev,
            renderer,
            render_target,
        })
    }

    fn sample(
        &mut self,
        scene_to_sample: &mut scenes::ExampleScene,
        count: usize,
    ) -> Result<SceneQueryResults> {
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
            }
            None => Affine::IDENTITY,
        };

        let mut scene = vello::Scene::new();
        scene.append(&fragment, Some(transform));

        let render_params = vello::RenderParams {
            base_color: scene_params
                .base_color
                .unwrap_or(vello::peniko::Color::BLACK),
            width: WIDTH,
            height: HEIGHT,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.sample_scene(&scene, &render_params, count)
    }

    fn sample_scene(
        &mut self,
        scene: &vello::Scene,
        params: &vello::RenderParams,
        count: usize,
    ) -> Result<SceneQueryResults> {
        let view = self
            .render_target
            .create_view(&wgpu::TextureViewDescriptor::default());
        let device = &self.context.devices[self.dev].device;
        let queue = &self.context.devices[self.dev].queue;
        let mut samples = vec![];
        for _ in 0..count {
            self.renderer
                .render_to_texture(device, queue, scene, &view, params)
                .or_else(|e| bail!("failed to render scene {:?}", e))?;
            device.poll(wgpu::Maintain::Wait);
            let timer_query_result = self
                .renderer
                .profiler
                .process_finished_frame(queue.get_timestamp_period());
            let result =
                timer_query_result.ok_or_else(|| anyhow!("no timer query was recorded"))?;
            samples.push(result);
        }
        Ok(SceneQueryResults { samples })
    }
}

impl Scenes {
    fn new() -> Self {
        Scenes {
            tests: scenes::test_scenes(),
        }
    }
}

impl SceneQueryResults {
    fn analyze(&self) -> Stats {
        let mut deltas = vec![];
        let mut min = std::f64::MAX;
        let mut max = std::f64::MIN;
        let mut mean = 0.;
        for sample in &self.samples {
            for query in sample {
                // When TIMESTAMP_QUERY_INSIDE_PASSES is supported:
                // TODO: this could process stages other than "flatten"
                let query = if !query.nested_queries.is_empty() {
                    let mut flatten = None;
                    for nq in &query.nested_queries {
                        if nq.label == "flatten" {
                            flatten = Some(nq);
                        }
                    }
                    flatten
                } else if query.label == "flatten" {
                    Some(query)
                } else {
                    None
                };
                let Some(query) = query else {
                    continue;
                };
                let delta = query.time.end - query.time.start;
                if delta < min {
                    min = delta;
                }
                if delta > max {
                    max = delta;
                }
                deltas.push(delta);
                mean += delta / self.samples.len() as f64;
            }
        }
        let sorted_deltas = {
            let mut sortable = deltas.iter().map(|f| SortableFloat(*f)).collect::<Vec<_>>();
            sortable.sort();
            sortable
        };
        Stats {
            flatten_deltas: deltas,
            min,
            max,
            median: sorted_deltas[sorted_deltas.len() / 2].0,
            mean,
        }
    }
}

#[derive(PartialEq, PartialOrd, Copy, Clone)]
struct SortableFloat(f64);

impl std::cmp::Eq for SortableFloat {}

impl std::cmp::Ord for SortableFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}

const BARS: [&'static str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

impl Stats {
    fn plot(&self) -> String {
        let mut plot = String::new();
        for delta in &self.flatten_deltas {
            if self.min == self.max {
                plot.push_str(BARS[0]);
                continue;
            }
            let s = (delta - self.min) / (self.max - self.min);
            let s = s * (BARS.len() as f64 - 1.);
            plot.push_str(BARS[(s + 0.5) as usize]);
        }
        plot
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:.2?},{:.2?},{:.2?},{:.2?},{}",
            Duration::from_secs_f64(self.mean),
            Duration::from_secs_f64(self.median),
            Duration::from_secs_f64(self.min),
            Duration::from_secs_f64(self.max),
            self.plot()
        )
    }
}

#[pollster::main]
async fn main() -> Result<()> {
    let mut bench = Bench::new().await?;
    let mut scenes = Scenes::new();
    println!("samples: {}", SAMPLE_COUNT);
    println!("test,mean,median,min,max,plot");
    for scene in &mut scenes.tests.scenes {
        let samples = bench.sample(scene, SAMPLE_COUNT)?;
        let stats = samples.analyze();
        println!("{},{}", scene.config.name, stats);
    }
    Ok(())
}
