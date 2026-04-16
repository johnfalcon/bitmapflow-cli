mod interpolation;
mod spritesheet;
mod types;

use std::io::{self, Write};

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

use crate::types::{FlowAlg, InterpolationParams};

/// CLI tool for generating animation inbetween frames using optical flow.
///
/// Takes a spritesheet PNG as input, generates interpolated inbetween frames,
/// and outputs a new spritesheet PNG with the additional frames.
#[derive(Parser, Debug)]
#[command(name = "bitmapflow-cli", version, about)]
struct Cli {
    /// Path to the input spritesheet PNG
    #[arg(short, long)]
    input: String,

    /// Path for the output spritesheet PNG
    #[arg(short, long)]
    output: String,

    /// Width of each frame in pixels
    #[arg(long)]
    frame_width: u32,

    /// Height of each frame in pixels
    #[arg(long)]
    frame_height: u32,

    /// Number of frames in the input spritesheet
    #[arg(long)]
    frame_count: usize,

    /// Number of inbetween frames to generate between each pair of frames
    #[arg(long, default_value = "1")]
    inbetweens: usize,

    /// Number of columns in the output spritesheet
    #[arg(long)]
    columns: Option<usize>,

    /// Make the animation loop seamlessly (adds inbetweens between last and first frame)
    #[arg(long, default_value = "false")]
    r#loop: bool,

    /// Motion multiplier (>1 exaggerates motion, <1 diminishes it)
    #[arg(long, default_value = "1.0")]
    motion_multiplier: f32,

    /// Optical flow algorithm: simpleflow or denserlof
    #[arg(long, default_value = "simpleflow")]
    algorithm: String,

    // -- SimpleFlow parameters --
    /// SimpleFlow: number of layers
    #[arg(long, default_value = "3")]
    layers: usize,

    /// SimpleFlow: averaging block size
    #[arg(long, default_value = "2")]
    averaging_block_size: usize,

    /// SimpleFlow: max flow
    #[arg(long, default_value = "4")]
    max_flow: usize,

    // -- DenseRLOF parameters --
    /// DenseRLOF: forward-backward threshold
    #[arg(long, default_value = "1.0")]
    forward_backward_threshold: f32,

    /// DenseRLOF: grid step X
    #[arg(long, default_value = "6")]
    grid_step_x: i32,

    /// DenseRLOF: grid step Y
    #[arg(long, default_value = "6")]
    grid_step_y: i32,

    /// DenseRLOF: enable post processing
    #[arg(long, default_value = "true")]
    use_post_proc: bool,

    /// DenseRLOF: enable variational refinement
    #[arg(long, default_value = "true")]
    use_variational_refinement: bool,

    /// Output results as JSON (for machine consumption)
    #[arg(long, default_value = "false")]
    json: bool,
}

#[derive(Serialize)]
struct JsonOutput {
    success: bool,
    input: String,
    output: String,
    input_frame_count: usize,
    output_frame_count: usize,
    frame_width: u32,
    frame_height: u32,
    output_columns: usize,
    algorithm: String,
    inbetweens: usize,
    looped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn build_flow_alg(cli: &Cli) -> FlowAlg {
    match cli.algorithm.to_lowercase().as_str() {
        "denserlof" | "dense_rlof" | "rlof" => FlowAlg::DenseRLOF {
            forward_backward_threshold: cli.forward_backward_threshold,
            grid_step_x: cli.grid_step_x,
            grid_step_y: cli.grid_step_y,
            use_post_proc: cli.use_post_proc,
            use_variational_refinement: cli.use_variational_refinement,
        },
        _ => FlowAlg::SimpleFlow {
            layers: cli.layers,
            averaging_block_size: cli.averaging_block_size,
            max_flow: cli.max_flow,
        },
    }
}

fn run(cli: &Cli) -> Result<JsonOutput> {
    let flow_alg = build_flow_alg(cli);
    let algorithm_name = match &flow_alg {
        FlowAlg::SimpleFlow { .. } => "SimpleFlow",
        FlowAlg::DenseRLOF { .. } => "DenseRLOF",
    };

    if !cli.json {
        eprintln!(
            "Loading spritesheet: {} ({}x{}, {} frames)",
            cli.input, cli.frame_width, cli.frame_height, cli.frame_count
        );
    }

    let input_frames = spritesheet::load_spritesheet(
        &cli.input,
        cli.frame_width,
        cli.frame_height,
        cli.frame_count,
    )
    .context("Failed to load input spritesheet")?;

    let params = InterpolationParams {
        inbetweens: cli.inbetweens,
        loop_seamlessly: cli.r#loop,
        flow_multiplier: cli.motion_multiplier,
        flow_alg,
    };

    if !cli.json {
        eprintln!(
            "Interpolating with {} ({} inbetweens, loop={})",
            algorithm_name, cli.inbetweens, cli.r#loop
        );
    }

    let on_progress: Box<dyn Fn(f64)> = if cli.json {
        Box::new(|_| {})
    } else {
        Box::new(|progress: f64| {
            eprint!("\rProgress: {:.0}%", progress * 100.0);
            io::stderr().flush().ok();
        })
    };

    let output_frames =
        interpolation::interpolate(&input_frames, &params, &on_progress)
            .context("Interpolation failed")?;

    if !cli.json {
        eprintln!(); // newline after progress
    }

    let output_frame_count = output_frames.len();
    let columns = cli.columns.unwrap_or(output_frame_count);

    if !cli.json {
        eprintln!(
            "Saving {} frames to: {} ({} columns)",
            output_frame_count, cli.output, columns
        );
    }

    spritesheet::save_spritesheet(&output_frames, &cli.output, columns)
        .context("Failed to save output spritesheet")?;

    Ok(JsonOutput {
        success: true,
        input: cli.input.clone(),
        output: cli.output.clone(),
        input_frame_count: cli.frame_count,
        output_frame_count,
        frame_width: cli.frame_width,
        frame_height: cli.frame_height,
        output_columns: columns,
        algorithm: algorithm_name.to_string(),
        inbetweens: cli.inbetweens,
        looped: cli.r#loop,
        error: None,
    })
}

fn main() {
    let cli = Cli::parse();
    let is_json = cli.json;

    match run(&cli) {
        Ok(output) => {
            if is_json {
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                eprintln!("Done! Generated {} frames.", output.output_frame_count);
            }
        }
        Err(err) => {
            if is_json {
                let error_output = JsonOutput {
                    success: false,
                    input: cli.input.clone(),
                    output: cli.output.clone(),
                    input_frame_count: cli.frame_count,
                    output_frame_count: 0,
                    frame_width: cli.frame_width,
                    frame_height: cli.frame_height,
                    output_columns: 0,
                    algorithm: cli.algorithm.clone(),
                    inbetweens: cli.inbetweens,
                    looped: cli.r#loop,
                    error: Some(format!("{:#}", err)),
                };
                println!("{}", serde_json::to_string_pretty(&error_output).unwrap());
            } else {
                eprintln!("Error: {:#}", err);
            }
            std::process::exit(1);
        }
    }
}
