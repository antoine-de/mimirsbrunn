// Copyright © 2018, Canal TP and/or its affiliates. All rights reserved.
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

use cosmogony::{Zone, ZoneIndex};
use failure::{format_err, Error};
use futures::stream::{Stream, StreamExt};
use mimir2::{
    adapters::secondary::elasticsearch::{
        self,
        internal::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
        ElasticsearchStorage,
    },
    domain::{
        model::{configuration::Configuration, document::Document, index::IndexVisibility},
        ports::remote::Remote,
        usecases::{
            generate_index::{GenerateIndex, GenerateIndexParameters},
            UseCase,
        },
    },
};
use mimirsbrunn::osm_reader::admin;
use mimirsbrunn::osm_reader::osm_utils;
use mimirsbrunn::utils;
use places::admin::Admin;
use serde::Serialize;
use slog_scope::{info, warn};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use structopt::StructOpt;

trait IntoAdmin {
    fn into_admin(
        self,
        _: &BTreeMap<ZoneIndex, (String, Option<String>)>,
        langs: &[String],
        retrocompat_on_french_id: bool,
        max_weight: f64,
        all_admins: Option<&HashMap<String, Arc<Admin>>>,
    ) -> Admin;
}

// We use a new type to wrap around Addr and implement the Document trait.
#[derive(Serialize)]
struct AdminDoc(Admin);

impl Document for AdminDoc {
    const IS_GEO_DATA: bool = true;
    const DOC_TYPE: &'static str = "admin";
    fn id(&self) -> String {
        self.0.id.clone()
    }
}

async fn import_admins<S>(
    client: ElasticsearchStorage,
    config: IndexConfiguration,
    admins: S,
) -> Result<(), Error>
where
    S: Stream<Item = Admin> + Send + Sync + Unpin + 'static,
{
    let config = serde_json::to_string(&config).map_err(|err| {
        format_err!(
            "could not serialize index configuration: {}",
            err.to_string()
        )
    })?;
    let admins = admins.map(|admin| AdminDoc(admin));
    let generate_index = GenerateIndex::new(Box::new(client));
    let parameters = GenerateIndexParameters {
        config: Configuration { value: config },
        documents: Box::new(admins),
        visibility: IndexVisibility::Public,
    };
    generate_index
        .execute(parameters)
        .await
        .map_err(|err| format_err!("could not generate index: {}", err.to_string()))?;

    Ok(())
}

fn get_weight(tags: &osmpbfreader::Tags, center_tags: &osmpbfreader::Tags) -> f64 {
    // to have an admin weight we use the osm 'population' tag to priorize
    // the big zones over the small one.
    // Note: this tags is not often filled , so only some zones
    // will have a weight (but the main cities have it).
    tags.get("population")
        .and_then(|p| p.parse().ok())
        .or_else(|| center_tags.get("population")?.parse().ok())
        .unwrap_or(0.)
}

impl IntoAdmin for Zone {
    fn into_admin(
        self,
        zones_osm_id: &BTreeMap<ZoneIndex, (String, Option<String>)>,
        langs: &[String],
        french_id_retrocompatibility: bool,
        max_weight: f64,
        all_admins: Option<&HashMap<String, Arc<Admin>>>,
    ) -> Admin {
        let insee = admin::read_insee(&self.tags).map(|s| s.to_owned());
        let zip_codes = admin::read_zip_codes(&self.tags);
        let label = self.label;
        let weight = get_weight(&self.tags, &self.center_tags);
        let center = self.center.map_or(places::coord::Coord::default(), |c| {
            places::coord::Coord::new(c.lng(), c.lat())
        });
        let format_id = |id, insee| {
            // for retrocompatibity reasons, Navitia needs the
            // french admins to have an id with the insee for cities
            match insee {
                Some(insee) if french_id_retrocompatibility => format!("admin:fr:{}", insee),
                _ => format!("admin:osm:{}", id),
            }
        };
        let parent_osm_id = self
            .parent
            .and_then(|id| zones_osm_id.get(&id))
            .map(|(id, insee)| format_id(id, insee.as_ref()));
        let codes = osm_utils::get_osm_codes_from_tags(&self.tags);
        let mut admin = Admin {
            id: zones_osm_id
                .get(&self.id)
                .map(|(id, insee)| format_id(id, insee.as_ref()))
                .expect("unable to find zone id in zones_osm_id"),
            insee: insee.unwrap_or_else(|| "".to_owned()),
            level: self.admin_level.unwrap_or(0),
            label,
            name: self.name,
            zip_codes,
            weight: utils::normalize_weight(weight, max_weight),
            bbox: self.bbox,
            boundary: self.boundary,
            coord: center,
            approx_coord: Some(center.into()),
            zone_type: self.zone_type,
            parent_id: parent_osm_id,
            // Note: Since we do not really attach an admin to its hierarchy, for the moment an admin only have it's own coutry code,
            // not the country code of it's country from the hierarchy
            // (so it has a country code mainly if it is a country)
            country_codes: utils::get_country_code(&codes).into_iter().collect(),
            codes,
            names: osm_utils::get_names_from_tags(&self.tags, &langs),
            labels: self
                .international_labels
                .into_iter()
                .filter(|(k, _)| langs.contains(&k))
                .collect(),
            distance: None,
            context: None,
            administrative_regions: Vec::new(),
        };
        if let Some(ref admins) = all_admins {
            // Get a list of encompassing parent ids, which will be used as the get
            // administrative_regions.
            let mut parent_ids = Vec::new();
            let mut current = &admin;
            while current.parent_id.is_some() {
                parent_ids.push(current.parent_id.clone().unwrap());
                if let Some(par) = admins.get(parent_ids.last().unwrap()) {
                    current = par;
                } else {
                    break;
                }
            }
            admin.administrative_regions = parent_ids
                .into_iter()
                .filter_map(|a| admins.get(&a))
                .map(|x| Arc::clone(x))
                .collect::<Vec<_>>();
        }
        admin
    }
}

// Its difficult to get a stream from cosmogony. Also, since we use that function a few times,
// might as well save the result once and for all.
fn read_zones(input: &str) -> Result<Vec<Zone>, Error> {
    let zones = cosmogony::read_zones_from_file(input)?;
    let zones = zones
        .filter_map(|r| r.map_err(|e| warn!("impossible to read zone: {}", e)).ok())
        .collect::<Vec<_>>();
    Ok(zones)
}

async fn index_cosmogony(args: Args) -> Result<(), Error> {
    let dataset = args.dataset.clone();

    let pool = elasticsearch::remote::connection_pool_url(&args.connection_string)
        .await
        .map_err(|err| {
            format_err!(
                "could not create elasticsearch connection pool: {}",
                err.to_string()
            )
        })?;

    let client = pool
        .conn()
        .await
        .map_err(|err| format_err!("could not connect elasticsearch pool: {}", err.to_string()))?;

    let config = IndexConfiguration {
        name: dataset.clone(),
        parameters: IndexParameters {
            timeout: String::from("10s"),
            wait_for_active_shards: String::from("1"), // only the primary shard
        },
        settings: IndexSettings {
            value: String::from(include_str!("../../config/admin/settings.json")),
        },
        mappings: IndexMappings {
            value: String::from(include_str!("../../config/admin/mappings.json")),
        },
    };

    let zones = read_zones(&args.input)?;

    info!("building maps");
    use cosmogony::ZoneType::City;

    let mut cosmogony_id_to_osm_id = BTreeMap::new();
    let max_weight = utils::ADMIN_MAX_WEIGHT;
    zones.iter().for_each(|z| {
        let insee = match z.zone_type {
            Some(City) => admin::read_insee(&z.tags).map(|s| s.to_owned()),
            _ => None,
        };
        cosmogony_id_to_osm_id.insert(z.id, (z.osm_id.clone(), insee));
    });
    let cosmogony_id_to_osm_id = cosmogony_id_to_osm_id;

    info!("building admins hierarchy");
    let admins_without_boundaries = zones
        .iter()
        .map(|zone| {
            let mut zone = zone.clone();
            zone.boundary = None;
            let admin = zone.into_admin(
                &cosmogony_id_to_osm_id,
                &args.langs,
                args.french_id_retrocompatibility,
                max_weight,
                None,
            );
            (admin.id.clone(), Arc::new(admin))
        })
        .collect::<HashMap<_, _>>();

    info!("importing cosmogony into Mimir");

    let admins = zones
        .into_iter()
        .map(|zone| {
            zone.into_admin(
                &cosmogony_id_to_osm_id,
                &args.langs,
                args.french_id_retrocompatibility,
                max_weight,
                Some(&admins_without_boundaries),
            )
        })
        .collect::<Vec<_>>();

    import_admins(client, config, futures::stream::iter(admins)).await
}

#[derive(StructOpt, Debug)]
struct Args {
    /// cosmogony file
    #[structopt(short = "i", long = "input")]
    input: String,
    /// Elasticsearch parameters.
    #[structopt(
        short = "c",
        long = "connection-string",
        default_value = "http://localhost:9200/munin"
    )]
    connection_string: String,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    /// Number of shards for the es index
    #[structopt(short = "s", long = "nb-shards", default_value = "1")]
    nb_shards: usize,
    /// Number of replicas for the es index
    #[structopt(short = "r", long = "nb-replicas", default_value = "1")]
    nb_replicas: usize,
    /// Languages codes, used to build i18n names and labels
    #[structopt(name = "lang", short, long)]
    langs: Vec<String>,
    /// Retrocompatibiilty on french admin id
    /// if activated, the french administrative regions will have an id like 'admin:fr:{insee}'
    /// instead of 'admin:osm:{osm_id}'
    #[structopt(long = "french-id-retrocompatibility")]
    french_id_retrocompatibility: bool,
}

#[tokio::main]
async fn main() {
    mimirsbrunn::utils::launch_async(index_cosmogony).await;
}
