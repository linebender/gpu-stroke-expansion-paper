// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

use {
    anyhow::{anyhow, bail, Context, Result},
    clap::Parser,
    std::{
        fmt,
        time::{Duration, Instant},
    },
    vello::{
        kurbo::{Affine, Vec2},
        util::RenderContext,
    },
};

const SAMPLE_COUNT: usize = 1000;
const WIDTH: u32 = 2048;
const HEIGHT: u32 = 2048;

struct Bench {
    context: RenderContext,
    dev: usize,
    renderer: vello::Renderer,
    render_target: wgpu::Texture,
}

type GpuTimerQuerySamples = Vec<Vec<wgpu_profiler::GpuTimerQueryResult>>;

#[derive(Debug)]
struct SceneQueryResults {
    prep_time: Duration,
    e2e_samples: Vec<Duration>,
    gpu_samples: GpuTimerQuerySamples,
}

#[derive(Debug)]
struct Stats {
    prep_time: f64,
    deltas: Vec<f64>,
    min: f64,
    max: f64,
    median: f64,
    mean: f64,
}

impl Bench {
    async fn new() -> Result<Self> {
        let mut context = RenderContext::new();
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
        scene: &mut scenes::ExampleScene,
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

        let prep_start_time = Instant::now();
        let mut fragment = vello::Scene::new();
        scene.function.render(&mut fragment, &mut scene_params);

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

        let prep_end_time = Instant::now();
        let (e2e_samples, gpu_samples) = self.sample_scene(&scene, &render_params, count)?;
        Ok(SceneQueryResults {
            prep_time: prep_end_time - prep_start_time,
            e2e_samples,
            gpu_samples,
        })
    }

    fn sample_scene(
        &mut self,
        scene: &vello::Scene,
        params: &vello::RenderParams,
        count: usize,
    ) -> Result<(Vec<Duration>, GpuTimerQuerySamples)> {
        let view = self
            .render_target
            .create_view(&wgpu::TextureViewDescriptor::default());
        let device = &self.context.devices[self.dev].device;
        let queue = &self.context.devices[self.dev].queue;
        let mut timer_query_samples = vec![];
        let mut end_to_end_samples = vec![];
        for _ in 0..count {
            let start_time = Instant::now();
            self.renderer
                .render_to_texture(device, queue, scene, &view, params)
                .or_else(|e| bail!("failed to render scene {:?}", e))?;
            device.poll(wgpu::Maintain::Wait);

            //std::thread::sleep(Duration::from_millis(16));

            let end_time = Instant::now();
            let timer_query_result = self
                .renderer
                .profiler
                .process_finished_frame(queue.get_timestamp_period());
            let result =
                timer_query_result.ok_or_else(|| anyhow!("no timer query was recorded"))?;
            end_to_end_samples.push(end_time - start_time);
            timer_query_samples.push(result);
        }
        Ok((end_to_end_samples, timer_query_samples))
    }
}

impl SceneQueryResults {
    fn analyze(&self, stage: &Option<String>) -> Stats {
        let deltas = match stage {
            Some(label) => {
                let mut deltas = vec![];
                for sample in &self.gpu_samples {
                    //println!("{sample:?}");
                    for query in sample {
                        // When TIMESTAMP_QUERY_INSIDE_PASSES is supported:
                        // TODO: this could process stages other than "flatten"
                        let query = if !query.nested_queries.is_empty() {
                            let mut flatten = None;
                            for nq in &query.nested_queries {
                                if nq.label == *label {
                                    flatten = Some(nq);
                                }
                            }
                            flatten
                        } else if query.label == *label {
                            Some(query)
                        } else {
                            None
                        };
                        let Some(query) = query else {
                            continue;
                        };
                        deltas.push(query.time.end - query.time.start);
                    }
                }
                deltas
            }
            None => self.e2e_samples.iter().map(|d| d.as_secs_f64()).collect(),
        };

        let mut min = std::f64::MAX;
        let mut max = std::f64::MIN;
        let mut mean = 0.;
        for delta in deltas.iter().copied() {
            if delta < min {
                min = delta;
            }
            if delta > max {
                max = delta;
            }
            mean += delta / self.gpu_samples.len() as f64;
        }
        let sorted_deltas = {
            let mut sortable = deltas.iter().map(|f| SortableFloat(*f)).collect::<Vec<_>>();
            sortable.sort();
            sortable
        };
        Stats {
            prep_time: self.prep_time.as_secs_f64(),
            deltas,
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
        for delta in &self.deltas {
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
            "{:.2?},{:.2?},{:.2?},{:.2?},{:.2?},{}",
            Duration::from_secs_f64(self.prep_time),
            Duration::from_secs_f64(self.mean),
            Duration::from_secs_f64(self.median),
            Duration::from_secs_f64(self.min),
            Duration::from_secs_f64(self.max),
            self.plot()
        )
    }
}

fn test_scenes(matches: &Option<String>) -> scenes::SceneSet {
    let filters: Vec<&str> = matches
        .as_ref()
        .map(|v| v.split(",").collect())
        .unwrap_or(vec![]);
    let scenes = scenes::test_scenes();
    return scenes::SceneSet {
        scenes: scenes
            .scenes
            .into_iter()
            .filter(|s| filters.is_empty() || filters.iter().any(|f| s.config.name.contains(f)))
            .collect(),
    };
}

fn svg_scenes(args: &SvgArgs) -> Result<scenes::SceneSet> {
    let filters: Vec<&str> = args
        .matches
        .as_ref()
        .map(|v| v.split(",").collect())
        .unwrap_or(vec![]);
    let mut svg_paths = vec![];
    for file in std::fs::read_dir(&args.directory)? {
        let entry = file?;
        if let Some(extension) = std::path::Path::new(&entry.file_name()).extension() {
            if extension == "svg"
                && (filters.is_empty()
                    || filters
                        .iter()
                        .any(|f| entry.file_name().into_string().unwrap().contains(f)))
            {
                svg_paths.push(entry.path());
            }
        }
    }
    scenes::scene_from_files(&svg_paths)
}

fn benchmark_scenes(
    bench: &mut Bench,
    scenes: &mut scenes::SceneSet,
    stage: &Option<String>,
    suffix: &str,
) -> Result<()> {
    for scene in &mut scenes.scenes {
        let samples = bench.sample(scene, SAMPLE_COUNT)?;
        let stats = samples.analyze(stage);
        println!("{}{},{}", scene.config.name, suffix, stats);
    }
    Ok(())
}

#[derive(Parser)]
struct Cli {
    /// If present, the benchmarks a restricted to just this pipeline stage. Otherwise the timings
    /// include the GPU render time for the entire vello pipeline.
    #[arg(short, long)]
    stage: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    VelloTestScenes(VelloTestScenesArgs),
    Svg(SvgArgs),
}

#[derive(Parser)]
struct VelloTestScenesArgs {
    /// Comma separated list of names to filter on
    #[arg(short, long)]
    matches: Option<String>,
}

#[derive(Parser)]
struct SvgArgs {
    directory: String,

    /// Comma separated list of names to filter on
    #[arg(short, long)]
    matches: Option<String>,
}

#[pollster::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut bench = Bench::new().await?;
    let (mut scenes, suffix) = match cli.command {
        Commands::VelloTestScenes(args) => (test_scenes(&args.matches), ""),
        Commands::Svg(args) => (svg_scenes(&args)?, ".svg"),
    };
    println!("samples: {}", SAMPLE_COUNT);
    println!("test,cpu_encode,mean,median,min,max,plot");
    benchmark_scenes(&mut bench, &mut scenes, &cli.stage, suffix)
}
