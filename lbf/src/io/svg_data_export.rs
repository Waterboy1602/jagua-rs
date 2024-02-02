use svg::node::element::{Circle, Path};
use svg::node::element::path::Data;
use jaguars::collision_detection::hazard::HazardEntity;

use jaguars::collision_detection::quadtree::qt_hazard::QTHazPresence;
use jaguars::collision_detection::quadtree::qt_node::QTNode;



use jaguars::geometry;
use jaguars::geometry::primitives::edge::Edge;

use jaguars::geometry::primitives::point::Point;
use jaguars::geometry::primitives::simple_polygon::SimplePolygon;


pub fn simple_polygon_data(s_poly: &SimplePolygon) -> Data {
    let mut data = Data::new().move_to::<(f64, f64)>(s_poly.get_point(0).into());
    for i in 1..s_poly.number_of_points() {
        data = data.line_to::<(f64, f64)>(s_poly.get_point(i).into());
    }
    data.close()
}

pub fn quad_tree_data(qt_root: &QTNode, ignored_entities: &[HazardEntity]) -> (Data, Data, Data) {
    qt_node_data(qt_root, Data::new(), Data::new(), Data::new(), ignored_entities)
}

fn qt_node_data(
    qt_node: &QTNode,
    mut data_eh: Data, //entire inclusion data
    mut data_ph: Data, //partial inclusion data
    mut data_nh: Data, //no inclusion data
    ignored_entities: &[HazardEntity],
) -> (Data, Data, Data) {
    //Only draw qt_nodes that do not have a child

    match (qt_node.has_children(), qt_node.hazards().strongest(ignored_entities)) {
        (true, Some(_)) => {
            //not a leaf node, go to children
            for child in qt_node.children().as_ref().unwrap().iter() {
                let data = qt_node_data(child, data_eh, data_ph, data_nh, ignored_entities);
                data_eh = data.0;
                data_ph = data.1;
                data_nh = data.2;
            }
        }
        (true, None) | (false, _) => {
            //leaf node, draw it
            let rect = qt_node.bbox();
            let draw = |data: Data| -> Data {
                data
                    .move_to((rect.x_min, rect.y_min))
                    .line_to((rect.x_max, rect.y_min))
                    .line_to((rect.x_max, rect.y_max))
                    .line_to((rect.x_min, rect.y_max))
                    .close()
            };

            match qt_node.hazards().strongest(ignored_entities) {
                Some(ch) => {
                    match ch.presence {
                        QTHazPresence::Entire => data_eh = draw(data_eh),
                        QTHazPresence::Partial(_) => data_ph = draw(data_ph),
                        QTHazPresence::None => unreachable!()
                    }
                }
                None => data_nh = draw(data_nh),
            }
        }
    }

    (data_eh, data_ph, data_nh)
}

pub fn data_to_path(data: Data, params: &[(&str, &str)]) -> Path {
    let mut path = Path::new();
    for param in params {
        path = path.set(param.0, param.1)
    }
    path.set("d", data)
}

pub fn point(Point(x, y): Point, fill: Option<&str>, rad: Option<f64>) -> Circle {
    Circle::new()
        .set("cx", x)
        .set("cy", y)
        .set("r", rad.unwrap_or(0.5))
        .set("fill", fill.unwrap_or("black"))
}

pub fn circle(circle: &geometry::primitives::circle::Circle, params: &[(&str, &str)]) -> Circle {
    let mut circle = Circle::new()
        .set("cx", circle.center.0)
        .set("cy", circle.center.1)
        .set("r", circle.radius);
    for param in params {
        circle = circle.set(param.0, param.1)
    }
    circle
}

pub fn edge_data(edge: &Edge) -> Data {
    Data::new()
        .move_to((edge.start().0, edge.start().1))
        .line_to((edge.end().0, edge.end().1))
}