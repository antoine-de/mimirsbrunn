use super::coord::Coord;

/* How to restrict the range of the query... */
#[derive(Debug, Default)]
pub struct Filters {
    pub coord: Option<Coord>,
    pub shape: Option<(String, Vec<String>)>,
    pub datasets: Option<Vec<String>>,
    pub zone_types: Option<Vec<String>>,
    pub poi_types: Option<Vec<String>>,
}
