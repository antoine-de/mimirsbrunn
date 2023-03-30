// Copyright © 2023, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Hove (www.kisio.com).
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

use crate::{
    addr_reader::import_addresses_from_input_path,
    admin::fetch_admins,
    bano::Bano,
    settings::{admin_settings::AdminSettings, bano2mimir as settings},
    utils::template::update_templates,
};
use mimir::domain::ports::primary::generate_index::GenerateIndex;
use snafu::{ResultExt, Snafu};
use std::sync::Arc;

use mimir::{adapters::secondary::elasticsearch, domain::ports::secondary::remote::Remote};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    Settings { source: settings::Error },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir::domain::ports::secondary::remote::Error,
    },

    #[snafu(display("Execution Error {}", source))]
    Execution { source: Box<dyn std::error::Error> },

    #[snafu(display("Configuration Error {}", source))]
    Configuration { source: common::config::Error },

    #[snafu(display("Index Creation Error {}", source))]
    IndexCreation {
        source: mimir::domain::model::error::Error,
    },

    #[snafu(display("Admin Retrieval Error {}", details))]
    AdminRetrieval { details: String },
}

pub async fn run(
    opts: settings::Opts,
    settings: settings::Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch)
        .await
        .context(ElasticsearchConnectionSnafu)
        .map_err(Box::new)?;

    tracing::info!("Connected to elasticsearch.");

    // Update all the template components and indexes
    if settings.update_templates {
        update_templates(&client, opts.config_dir).await?;
    }

    // TODO There might be an opportunity for optimization here:
    // Lets say we're indexing a single bano department.... we don't need to retrieve
    // the admins for other regions!
    let into_addr = {
        let admin_settings = AdminSettings::build(&settings.admins);
        let admins = fetch_admins(&admin_settings, &client).await?;

        let admins_by_insee = admins
            .iter()
            .cloned()
            .filter(|a| !a.insee.is_empty())
            .map(|mut a| {
                a.boundary = None; // to save some space we remove the admin boundary
                (a.insee.clone(), Arc::new(a))
            })
            .collect();

        let admins_geofinder = admins.into_iter().collect();
        move |b: Bano| b.into_addr(&admins_by_insee, &admins_geofinder)
    };

    let addresses = import_addresses_from_input_path(&opts.input, false, into_addr);

    client
        .generate_index(&settings.container, futures::stream::iter(addresses))
        .await
        .context(IndexCreationSnafu)?;

    Ok(())
}
