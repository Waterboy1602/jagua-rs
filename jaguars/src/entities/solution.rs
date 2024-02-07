use std::time::Instant;

use itertools::Itertools;

use crate::entities::instance::{Instance, PackingType};
use crate::entities::layout::LayoutSnapshot;
use crate::geometry::geo_traits::Shape;

//TODO: clean this up properly
#[derive(Debug, Clone)]
pub struct Solution {
    id: usize,
    layout_snapshots: Vec<LayoutSnapshot>,
    usage: f64,
    placed_item_qtys: Vec<usize>,
    target_item_qtys: Vec<usize>,
    bin_qtys: Vec<usize>,
    time_stamp: Instant,
}

impl Solution {
    pub fn new(id: usize, layout_snapshots: Vec<LayoutSnapshot>, usage: f64, placed_item_qtys: Vec<usize>, target_item_qtys: Vec<usize>, bin_qtys: Vec<usize>) -> Self {
        Solution {
            id,
            layout_snapshots,
            usage,
            placed_item_qtys,
            target_item_qtys,
            bin_qtys,
            time_stamp: Instant::now(),
        }
    }

    pub fn layout_snapshots(&self) -> &Vec<LayoutSnapshot> {
        &self.layout_snapshots
    }

    pub fn is_complete(&self, instance: &Instance) -> bool {
        self.placed_item_qtys.iter().enumerate().all(|(i, &qty)| qty >= instance.item_qty(i))
    }

    pub fn completeness(&self, instance: &Instance) -> f64 {
        //ratio of included item area vs total instance item area
        let total_item_area = instance.item_area();
        let included_item_area = self.placed_item_qtys.iter().enumerate()
            .map(|(i, qty)| instance.item(i).shape().area() * *qty as f64)
            .sum::<f64>();
        let completeness = included_item_area / total_item_area;
        completeness
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn placed_item_qtys(&self) -> &Vec<usize> {
        &self.placed_item_qtys
    }

    pub fn missing_item_qtys(&self, instance: &Instance) -> Vec<isize> {
        debug_assert!(instance.items().len() == self.placed_item_qtys.len());
        self.placed_item_qtys.iter().enumerate()
            .map(|(i, &qty)| instance.item_qty(i) as isize - qty as isize)
            .collect_vec()
    }

    pub fn bin_qtys(&self) -> &Vec<usize> {
        &self.bin_qtys
    }

    pub fn usage(&self) -> f64 {
        self.usage
    }

    pub fn target_item_qtys(&self) -> &Vec<usize> {
        &self.target_item_qtys
    }

    pub fn is_best_possible(&self, instance: &Instance) -> bool {
        match instance.packing_type() {
            PackingType::StripPacking { .. } => false,
            PackingType::BinPacking(bins) => {
                match self.layout_snapshots.len() {
                    0 => panic!("No stored layouts in solution"),
                    1 => {
                        let cheapest_bin = &bins.iter().min_by(|(b1, _), (b2, _)| b1.value().cmp(&b2.value())).unwrap().0;
                        self.layout_snapshots[0].bin().id() == cheapest_bin.id()
                    }
                    _ => false
                }
            }
        }
    }

    pub fn time_stamp(&self) -> Instant {
        self.time_stamp
    }

    pub fn n_items_placed(&self) -> usize {
        self.placed_item_qtys.iter().sum()
    }
}
