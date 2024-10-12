use crate::{
    compute_sand::SandComputePipeline, render::Renderer, render_pass::RenderPassPlaceOverFrame,
};

pub struct RenderState {}

pub struct RenderPipeline {
    pub compute: SandComputePipeline,
    pub place_over_frame: RenderPassPlaceOverFrame,
}

impl RenderPipeline {
    pub fn new(renderer: &Renderer) -> RenderPipeline {
        RenderPipeline {
            compute: SandComputePipeline::new(renderer),
            place_over_frame: RenderPassPlaceOverFrame::new(renderer),
        }
    }
}
