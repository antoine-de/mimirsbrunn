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
use ntree;
use geo::contains::Contains;
use geo;

pub struct AdminGeoFinder {
    //quadtree: ntree::NTree<QuadTreeRegion, Admin>,
    admins: Vec<Admin>
}

impl AdminGeoFinder {
    pub fn new() -> AdminGeoFinder {
        AdminGeoFinder {
            //quadtree: ntree::NTree::new(QuadTreeRegion::new(-180.0, -180.0, 360.0), 4)
            admins: vec![]
        }

    }

    pub fn add_admin(&mut self, admin: Admin) {
        self.admins.push(admin);
    }

    /// Get all Admins overlaping the coordinate
    pub fn get_admins_for_coord(&self, coord: &geo::Coordinate) -> Vec<&Admin> {
        self.admins.iter().filter(|a| {
                a.boundary.as_ref().map_or(false, |b| {
                    b.contains(&geo::Point(coord.clone()))
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct QuadTreeRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

impl QuadTreeRegion {
    pub fn new(x: f64, y: f64, wh: f64) -> QuadTreeRegion {
        QuadTreeRegion {
            y: y,
            x: x,
            width: wh,
        }
    }
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        self.y <= y && self.x <= x && (self.y + self.width) >= y &&
        (self.x + self.width) >= x
    }
}

impl ntree::Region<Admin> for QuadTreeRegion {
    fn contains(&self, admin: &Admin) -> bool {
        admin.coord.as_ref().map_or(false, |c| self.contains_point(c.x, c.y))
    }

    fn split(&self) -> Vec<QuadTreeRegion> {
        println!("we split a square {:?}", &self);
        let halfwidth = self.width / 2.0;
        vec![
            QuadTreeRegion::new(self.x, self.y, halfwidth),
            QuadTreeRegion::new(self.x, self.y + halfwidth, halfwidth),
            QuadTreeRegion::new(self.x + halfwidth, self.y, halfwidth),
            QuadTreeRegion::new(self.x + halfwidth, self.y + halfwidth, halfwidth),
         ]
    }

    fn overlaps(&self, other: &QuadTreeRegion) -> bool {
        other.contains_point(self.x, self.y) ||
        other.contains_point(self.x + self.width, self.y) ||
        other.contains_point(self.x, self.y + self.width) ||
        other.contains_point(self.x + self.width, self.y + self.width)
    }
}


#[cfg(test)]
mod tests {
    use mimir::Admin;
    use geo;
    use ntree::Region;

    #[test]
    fn test_no_overlap() {
        let a = super::QuadTreeRegion::new(10., 20., 5.);
        let b = super::QuadTreeRegion::new(20., 10., 5.);

        // a region must overlap itself
        assert!(a.overlaps(&a));
        assert!(b.overlaps(&b));

        // but the 2 region does not overlap
        assert!(! a.overlaps(&b));
        assert!(! b.overlaps(&a));
    }
    #[test]
    fn test_overlap_nerby() {
        let a = super::QuadTreeRegion::new(10., 10., 5.);
        let b = super::QuadTreeRegion::new(15., 10., 5.);

        // b touches a, the regions overlap
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }
    #[test]
    fn test_overlap() {
        let a = super::QuadTreeRegion::new(-10., -10., 5.);
        let b = super::QuadTreeRegion::new(-12., -12., 5.);

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    fn make_dummy_admin() -> Admin {
        let p = |x, y| geo::Point(geo::Coordinate { x: x, y: y });
        // the boundary is a big square around the centre of the admin
        let square = geo::Polygon(geo::LineString(vec![p(41., 23.),
                                                       p(41., 25.),
                                                       p(43., 25.),
                                                       p(43., 23.)]),
                                  vec![]);
        let boundary = geo::MultiPolygon(vec![square]);

        Admin {
            id: "admin:fr:bob".to_string(),
            level: 8,
            name: "bob".to_string(),
            zip_code: "ziiip".to_string(),
            weight: 1,
            coord: Some(geo::Coordinate { x: 42.0, y: 24.0 }),
            boundary: Some(boundary),
            insee: "my_insee".to_string()
        }
    }

    #[test]
    fn test_one_admin() {
        let a = make_dummy_admin();
        let a_coord = a.coord.unwrap().clone();

        let mut geo_finder = super::AdminGeoFinder::new();

        geo_finder.add_admin(a);

        let admins = geo_finder.get_admins_for_coord(&a_coord);

        assert_eq!(admins.len(), 1);
    }
}
