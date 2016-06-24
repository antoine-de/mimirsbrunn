// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Canal TP (www.canaltp.fr).
// Help us simplify mobility and open public transport:
//     a non ending quest to the responsive locomotion way of traveling!
//
// LICENCE: This program is free software; you can redistribute it
// and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program. If not, see
// <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// IRC #navitia on freenode
// https://groups.google.com/d/forum/navitia
// www.navitia.io
use mimir::Admin;
use geo::contains::Contains;
use geo;
use std::rc::Rc;
use gst::rtree::{RTree, Rect, Point};

pub struct AdminGeoFinder {
    admins: RTree<Rc<Admin>>
}

impl AdminGeoFinder {
    pub fn new() -> AdminGeoFinder {
        AdminGeoFinder {
            admins: RTree::new()
        }
    }

    pub fn insert(&mut self, admin: Rc<Admin>) {
        use ::ordered_float::OrderedFloat;
        fn min(a: OrderedFloat<f32>, b: f64) -> f32 {
            ::std::cmp::min(a, OrderedFloat(b as f32)).0
        }
        fn max(a: OrderedFloat<f32>, b: f64) -> f32 {
            ::std::cmp::max(a, OrderedFloat(b as f32)).0
        }

        let rect = {
            let mut coords = match admin.boundary {
                Some(ref b) => b.0.iter().flat_map(|poly| (poly.0).0.iter()),
                None => return,
            };
            let first_coord = match coords.next() {
                Some(c) => c,
                None => return,
            };
            let first_rect: Rect = Point::new(first_coord.x() as f32, first_coord.y() as f32).into();
            coords.fold(first_rect, |accu, p| Rect::from_float(min(accu.xmin, p.x()),
                                                               max(accu.xmax, p.x()),
                                                               min(accu.ymin, p.y()),
                                                               max(accu.ymax, p.y())))
        };
        self.admins.insert(rect, admin);
    }

    /// Get all Admins overlapping the coordinate
    pub fn get(&self, coord: &geo::Coordinate) -> Vec<Rc<Admin>> {
        let search: Rect = Point::new(coord.x as f32, coord.y as f32).into();
        self.admins.get(&search).into_iter().map(|(_, a)| a).filter(|a| {
                a.boundary.as_ref().map_or(false, |b| {
                    b.contains(&geo::Point(coord.clone()))
                })
            })
            .cloned().collect()
    }
}
