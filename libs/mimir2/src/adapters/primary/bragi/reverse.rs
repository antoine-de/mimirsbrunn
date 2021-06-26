//        r#"{{
//            "query": {query}
//        }}"#,

pub fn build_reverse_query(distance: &str, lat: f64, lon: f64) -> String {
    format!(
        r#"{{
            "bool": {{
                "must": {{
                    "match_all": {{}}
                }},
                "filters": {{
                    "geo_distance": {{
                        "distance": {distance},
                        "...": {{
                            "lat": {lat},
                            "lon": {lon},
                        }}
                    }}
                }}
            }}
        }}"#,
        distance = distance,
        lat = lat,
        lon = lon
    )
}
