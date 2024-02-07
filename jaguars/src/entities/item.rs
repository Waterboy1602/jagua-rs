use std::sync::Arc;

use crate::collision_detection::hazard_filters::qz_haz_filter::QZHazardFilter;
use crate::geometry::geo_enums::AllowedRotation;
use crate::geometry::primitives::simple_polygon::SimplePolygon;
use crate::geometry::transformation::Transformation;
use crate::util::config::SPSurrogateConfig;

#[derive(Clone, Debug)]
pub struct Item {
    id: usize,
    shape: Arc<SimplePolygon>,
    allowed_rotation: AllowedRotation,
    base_quality: Option<usize>,
    value: u64,
    centering_transform: Transformation,
    hazard_filter: Option<QZHazardFilter>,
}

impl Item {
    pub fn new(id: usize, mut shape: SimplePolygon, value: u64, allowed_rotation: AllowedRotation,
               centering_transform: Transformation, base_quality: Option<usize>, surrogate_config: SPSurrogateConfig) -> Item {
        shape.generate_surrogate(surrogate_config);
        let shape = Arc::new(shape);
        let hazard_filter = base_quality.map(|q| QZHazardFilter { cutoff_quality: q });
        Item { id, shape, allowed_rotation, base_quality, value, centering_transform, hazard_filter }
    }

    pub fn clone_with_id(&self, id: usize) -> Item {
        Item {
            id,
            ..self.clone()
        }
    }

    pub fn shape(&self) -> &SimplePolygon {
        &self.shape
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn centering_transform(&self) -> &Transformation {
        &self.centering_transform
    }

    pub fn base_quality(&self) -> Option<usize> {
        self.base_quality
    }

    pub fn hazard_filter(&self) -> Option<&QZHazardFilter> {
        self.hazard_filter.as_ref()
    }

    pub fn allowed_rotation(&self) -> &AllowedRotation {
        &self.allowed_rotation
    }
}