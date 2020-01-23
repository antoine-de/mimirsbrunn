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

use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo_types::{MultiPolygon, Point};
use mimir::Admin;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
// use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::sync::Arc;

pub struct BoundedId {
    pub boundary: MultiPolygon<f64>,
    pub admin: Arc<Admin>,
}

impl RTreeObject for BoundedId {
    type Envelope = AABB<Point<f64>>;

    fn envelope(&self) -> Self::Envelope {
        let bb = self.boundary.bounding_rect().unwrap();
        AABB::from_corners(
            Point::new(bb.min.x, bb.min.y),
            Point::new(bb.max.x, bb.max.y),
        )
    }
}

impl PointDistance for BoundedId {
    // This function computes the square of the distance from an Admin to a point.
    // We compute the distance from the boundary to a point.
    fn distance_2(&self, point: &Point<f64>) -> f64 {
        let d = self.boundary.euclidean_distance(point);
        d * d
    }
}

pub struct AdminGeoFinder {
    rtree: RTree<BoundedId>,
}

impl AdminGeoFinder {
    pub fn insert(&mut self, admin: Admin) {
        let mut admin = admin;
        let boundary = std::mem::replace(&mut admin.boundary, None);
        if let Some(boundary) = boundary {
            let bounded_id = BoundedId {
                boundary,
                admin: Arc::new(admin),
            };
            self.rtree.insert(bounded_id);
        }
    }

    // Get all Admins overlapping the given coordinates.
    pub fn get(&self, coord: &geo_types::Coordinate<f64>) -> Vec<Arc<Admin>> {
        let point: geo_types::Point<f64> = coord.clone().into();
        // Get a list of overlapping admins...
        let mut admins: Vec<Arc<Admin>> = self
            .rtree
            .locate_all_at_point(&point)
            .map(|bounded_id| bounded_id.admin.clone())
            .collect();
        // Then dedup by zone_type (provided the zone_type is not None)
        // Note that dedup requires a sorted vector.
        admins.sort_by_key(|adm| adm.zone_type);
        admins.dedup_by(|adm1, adm2| {
            if adm1.zone_type != adm2.zone_type {
                false
            } else {
                if adm1.zone_type == None {
                    false
                } else {
                    true
                }
            }
        });
        admins
    }

    /// Iterates on all the admins with a not None boundary.
    pub fn admins(&self) -> impl Iterator<Item = Arc<Admin>> + '_ {
        self.rtree.iter().map(|bounded_id| bounded_id.admin.clone())
    }
}

impl Default for AdminGeoFinder {
    fn default() -> Self {
        AdminGeoFinder {
            rtree: RTree::new(),
        }
    }
}

impl FromIterator<Admin> for AdminGeoFinder {
    fn from_iter<I: IntoIterator<Item = Admin>>(admins: I) -> Self {
        let mut geofinder = AdminGeoFinder::default();

        for admin in admins {
            geofinder.insert(admin);
        }

        geofinder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmogony::ZoneType;
    use geo::prelude::BoundingRect;

    // the goal is that f in [down(f as f32) as f64, up(f as f32) as f64]
    fn down(f: f32) -> f32 {
        f - (f * ::std::f32::EPSILON).abs()
    }
    fn up(f: f32) -> f32 {
        f + (f * ::std::f32::EPSILON).abs()
    }

    #[test]
    fn test_up_down() {
        for &f in [1.0f64, 0., -0., -1., 0.1, -0.1, 0.9, -0.9, 42., -42.].iter() {
            let small_f = f as f32;
            assert!(
                down(small_f) as f64 <= f,
                format!("{} <= {}", down(small_f) as f64, f)
            );
            assert!(
                f <= up(small_f) as f64,
                format!("{} <= {}", f, up(small_f) as f64)
            );
        }
    }

    fn p(x: f64, y: f64) -> geo_types::Point<f64> {
        geo_types::Point(geo_types::Coordinate { x: x, y: y })
    }

    fn make_admin(offset: f64, zt: Option<ZoneType>) -> ::mimir::Admin {
        make_complex_admin(&format!("admin:offset:{}", offset,), offset, zt, 1., None)
    }

    fn make_complex_admin(
        id: &str,
        offset: f64,
        zone_type: Option<ZoneType>,
        zone_size: f64,
        parent_offset: Option<&str>,
    ) -> ::mimir::Admin {
        // the boundary is a big octogon
        // the zone_size param is used to control the area of the zone
        let shape = geo_types::Polygon::new(
            geo_types::LineString(vec![
                (3. * zone_size + offset, 0. * zone_size + offset).into(), //     ^
                (6. * zone_size + offset, 0. * zone_size + offset).into(), //     |   x   x
                (9. * zone_size + offset, 3. * zone_size + offset).into(), //     |
                (9. * zone_size + offset, 6. * zone_size + offset).into(), //     x           x
                (6. * zone_size + offset, 9. * zone_size + offset).into(), //     |
                (3. * zone_size + offset, 9. * zone_size + offset).into(), //     x           x
                (0. * zone_size + offset, 6. * zone_size + offset).into(), //     |
                (0. * zone_size + offset, 3. * zone_size + offset).into(), //     +---x---x------->
                (3. * zone_size + offset, 0. * zone_size + offset).into(), //
            ]),
            vec![],
        );
        let boundary = geo_types::MultiPolygon(vec![shape]);

        let coord = ::mimir::Coord::new(4.0 + offset, 4.0 + offset);
        ::mimir::Admin {
            id: id.into(),
            level: 8,
            name: "city".to_string(),
            label: format!("city {}", offset),
            zip_codes: vec!["421337".to_string()],
            weight: 0f64,
            coord: coord.clone(),
            approx_coord: Some(coord.into()),
            bbox: boundary.bounding_rect(),
            boundary: Some(boundary),
            insee: "outlook".to_string(),
            zone_type: zone_type,
            parent_id: parent_offset.map(|id| id.into()),
            ..Default::default()
        }
    }

    #[test]
    fn test_two_fake_admins() {
        let mut finder = AdminGeoFinder::default();
        finder.insert(make_admin(40., Some(ZoneType::City)));
        finder.insert(make_admin(43., Some(ZoneType::State)));

        // outside
        for coord in [p(48., 41.), p(411., 41.), p(51., 54.), p(53., 53.)].iter() {
            assert!(finder.get(&coord.0).is_empty());
        }

        // inside one
        let admins = finder.get(&p(44., 44.).0);
        assert_eq!(admins.len(), 1);
        assert_eq!(admins[0].id, "admin:offset:40");
        let admins = finder.get(&p(48., 48.).0);
        assert_eq!(admins.len(), 1);
        assert_eq!(admins[0].id, "admin:offset:43");

        // inside two
        let mut admins = finder.get(&p(46., 46.).0);
        admins.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(admins.len(), 2);
        assert_eq!(admins[0].id, "admin:offset:40");
        assert_eq!(admins[1].id, "admin:offset:43");
    }

    #[test]
    fn test_two_admin_same_zone_type() {
        // a point can be associated to only 1 admin type
        // so a point is in 2 city, it is associated to only one
        let mut finder = AdminGeoFinder::default();
        finder.insert(make_admin(40., Some(ZoneType::City)));
        finder.insert(make_admin(43., Some(ZoneType::City)));
        let admins = finder.get(&p(46., 46.).0);
        assert_eq!(admins.len(), 1);
    }

    // #[test]
    // fn test_two_no_zone_type() {
    //     // a point can be associated to only 1 admin type
    //     // but a point can be associated to multiple admin without zone_type
    //     // (for retrocompatibility of the data imported without cosmogony)
    //     let mut finder = AdminGeoFinder::default();
    //     finder.insert(make_admin(40., None));
    //     finder.insert(make_admin(43., None));
    //     let admins = finder.get(&p(46., 46.).0);
    //     assert_eq!(admins.len(), 2);
    // }

    #[test]
    fn test_hierarchy() {
        // In this test we use 3 admin regions that include each other.
        let mut finder = AdminGeoFinder::default();
        finder.insert(make_complex_admin(
            "bob_city",
            40.,
            Some(ZoneType::City),
            1.,
            Some("bob_state"),
        ));
        finder.insert(make_complex_admin(
            "bob_state",
            40.,
            Some(ZoneType::StateDistrict),
            2.,
            Some("bob_country"),
        ));
        finder.insert(make_complex_admin(
            "bob_country",
            40.,
            Some(ZoneType::Country),
            3.,
            None,
        ));

        let admins = finder.get(&p(46., 46.).0);
        assert_eq!(admins.len(), 3);
        assert_eq!(admins[0].id, "bob_city");
        assert_eq!(admins[1].id, "bob_state");
        assert_eq!(admins[2].id, "bob_country");
    }

    #[test]
    fn test_hierarchy_orphan() {
        let mut finder = AdminGeoFinder::default();
        finder.insert(make_complex_admin(
            "bob_city",
            40.,
            Some(ZoneType::City),
            1.,
            Some("bob_state"),
        ));
        finder.insert(make_complex_admin(
            "bob_state",
            40.,
            Some(ZoneType::StateDistrict),
            2.,
            Some("bob_country"),
        ));
        finder.insert(make_complex_admin(
            "bob_country",
            40.,
            Some(ZoneType::Country),
            3.,
            None,
        ));

        // another_state also contains the point, but the geofinder look for only 1 admin by type (it needs only 1 state)
        // since bob_city has been tester first, it's hierarchy has been added automatically
        // so [46., 46.] will not be associated to another_state
        finder.insert(make_complex_admin(
            "another_state",
            40.,
            Some(ZoneType::StateDistrict),
            2.,
            Some("bob_country"),
        ));

        let admins = finder.get(&p(46., 46.).0);
        assert_eq!(admins.len(), 3);
        assert_eq!(admins[0].id, "bob_city");
        assert_eq!(admins[1].id, "another_state");
        assert_eq!(admins[2].id, "bob_country");
    }

    #[test]
    fn test_hierarchy_and_not_typed_zone() {
        let mut finder = AdminGeoFinder::default();
        finder.insert(make_complex_admin(
            "bob_city",
            40.,
            Some(ZoneType::City),
            1.,
            Some("bob_state"),
        ));
        finder.insert(make_complex_admin(
            "bob_state",
            40.,
            Some(ZoneType::StateDistrict),
            2.,
            Some("bob_country"),
        ));
        finder.insert(make_complex_admin(
            "bob_country",
            40.,
            Some(ZoneType::Country),
            3.,
            None,
        ));

        // not_typed zone is outside the hierarchy, but since it contains the point and it has no type it is added
        finder.insert(make_complex_admin("no_typed_zone", 40., None, 2., None));

        let admins = finder.get(&p(46., 46.).0);
        assert_eq!(admins.len(), 4);
        assert_eq!(admins[0].id, "no_typed_zone");
        assert_eq!(admins[1].id, "bob_city");
        assert_eq!(admins[2].id, "bob_state");
        assert_eq!(admins[3].id, "bob_country");
    }
}
