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
//

extern crate mimir;
extern crate osmpbfreader;
use std::collections::BTreeMap;

pub fn named_node(lat: f64, lon: f64, name: &'static str) -> (mimir::Coord, Option<String>) {
    (mimir::Coord::new(lon, lat), Some(name.to_string()))
}

pub struct Relation<'a> {
    builder: &'a mut OsmBuilder,
    pub relation_id: osmpbfreader::RelationId,
}

impl<'a> Relation<'a> {
    pub fn outer(&mut self, coords: Vec<(mimir::Coord, Option<String>)>) -> &'a mut Relation {
        let id = self.builder.way(coords);
        if let &mut osmpbfreader::OsmObj::Relation(ref mut rel) = self.builder
            .objects
            .get_mut(&self.relation_id.into())
            .unwrap()
        {
            rel.refs.push(osmpbfreader::Ref {
                role: "outer".to_string(),
                member: id.into(),
            });
        }
        self
    }
}

pub struct OsmBuilder {
    node_id: i64,
    way_id: i64,
    relation_id: i64,
    pub objects: BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    named_nodes: BTreeMap<String, osmpbfreader::NodeId>,
}

impl OsmBuilder {
    pub fn new() -> OsmBuilder {
        OsmBuilder {
            node_id: 0,
            way_id: 0,
            relation_id: 0,
            objects: BTreeMap::new(),
            named_nodes: BTreeMap::new(),
        }
    }

    pub fn relation(&mut self) -> Relation {
        let id = osmpbfreader::RelationId(self.relation_id);
        let r = osmpbfreader::Relation {
            id: id,
            refs: vec![],
            tags: osmpbfreader::Tags::new(),
        };
        self.relation_id += 1;
        self.objects.insert(id.into(), r.into());
        Relation {
            builder: self,
            relation_id: id,
        }
    }

    pub fn way(&mut self, coords: Vec<(mimir::Coord, Option<String>)>) -> osmpbfreader::WayId {
        let nodes = coords
            .into_iter()
            .map(|pair| self.node(pair.0, pair.1))
            .collect::<Vec<_>>();
        let id = osmpbfreader::WayId(self.way_id);
        let w = osmpbfreader::Way {
            id: id,
            nodes: nodes,
            tags: osmpbfreader::Tags::new(),
        };
        self.way_id += 1;
        self.objects.insert(id.into(), w.into());
        id
    }

    pub fn node(&mut self, coord: mimir::Coord, name: Option<String>) -> osmpbfreader::NodeId {
        if let Some(value) = name.as_ref().and_then(|n| self.named_nodes.get(n)) {
            return *value;
        }
        let id = osmpbfreader::NodeId(self.node_id);
        let n = osmpbfreader::Node {
            id: id,
            decimicro_lat: (coord.lat() * 1e7) as i32,
            decimicro_lon: (coord.lon() * 1e7) as i32,
            tags: osmpbfreader::Tags::new(),
        };
        self.node_id += 1;
        self.objects.insert(id.into(), n.into());
        if let Some(ref n) = name {
            self.named_nodes.insert(n.clone(), id);
        }
        id
    }
}
