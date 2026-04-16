use anyhow::{Context, Result};
use image::RgbaImage;
use opencv::{
    core::{Mat, MatTraitConst, Size2i, Vec2f, Vec3b, CV_32FC2, CV_8UC3},
    optflow,
    prelude::*,
};

use crate::types::{FlowAlg, Frame, InterpolationParams};

/// Convert a Frame to an OpenCV BGR Mat (discards alpha).
fn frame_to_mat(frame: &Frame) -> Mat {
    let w = frame.width();
    let h = frame.height();
    let mut mat = Mat::new_rows_cols_with_default(h as i32, w as i32, CV_8UC3, 0.into()).unwrap();

    for y in 0..h {
        for x in 0..w {
            let [r, g, b, a] = frame[(x, y)].0;
            let bgr = if a < 30 {
                Vec3b::from([0, 0, 0])
            } else {
                Vec3b::from([b, g, r])
            };
            *mat.at_2d_mut::<Vec3b>(y as i32, x as i32).unwrap() = bgr;
        }
    }
    mat
}

/// Compute optical flow between two frames.
fn compute_optical_flow(mat_a: &Mat, mat_b: &Mat, flow_alg: &FlowAlg) -> Result<Mat> {
    let mut flow =
        Mat::new_rows_cols_with_default(mat_a.rows(), mat_a.cols(), CV_32FC2, 0.into())?;

    match flow_alg {
        FlowAlg::SimpleFlow {
            layers,
            averaging_block_size,
            max_flow,
        } => {
            optflow::calc_optical_flow_sf(
                mat_a,
                mat_b,
                &mut flow,
                *layers as i32,
                *averaging_block_size as i32,
                *max_flow as i32,
            )
            .context("SimpleFlow optical flow computation failed")?;
        }
        FlowAlg::DenseRLOF {
            forward_backward_threshold,
            grid_step_x,
            grid_step_y,
            use_post_proc,
            use_variational_refinement,
        } => {
            let rlof_param = optflow::RLOFOpticalFlowParameter::create()?;
            let grid_step = Size2i::new(*grid_step_x, *grid_step_y);
            let interp_type = optflow::InterpolationType::INTERP_EPIC;
            optflow::calc_optical_flow_dense_rlof(
                mat_a,
                mat_b,
                &mut flow,
                rlof_param,
                *forward_backward_threshold,
                grid_step,
                interp_type,
                128,
                0.05,
                100.0,
                15,
                100,
                *use_post_proc,
                500.0,
                1.5,
                *use_variational_refinement,
            )
            .context("DenseRLOF optical flow computation failed")?;
        }
    }

    Ok(flow)
}

/// Warp a frame using the computed optical flow field.
fn apply_flow(frame: &Frame, flow: &Mat, flow_multiplier: f32) -> Frame {
    let w = frame.width();
    let h = frame.height();

    let inner = RgbaImage::from_fn(w, h, |x, y| {
        let flow_val: &Vec2f = flow.at_2d(y as i32, x as i32).unwrap();
        let mut fx = flow_val[0];
        let mut fy = flow_val[1];

        if !fx.is_finite() || !fy.is_finite() {
            fx = 0.0;
            fy = 0.0;
        }

        let new_x =
            ((x as f32 - fx * flow_multiplier).round() as i32).clamp(0, (w - 1) as i32) as u32;
        let new_y =
            ((y as f32 - fy * flow_multiplier).round() as i32).clamp(0, (h - 1) as i32) as u32;

        frame[(new_x, new_y)]
    });

    Frame(inner)
}

/// Run the full interpolation pipeline on a list of input frames.
/// Returns the output frames with inbetweens inserted.
pub fn interpolate(
    input_frames: &[Frame],
    params: &InterpolationParams,
    on_progress: &dyn Fn(f64),
) -> Result<Vec<Frame>> {
    let frame_count = input_frames.len();
    if frame_count < 2 {
        anyhow::bail!("Need at least 2 frames for interpolation, got {}", frame_count);
    }

    let inbetweens = params.inbetweens;

    // Build flow multiplier schedule: e.g. for 2 inbetweens -> [0.0, 0.33, 0.66]
    let flow_multipliers: Vec<f32> = (0..=inbetweens)
        .map(|i| i as f32 / (inbetweens + 1) as f32)
        .collect();

    // Build the frame pairs list
    let mut frame_refs: Vec<&Frame> = input_frames.iter().collect();
    if params.loop_seamlessly {
        frame_refs.push(&input_frames[0]);
    } else {
        frame_refs.push(input_frames.last().unwrap());
    }

    let pair_count = frame_refs.len() - 1;
    let total_steps = pair_count * flow_multipliers.len();
    let mut output_frames: Vec<Frame> = Vec::new();
    let mut step = 0;

    on_progress(0.0);

    for window in frame_refs.windows(2) {
        let frame_a = window[0];
        let frame_b = window[1];

        let mat_a = frame_to_mat(frame_a);
        let mat_b = frame_to_mat(frame_b);

        let flow =
            compute_optical_flow(&mat_a, &mat_b, &params.flow_alg).context("Optical flow failed")?;

        for &fm in &flow_multipliers {
            let effective_multiplier = fm * params.flow_multiplier;
            step += 1;
            on_progress(step as f64 / total_steps as f64);

            if effective_multiplier.abs() < f32::EPSILON {
                output_frames.push(frame_a.clone());
            } else {
                output_frames.push(apply_flow(frame_a, &flow, effective_multiplier));
            }
        }
    }

    on_progress(1.0);
    Ok(output_frames)
}
