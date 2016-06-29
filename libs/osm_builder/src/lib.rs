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

extern crate osmpbfreader;
extern crate mimir;
use std::collections::BTreeMap;

pub fn named_node(lat: f64, lon: f64, name: &'static str) -> (mimir::CoordWrapper, Option<String>) {
    (mimir::CoordWrapper::new(lon, lat), Some(name.to_string()))
}


pub struct Relation<'a> {
    builder: &'a mut OsmBuilder,
    pub relation_id: osmpbfreader::OsmId,
}

impl<'a> Relation<'a> {
    pub fn outer(&mut self, coords: Vec<(mimir::CoordWrapper, Option<String>)>) -> &'a mut Relation {
        let id = self.builder.way(coords);
        if let &mut osmpbfreader::OsmObj::Relation(ref mut rel) = self.builder.objects.get_mut(&self.relation_id).unwrap() {
            rel.refs.push(osmpbfreader::Ref{role: "outer".to_string(), member: id});
        }
        self
    }
}

pub struct OsmBuilder {
    node_id: i64,
    way_id: i64,
    relation_id: i64,
    pub objects: BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    named_nodes: BTreeMap<String, osmpbfreader::OsmId>,
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
        let r = osmpbfreader::Relation {
            id: self.relation_id,
            refs: vec!(),
            tags: osmpbfreader::Tags::new(),
        };
        let id = osmpbfreader::OsmId::Relation(self.relation_id);
        self.relation_id += 1;
        self.objects.insert(id, osmpbfreader::OsmObj::Relation(r));
        Relation {
            builder: self,
            relation_id: id,
        }
    }

    pub fn way(&mut self, coords: Vec<(mimir::CoordWrapper, Option<String>)>) -> osmpbfreader::OsmId {
        let nodes = coords.into_iter()
                          .map(|pair| self.node(pair.0, pair.1))
                          .filter_map(|osm_id| if let osmpbfreader::OsmId::Node(id) = osm_id {
                              Some(id)
                          } else {
                              None
                          })
                          .collect::<Vec<_>>();
        let w = osmpbfreader::Way {
            id: self.way_id,
            nodes: nodes,
            tags: osmpbfreader::Tags::new(),
        };
        let id = osmpbfreader::OsmId::Way(self.way_id);
        self.way_id += 1;
        self.objects.insert(id, osmpbfreader::OsmObj::Way(w));
        id
    }

    pub fn node(&mut self, coord: mimir::CoordWrapper, name: Option<String>) -> osmpbfreader::OsmId {
        if let Some(ref value) = name.clone().and_then(|n| self.named_nodes.get(&n)){
            return *value.clone()
        }
        let n = osmpbfreader::Node {
            id: self.node_id,
            lat: coord.x,
            lon: coord.y,
            tags: osmpbfreader::Tags::new(),
        };
        let id = osmpbfreader::OsmId::Node(self.node_id);
        self.node_id += 1;
        self.objects.insert(id, osmpbfreader::OsmObj::Node(n));
        if let Some(ref n) = name {
            self.named_nodes.insert(n.clone(), id);
        }
        id
    }
}
