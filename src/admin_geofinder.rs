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
use geo;
use geo::contains::Contains;
use gst::rtree::{RTree, Rect};
use mimir::Admin;
use std;
use std::collections::{BTreeMap, BTreeSet};
use std::iter::FromIterator;
use std::sync::Arc;

/// We want to strip the admin's boundary for the objects referencing it (for performance purpose)
/// thus in the `AdminGeoFinder` we store an Admin without the boundary (the option is emptied)
/// and we store the boundary aside
struct BoundaryAndAdmin(Option<geo::MultiPolygon<f64>>, Arc<Admin>);

impl BoundaryAndAdmin {
    fn new(mut admin: Admin) -> BoundaryAndAdmin {
        let b = std::mem::replace(&mut admin.boundary, None);
        let minimal_admin = Arc::new(admin);
        BoundaryAndAdmin(b, minimal_admin)
    }
}

pub struct AdminGeoFinder {
    admins: RTree<BoundaryAndAdmin>,
    admin_by_id: BTreeMap<String, Arc<Admin>>,
}

impl AdminGeoFinder {
    pub fn insert(&mut self, admin: Admin) {
        use ordered_float::OrderedFloat;
        fn min(a: OrderedFloat<f32>, b: f64) -> f32 {
            a.0.min(down(b as f32))
        }
        fn max(a: OrderedFloat<f32>, b: f64) -> f32 {
            a.0.max(up(b as f32))
        }

        let rect = {
            let mut coords = match admin.boundary {
                Some(ref b) => b.0.iter().flat_map(|poly| (poly.exterior).0.iter()),
                None => return,
            };
            let first_coord = match coords.next() {
                Some(c) => c,
                None => return,
            };
            let first_rect: Rect = {
                let (x, y) = (first_coord.x() as f32, first_coord.y() as f32);
                Rect::from_float(down(x), up(x), down(y), up(y))
            };
            coords.fold(first_rect, |accu, p| {
                Rect::from_float(
                    min(accu.xmin, p.x()),
                    max(accu.xmax, p.x()),
                    min(accu.ymin, p.y()),
                    max(accu.ymax, p.y()),
                )
            })
        };
        let bound_admin = BoundaryAndAdmin::new(admin);
        self.admin_by_id
            .insert(bound_admin.1.id.clone(), bound_admin.1.clone());
        self.admins.insert(rect, bound_admin);
    }

    /// Get all Admins overlapping the coordinate
    pub fn get(&self, coord: &geo::Coordinate<f64>) -> Vec<Arc<Admin>> {
        let (x, y) = (coord.x as f32, coord.y as f32);
        let search = Rect::from_float(down(x), up(x), down(y), up(y));
        let mut rtree_results = self.admins.get(&search);

        rtree_results.sort_by_key(|(_, a)| a.1.zone_type);

        let mut tested_hierarchy = BTreeSet::<String>::new();
        let mut added_zone_types = BTreeSet::new();
        let mut res = vec![];

        for (_, boundary_and_admin) in rtree_results {
            let boundary = &boundary_and_admin.0;
            let admin = &boundary_and_admin.1;
            if tested_hierarchy.contains(&admin.id) {
                res.push(admin.clone());
            } else if admin
                .zone_type
                .as_ref()
                .map_or(false, |zt| added_zone_types.contains(zt))
            {
                // we don't want it, we already have this kind of ZoneType
            } else if boundary
                .as_ref()
                .map_or(false, |b| b.contains(&geo::Point(*coord)))
            {
                // we found a valid admin, we save it's hierarchy not to have to test their boundaries
                if let Some(zt) = admin.zone_type {
                    added_zone_types.insert(zt.clone());
                }
                let mut admin_parent_id = admin.parent_id.clone();
                while let Some(id) = admin_parent_id {
                    let admin_parent = self.admin_by_id.get(&id);
                    if let Some(zt) = admin_parent.as_ref().and_then(|a| a.zone_type) {
                        added_zone_types.insert(zt.clone());
                    }
                    tested_hierarchy.insert(id);
                    admin_parent_id = admin_parent.and_then(|a| a.parent_id.clone());
                }

                res.push(admin.clone());
            }
        }
        res
    }

    /// Iterates on all the admins with a not None boundary.
    pub fn admins<'a>(&'a self) -> impl Iterator<Item = Admin> + 'a {
        self.admins
            .get(&Rect::from_float(
                std::f32::NEG_INFINITY,
                std::f32::INFINITY,
                std::f32::NEG_INFINITY,
                std::f32::INFINITY,
            ))
            .into_iter()
            .map(|(_, a)| {
                let mut admin = (*a.1).clone();
                admin.boundary = a.0.clone();
                admin
            })
    }

    /// Iterates on all the `Rc<Admin>` in the structure as returned by `get`.
    pub fn admins_without_boundary<'a>(&'a self) -> impl Iterator<Item = Arc<Admin>> + 'a {
        self.admins
            .get(&Rect::from_float(
                std::f32::NEG_INFINITY,
                std::f32::INFINITY,
                std::f32::NEG_INFINITY,
                std::f32::INFINITY,
            ))
            .into_iter()
            .map(|(_, a)| a.1.clone())
    }
}

impl Default for AdminGeoFinder {
    fn default() -> Self {
        AdminGeoFinder {
            admins: RTree::new(),
            admin_by_id: BTreeMap::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmogony::ZoneType;
    use geo::prelude::BoundingBox;

    fn p(x: f64, y: f64) -> ::geo::Point<f64> {
        ::geo::Point(::geo::Coordinate { x: x, y: y })
    }

    fn make_admin(offset: f64, zt: Option<ZoneType>) -> ::mimir::Admin {
        make_complex_admin(&format!("admin:offset:{}", offset,), offset, zt, 1., None)
    }

    fn make_complex_admin(
        id: &str,
        offset: f64,
        zt: Option<ZoneType>,
        zone_size: f64,
        parent_offset: Option<&str>,
    ) -> ::mimir::Admin {
        // the boundary is a big octogon
        // the zone_size param is used to control the area of the zone
        let shape = ::geo::Polygon::new(
            ::geo::LineString(vec![
                p(3. * zone_size + offset, 0. * zone_size + offset),
                p(6. * zone_size + offset, 0. * zone_size + offset),
                p(9. * zone_size + offset, 3. * zone_size + offset),
                p(9. * zone_size + offset, 6. * zone_size + offset),
                p(6. * zone_size + offset, 9. * zone_size + offset),
                p(3. * zone_size + offset, 9. * zone_size + offset),
                p(0. * zone_size + offset, 6. * zone_size + offset),
                p(0. * zone_size + offset, 3. * zone_size + offset),
                p(3. * zone_size + offset, 0. * zone_size + offset),
            ]),
            vec![],
        );
        let boundary = ::geo::MultiPolygon(vec![shape]);

        ::mimir::Admin {
            id: id.into(),
            level: 8,
            name: "city".to_string(),
            label: format!("city {}", offset),
            zip_codes: vec!["421337".to_string()],
            weight: 0f64,
            coord: ::mimir::Coord::new(4.0 + offset, 4.0 + offset),
            bbox: boundary.bbox(),
            boundary: Some(boundary),
            insee: "outlook".to_string(),
            zone_type: zt,
            parent_id: parent_offset.map(|id| id.into()),
            codes: vec![],
            names: ::mimir::I18nProperties::default(),
            labels: ::mimir::I18nProperties::default(),
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

    #[test]
    fn test_two_no_zone_type() {
        // a point can be associated to only 1 admin type
        // but a point can be associated to multiple admin without zone_type
        // (for retrocompatibility of the data imported without cosmogony)
        let mut finder = AdminGeoFinder::default();
        finder.insert(make_admin(40., None));
        finder.insert(make_admin(43., None));
        let admins = finder.get(&p(46., 46.).0);
        assert_eq!(admins.len(), 2);
    }

    #[test]
    fn test_hierarchy() {
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
        assert_eq!(admins[1].id, "bob_state");
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
