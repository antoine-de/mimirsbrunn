use geo;
use geo::prelude::BoundingBox;
use geojson;

pub fn geo_to_geojson_bbox(geo_bbox: geo::Bbox<f64>) -> geojson::Bbox {
    return vec![geo_bbox.xmin, geo_bbox.ymin, geo_bbox.xmax, geo_bbox.ymax];
}

pub fn mpoly_to_geojson_bbox(mpoly: &geo::MultiPolygon<f64>) -> Option<geojson::Bbox> {
    mpoly.bbox().map(geo_to_geojson_bbox)
}
