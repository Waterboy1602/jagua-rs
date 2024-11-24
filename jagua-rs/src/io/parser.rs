use std::sync::Arc;
use std::time::Instant;
use std::path::{Path, PathBuf};

use itertools::Itertools;
use log::{log, Level};
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;

use dxf::Drawing;
use dxf::entities::*;

use crate::entities::bin::Bin;
use crate::entities::instances::bin_packing::BPInstance;
use crate::entities::instances::instance::Instance;
use crate::entities::instances::instance_generic::InstanceGeneric;
use crate::entities::instances::strip_packing::SPInstance;
use crate::entities::item::Item;
use crate::entities::placing_option::PlacingOption;
use crate::entities::problems::bin_packing::BPProblem;
use crate::entities::problems::problem_generic::{LayoutIndex, ProblemGeneric, STRIP_LAYOUT_IDX};
use crate::entities::problems::strip_packing::SPProblem;
use crate::entities::quality_zone::InferiorQualityZone;
use crate::entities::quality_zone::N_QUALITIES;
use crate::entities::solution::Solution;
use crate::fsize;
use crate::geometry::d_transformation::DTransformation;
use crate::geometry::geo_enums::AllowedRotation;
use crate::geometry::geo_traits::{Shape, Transformable};
use crate::geometry::primitives::aa_rectangle::AARectangle;
use crate::geometry::primitives::point::Point;
use crate::geometry::primitives::simple_polygon::SimplePolygon;
use crate::geometry::transformation::Transformation;
use crate::io::json_instance::{JsonBin, JsonInstance, JsonItem, JsonShape, JsonSimplePoly};
use crate::io::json_solution::{
    JsonContainer, JsonLayout, JsonLayoutStats, JsonPlacedItem, JsonSolution, JsonTransformation,
};
use crate::io::dxf_instance::DxfInstance;
use crate::util::config::CDEConfig;
use crate::util::polygon_simplification;
use crate::util::polygon_simplification::{PolySimplConfig, PolySimplMode};


/// Parses a `JsonInstance` into an `Instance`.
pub struct Parser {
    poly_simpl_config: PolySimplConfig,
    cde_config: CDEConfig,
    center_polygons: bool,
    path_assets_folder: PathBuf,
}

impl Parser {
    pub fn new(
        poly_simpl_config: PolySimplConfig,
        cde_config: CDEConfig,
        center_polygons: bool,
        path_assets_folder: PathBuf,
    ) -> Parser {
        Parser {
            poly_simpl_config,
            cde_config,
            center_polygons,
            path_assets_folder,
        }
    }

    /// Parses a `JsonInstance` into an `Instance`.
    pub fn parse(&self, json_instance: &JsonInstance) -> Instance {

        let items = json_instance
            .items
            .par_iter()
            .enumerate()
            .map(|(item_id, json_item)| self.parse_item(json_item, item_id, &self.path_assets_folder))
            .collect();

        let instance: Instance = match (json_instance.bins.as_ref(), json_instance.strip.as_ref()) {
            (Some(json_bins), None) => {
                let bins: Vec<(Bin, usize)> = json_bins
                    .par_iter()
                    .enumerate()
                    .map(|(bin_id, json_bin)| self.parse_bin(json_bin, bin_id))
                    .collect();
                BPInstance::new(items, bins).into()
            }
            (None, Some(json_strip)) => SPInstance::new(items, json_strip.height).into(),
            (Some(_), Some(_)) => {
                panic!("Both bins and strip packing specified, has to be one or the other")
            }
            (None, None) => panic!("Neither bins or strips specified"),
        };

        match &instance {
            Instance::SP(spi) => {
                log!(
                    Level::Info,
                    "[PARSE] strip packing instance \"{}\": {} items ({} unique), {} strip height",
                    json_instance.name,
                    spi.total_item_qty(),
                    spi.items.len(),
                    spi.strip_height
                );
            }
            Instance::BP(bpi) => {
                log!(
                    Level::Info,
                    "[PARSE] bin packing instance \"{}\": {} items ({} unique), {} bins ({} unique)",
                    json_instance.name,
                    bpi.total_item_qty(),
                    bpi.items.len(),
                    bpi.bins.iter().map(|(_, qty)| *qty).sum::<usize>(),
                    bpi.bins.len()
                );
            }
        }

        instance
    }

    /// Parses a `JsonInstance` and accompanying `JsonLayout`s into an `Instance` and `Solution`.
    pub fn parse_and_build_solution(
        &self,
        json_instance: &JsonInstance,
        json_layouts: &Vec<JsonLayout>,
    ) -> (Instance, Solution) {
        let instance = Arc::new(self.parse(json_instance));
        let solution = build_solution_from_json(instance.as_ref(), &json_layouts, self.cde_config);
        let instance =
            Arc::try_unwrap(instance).expect("Cannot unwrap instance, strong references present");
        (instance, solution)
    }

    fn parse_item(&self, json_item: &JsonItem, item_id: usize, path_assets_folder: &PathBuf) -> (Item, usize) {
        if json_item.dxf.is_some() {
            let dxf_path = json_item.dxf.as_ref().unwrap();
            let path = Path::new(path_assets_folder).join(dxf_path);

            let drawing = match Drawing::load_file(path) {
                Ok(drawing) => drawing,
                Err(err) => {
                    panic!("Failed to load DXF file: {}", err);
                }
            };

            for e in drawing.entities() {
                println!("Found entity on layer {}", e.common.layer);
                println!("Entity {:?}", e.specific);
                match e.specific {
                    EntityType::LwPolyline(ref lw_polyline) => {
                        println!("{}", e.common.layer);
                        println!("{}", lw_polyline.get_is_closed());
                    },
                    EntityType::Polyline(ref polyline) => {
                        println!("{}", e.common.layer);
                        println!("{}", polyline.get_is_closed());
                    },
                    _ => (),
                }
                // dxf_items.push(dxf_item);
            }  
        }
        
        let (shape, centering_transf) = match &json_item.shape {
            Some(JsonShape::Rectangle { width, height }) => {
                let shape = SimplePolygon::from(AARectangle::new(0.0, 0.0, *width, *height));
                (shape, Transformation::empty())
            }
            Some(JsonShape::SimplePolygon(sp)) => convert_json_simple_poly(
                sp,
                self.center_polygons,
                self.poly_simpl_config,
                PolySimplMode::Inflate,
            ),
            Some(JsonShape::Polygon(_)) => {
                unimplemented!("No support for polygon shapes yet")
            }
            Some(JsonShape::MultiPolygon(_)) => {
                unimplemented!("No support for multipolygon shapes yet")
            }
            None => panic!("No shape specified for item"),
        };

        let item_value = json_item.value.unwrap_or(0);
        let base_quality = json_item.base_quality;

        let allowed_orientations = match json_item.allowed_orientations.as_ref() {
            Some(a_o) => {
                if a_o.is_empty() || (a_o.len() == 1 && a_o[0] == 0.0) {
                    AllowedRotation::None
                } else {
                    AllowedRotation::Discrete(a_o.iter().map(|angle| angle.to_radians()).collect())
                }
            }
            None => AllowedRotation::Continuous,
        };

        (
            Item::new(
                item_id,
                shape,
                item_value,
                allowed_orientations,
                centering_transf,
                base_quality,
                self.cde_config.item_surrogate_config.clone(),
            ),
            json_item.demand as usize,
        )
    }

    fn parse_bin(&self, json_bin: &JsonBin, bin_id: usize) -> (Bin, usize) {
        let (bin_outer, centering_transf) = match &json_bin.shape {
            Some(JsonShape::Rectangle { width, height }) => {
                let shape = SimplePolygon::from(AARectangle::new(0.0, 0.0, *width, *height));
                (shape, Transformation::empty())
            }
            Some(JsonShape::SimplePolygon(jsp)) => convert_json_simple_poly(
                jsp,
                self.center_polygons,
                self.poly_simpl_config,
                PolySimplMode::Deflate,
            ),
            Some(JsonShape::Polygon(jp)) => convert_json_simple_poly(
                &jp.outer,
                self.center_polygons,
                self.poly_simpl_config,
                PolySimplMode::Deflate,
            ),
            Some(JsonShape::MultiPolygon(_)) => {
                unimplemented!("No support for multipolygon shapes yet")
            }
            None => panic!("No shape specified for bin"),
        };

        let bin_holes = match &json_bin.shape {
            Some(JsonShape::SimplePolygon(_)) | Some(JsonShape::Rectangle { .. }) => vec![],
            Some(JsonShape::Polygon(jp)) => jp
                .inner
                .iter()
                .map(|jsp| {
                    let (hole, _) = convert_json_simple_poly(
                        jsp,
                        false,
                        self.poly_simpl_config,
                        PolySimplMode::Inflate,
                    );
                    hole.transform_clone(&centering_transf)
                })
                .collect_vec(),
            Some(JsonShape::MultiPolygon(_)) => {
                unimplemented!("No support for multipolygon shapes yet")
            }
            None => panic!("No shape specified for bin"),
        };

        let material_value =
            (bin_outer.area() - bin_holes.iter().map(|hole| hole.area()).sum::<fsize>()) as u64;

        assert!(
            json_bin.zones.iter().all(|zone| zone.quality < N_QUALITIES),
            "Quality must be less than N_QUALITIES"
        );

        let quality_zones = (0..N_QUALITIES)
            .map(|quality| {
                let zones = json_bin
                    .zones
                    .iter()
                    .filter(|zone| zone.quality == quality)
                    .map(|zone| {
                        let (zone_shape, _) = match &zone.shape {
                            JsonShape::Rectangle { width, height } => {
                                let shape = SimplePolygon::from(AARectangle::new(
                                    0.0, 0.0, *width, *height,
                                ));
                                (shape, Transformation::empty())
                            }
                            JsonShape::SimplePolygon(jsp) => convert_json_simple_poly(
                                jsp,
                                false,
                                self.poly_simpl_config,
                                PolySimplMode::Inflate,
                            ),
                            JsonShape::Polygon(_) => {
                                unimplemented!(
                                    "No support for polygon to simplepolygon conversion yet"
                                )
                            }
                            JsonShape::MultiPolygon(_) => {
                                unimplemented!("No support for multipolygon shapes yet")
                            }
                        };
                        zone_shape.transform_clone(&centering_transf)
                    })
                    .collect_vec();

                InferiorQualityZone::new(quality, zones)
            })
            .collect_vec();

        let bin = Bin::new(
            bin_id,
            bin_outer,
            material_value,
            centering_transf,
            bin_holes,
            quality_zones,
            self.cde_config,
        );
        let stock = json_bin.stock.unwrap_or(u64::MAX) as usize;

        (bin, stock)
    }

    // pub fn parse_dxf(&self, dxf_instance: &DxfInstance) -> Instance {
    //     let items = dxf_instance
    //         .items
    //         .par_iter()
    //         .enumerate()
    //         .map(|(item_id, dxf_item)| self.parse_item(dxf_item, item_id))
    //         .collect();
    
    //     let instance: Instance = match (dxf_instance.bins.as_ref(), dxf_instance.strip.as_ref()) {
    //         (Some(dxf_bins), None) => {
    //             let bins: Vec<(Bin, usize)> = dxf_bins
    //                 .par_iter()
    //                 .enumerate()
    //                 .map(|(bin_id, dxf_bin)| self.parse_bin(dxf_bin, bin_id))
    //                 .collect();
    //             BPInstance::new(items, bins).into()
    //         }
    //         (None, Some(dxf_strip)) => SPInstance::new(items, dxf_strip.height).into(),
    //         (Some(_), Some(_)) => {
    //             panic!("Both bins and strip packing specified, has to be one or the other")
    //         }
    //         (None, None) => panic!("Neither bins or strips specified"),
    //     };
    
    //     match &instance {
    //         Instance::SP(spi) => {
    //             log!(
    //                 Level::Info,
    //                 "[PARSE] strip packing instance \"{}\": {} items ({} unique), {} strip height",
    //                 dxf_instance.name,
    //                 spi.total_item_qty(),
    //                 spi.items.len(),
    //                 spi.strip_height
    //             );
    //         }
    //         Instance::BP(bpi) => {
    //             log!(
    //                 Level::Info,
    //                 "[PARSE] bin packing instance \"{}\": {} items ({} unique), {} bins ({} unique)",
    //                 dxf_instance.name,
    //                 bpi.total_item_qty(),
    //                 bpi.items.len(),
    //                 bpi.bins.iter().map(|(_, qty)| *qty).sum::<usize>(),
    //                 bpi.bins.len()
    //             );
    //         }
    //     }
    
    //     instance
    // } 
} 
    
    /// Builds a `Solution` from a set of `JsonLayout`s and an `Instance`.
    pub fn build_solution_from_json(
        instance: &Instance,
        json_layouts: &[JsonLayout],
        cde_config: CDEConfig,
    ) -> Solution {
        match instance {
            Instance::BP(bp_i) => build_bin_packing_solution(bp_i, json_layouts),
            Instance::SP(sp_i) => {
                assert_eq!(json_layouts.len(), 1);
                build_strip_packing_solution(sp_i, &json_layouts[0], cde_config)
            }
        }
    }
    
    pub fn build_strip_packing_solution(
        instance: &SPInstance,
        json_layout: &JsonLayout,
        cde_config: CDEConfig,
    ) -> Solution {
        let mut problem = match json_layout.container {
            JsonContainer::Bin { .. } => {
                panic!("Strip packing solution should not contain layouts with references to an Object")
            }
            JsonContainer::Strip { width, height: _ } => {
                SPProblem::new(instance.clone(), width, cde_config)
            }
        };
    
        for json_item in json_layout.placed_items.iter() {
            let item = instance.item(json_item.index);
            let json_rotation = json_item.transformation.rotation;
            let json_translation = json_item.transformation.translation;
    
            let abs_transform = DTransformation::new(json_rotation, json_translation);
            let transform = absolute_to_internal_transform(
                &abs_transform,
                &item.pretransform,
                &problem.layout.bin().pretransform,
            );
    
            let d_transform = transform.decompose();
    
            let placing_opt = PlacingOption {
                layout_index: STRIP_LAYOUT_IDX,
                item_id: item.id,
                transform,
                d_transform,
            };
    
            problem.place_item(&placing_opt);
            problem.flush_changes();
        }
    
        problem.create_solution(&None)
    }



pub fn build_bin_packing_solution(instance: &BPInstance, json_layouts: &[JsonLayout]) -> Solution {
    let mut problem = BPProblem::new(instance.clone());

    for json_layout in json_layouts {
        let bin = match json_layout.container {
            JsonContainer::Bin { index } => &instance.bins[index].0,
            JsonContainer::Strip { .. } => {
                panic!("Bin packing solution should not contain layouts with references to a Strip")
            }
        };
        //Create the layout by inserting the first item

        //Find the template layout matching the bin id in the JSON solution
        let template_index = problem
            .template_layouts()
            .iter()
            .position(|tl| tl.bin().id == bin.id)
            .expect("no template layout found for bin");

        let json_first_item = json_layout.placed_items.get(0).expect("no items in layout");
        let first_item = instance.item(json_first_item.index);
        let abs_transform = DTransformation::new(
            json_first_item.transformation.rotation,
            json_first_item.transformation.translation,
        );

        let transform = absolute_to_internal_transform(
            &abs_transform,
            &first_item.pretransform,
            &bin.pretransform,
        );
        let d_transform = transform.decompose();

        let initial_insert_opt = PlacingOption {
            layout_index: LayoutIndex::Template(template_index),
            item_id: first_item.id,
            transform: transform,
            d_transform: d_transform,
        };
        let layout_index = problem.place_item(&initial_insert_opt);
        problem.flush_changes();

        //Insert the rest of the items
        for json_item in json_layout.placed_items.iter().skip(1) {
            let item = instance.item(json_item.index);
            let json_rotation = json_item.transformation.rotation;
            let json_translation = json_item.transformation.translation;

            let abs_transform = DTransformation::new(json_rotation, json_translation);
            let transform = absolute_to_internal_transform(
                &abs_transform,
                &item.pretransform,
                &bin.pretransform,
            );

            let d_transform = transform.decompose();

            let insert_opt = PlacingOption {
                layout_index,
                item_id: item.id,
                transform,
                d_transform,
            };
            problem.place_item(&insert_opt);
            problem.flush_changes();
        }
    }

    problem.create_solution(&None)
}

/// Composes a `JsonSolution` from a `Solution` and an `Instance`.
pub fn compose_json_solution(
    solution: &Solution,
    instance: &Instance,
    epoch: Instant,
) -> JsonSolution {
    let layouts = solution
        .layout_snapshots
        .iter()
        .map(|sl| {
            let container = match &instance {
                Instance::BP(_bpi) => JsonContainer::Bin { index: sl.bin.id },
                Instance::SP(spi) => JsonContainer::Strip {
                    width: sl.bin.bbox().width(),
                    height: spi.strip_height,
                },
            };

            let placed_items = sl
                .placed_items
                .iter()
                .map(|placed_item| {
                    let item_index = placed_item.item_id();
                    let item = instance.item(item_index);

                    let abs_transf = internal_to_absolute_transform(
                        placed_item.d_transformation(),
                        &item.pretransform,
                        &sl.bin.pretransform,
                    )
                    .decompose();

                    JsonPlacedItem {
                        index: item_index,
                        transformation: JsonTransformation {
                            rotation: abs_transf.rotation(),
                            translation: abs_transf.translation(),
                        },
                    }
                })
                .collect::<Vec<JsonPlacedItem>>();
            let statistics = JsonLayoutStats { usage: sl.usage };
            JsonLayout {
                container,
                placed_items,
                statistics,
            }
        })
        .collect::<Vec<JsonLayout>>();

    JsonSolution {
        layouts,
        usage: solution.usage,
        run_time_sec: solution.time_stamp.duration_since(epoch).as_secs(),
    }
}

fn convert_json_simple_poly(
    s_json_shape: &JsonSimplePoly,
    center_polygon: bool,
    simpl_config: PolySimplConfig,
    simpl_mode: PolySimplMode,
) -> (SimplePolygon, Transformation) {
    let shape = SimplePolygon::new(json_simple_poly_to_points(s_json_shape));

    let shape = match simpl_config {
        PolySimplConfig::Enabled { tolerance } => {
            polygon_simplification::simplify_shape(&shape, simpl_mode, tolerance)
        }
        PolySimplConfig::Disabled => shape,
    };

    let (shape, centering_transform) = match center_polygon {
        true => shape.center_around_centroid(),
        false => (shape, Transformation::empty()),
    };

    (shape, centering_transform)
}

fn json_simple_poly_to_points(jsp: &JsonSimplePoly) -> Vec<Point> {
    //Strip the last vertex if it is the same as the first one
    let n_vertices = match jsp.0[0] == jsp.0[jsp.0.len() - 1] {
        true => jsp.0.len() - 1,
        false => jsp.0.len(),
    };

    (0..n_vertices).map(|i| Point::from(jsp.0[i])).collect_vec()
}

fn internal_to_absolute_transform(
    placed_item_transf: &DTransformation,
    item_pretransf: &Transformation,
    bin_pretransf: &Transformation,
) -> Transformation {
    //1. apply the item pretransform
    //2. apply the placement transformation
    //3. undo the bin pretransformation

    Transformation::empty()
        .transform(item_pretransf)
        .transform_from_decomposed(placed_item_transf)
        .transform(&bin_pretransf.clone().inverse())
}

fn absolute_to_internal_transform(
    abs_transf: &DTransformation,
    item_pretransf: &Transformation,
    bin_pretransf: &Transformation,
) -> Transformation {
    //1. undo the item pretransform
    //2. do the absolute transformation
    //3. apply the bin pretransform

    Transformation::empty()
        .transform(&item_pretransf.clone().inverse())
        .transform_from_decomposed(&abs_transf)
        .transform(bin_pretransf)
}
