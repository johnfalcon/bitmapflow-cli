use image::RgbaImage;
use serde::{Deserialize, Serialize};

/// A single animation frame wrapping an RGBA image.
#[derive(Clone, Debug)]
pub struct Frame(pub RgbaImage);

impl std::ops::Deref for Frame {
    type Target = RgbaImage;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Optical flow algorithm selection with parameters.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "algorithm")]
pub enum FlowAlg {
    SimpleFlow {
        layers: usize,
        averaging_block_size: usize,
        max_flow: usize,
    },
    DenseRLOF {
        forward_backward_threshold: f32,
        grid_step_x: i32,
        grid_step_y: i32,
        use_post_proc: bool,
        use_variational_refinement: bool,
    },
}

impl Default for FlowAlg {
    fn default() -> Self {
        FlowAlg::SimpleFlow {
            layers: 3,
            averaging_block_size: 2,
            max_flow: 4,
        }
    }
}

/// Parameters controlling the interpolation process.
#[derive(Clone, Debug)]
pub struct InterpolationParams {
    pub inbetweens: usize,
    pub loop_seamlessly: bool,
    pub flow_multiplier: f32,
    pub flow_alg: FlowAlg,
}
