use snafu::Snafu;

use tests::{bano, cosmogony, download, ntfs, osm};

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Download Error: {}", source))]
    Download { source: download::Error },

    #[snafu(display("Generate Cosmogony Error: {}", source))]
    GenerateCosmogony { source: cosmogony::Error },

    #[snafu(display("Index Cosmogony Error: {}", source))]
    IndexCosmogony { source: cosmogony::Error },

    #[snafu(display("Index Bano Error: {}", source))]
    IndexBano { source: bano::Error },

    #[snafu(display("Index Osm Error: {}", source))]
    IndexOsm { source: osm::Error },

    #[snafu(display("Index NTFS Error: {}", source))]
    IndexNTFS { source: ntfs::Error },
}
